//! WiFi Parser - Pure Functions
//!
//! Parse raw beacon data into structured Network.
//! No state, no side effects, just data transformation.

mod ie;

use crate::types::*;
use crate::vendor::lookup_vendor;

pub use ie::parse_all_ies;

/// Parse a raw beacon into a Network struct.
pub fn parse_beacon(raw: &RawBeacon) -> Network {
    let bssid = raw.bssid_string();

    // Parse IE data
    let (standards, mut features, protocols, security, security_details, bss_load, country_code, wps, supported_rates) =
        ie::parse_capabilities(&raw.ie_data);

    // Detect channel width from IE
    let channel_width = ie::detect_channel_width(&raw.ie_data, raw.channel, raw.band);
    let wifi_generation = detect_wifi_generation(&standards);
    let min_data_rate = calculate_min_rate(&standards, channel_width, &supported_rates);
    let max_data_rate = calculate_max_rate(&standards, channel_width, &features, &supported_rates);
    features.max_data_rate = max_data_rate.round() as u32;

    let ssid = raw.ssid_string();
    let vendor = lookup_vendor(&bssid);
    let now = raw.timestamp;
    let is_hidden = ssid.is_none();

    Network {
        ssid,
        bssid,
        signal: raw.signal_dbm,
        noise: raw.noise_dbm,
        snr: raw.snr(),
        channel: raw.channel as u16,
        frequency: raw.frequency(),
        band: raw.band.to_string(),
        connected: raw.connected,
        standards,
        wifi_generation,
        channel_width,
        center_channel: None,
        secondary_channel: None,
        features,
        min_data_rate,
        max_data_rate,
        security,
        security_details,
        protocols,
        bss_load,
        is_hidden,
        network_group_id: None,
        vendor,
        country_code,
        supported_rates,
        wps_enabled: wps,
        ap_mode: 0,
        capabilities: 0,
        beacon_interval: raw.beacon_interval,
        first_seen: now,
        last_seen: now,
        seen_age_secs: 0,
        ap_uptime_secs: raw.uptime_ms.map(|uptime| uptime / 1000),
    }
}

fn detect_wifi_generation(standards: &[String]) -> u8 {
    if standards.iter().any(|standard| standard == "be") {
        7
    } else if standards.iter().any(|standard| standard == "ax") {
        6
    } else if standards.iter().any(|standard| standard == "ac") {
        5
    } else if standards.iter().any(|standard| standard == "n") {
        4
    } else {
        3
    }
}

fn calculate_min_rate(standards: &[String], channel_width: u16, supported_rates: &[u32]) -> f32 {
    if standards.iter().any(|standard| standard == "ac" || standard == "ax" || standard == "be") {
        return round_rate(ofdm_rate_mbps(channel_width, 0, 1));
    }

    if standards.iter().any(|standard| standard == "n") {
        return round_rate(ht_rate_mbps(channel_width, true, 0, 1));
    }

    supported_rates
        .iter()
        .copied()
        .min()
        .map(|rate| rate as f32)
        .unwrap_or(0.0)
}

fn calculate_max_rate(
    standards: &[String],
    channel_width: u16,
    features: &PerformanceFeatures,
    supported_rates: &[u32],
) -> f32 {
    let spatial_streams = features.spatial_streams.max(1) as u32;

    if standards.iter().any(|standard| standard == "ac" || standard == "ax" || standard == "be") {
        return round_rate(ofdm_rate_mbps(channel_width, 11, spatial_streams));
    }

    if standards.iter().any(|standard| standard == "n") {
        return round_rate(ht_rate_mbps(channel_width, features.guard_interval == 400, 7, spatial_streams));
    }

    supported_rates
        .iter()
        .copied()
        .max()
        .map(|rate| rate as f32)
        .unwrap_or(0.0)
}

fn ht_rate_mbps(channel_width: u16, short_gi: bool, mcs: usize, streams: u32) -> f32 {
    let table_20_long = [6.5, 13.0, 19.5, 26.0, 39.0, 52.0, 58.5, 65.0];
    let table_20_short = [7.2, 14.4, 21.7, 28.9, 43.3, 57.8, 65.0, 72.2];
    let table_40_long = [13.5, 27.0, 40.5, 54.0, 81.0, 108.0, 121.5, 135.0];
    let table_40_short = [15.0, 30.0, 45.0, 60.0, 90.0, 120.0, 135.0, 150.0];

    let per_stream = match (channel_width, short_gi) {
        (40, true) => table_40_short[mcs.min(7)],
        (40, false) => table_40_long[mcs.min(7)],
        (_, true) => table_20_short[mcs.min(7)],
        _ => table_20_long[mcs.min(7)],
    };

    per_stream * streams as f32
}

fn ofdm_rate_mbps(channel_width: u16, mcs: usize, streams: u32) -> f32 {
    let per_stream = match channel_width {
        20 => [8.6, 143.4],
        40 => [17.2, 286.8],
        80 => [36.0, 600.5],
        160 => [72.1, 1201.0],
        320 => [144.1, 2402.0],
        _ => [8.6, 143.4],
    };

    let selected = if mcs == 0 { per_stream[0] } else { per_stream[1] };
    selected * streams as f32
}

fn round_rate(value: f32) -> f32 {
    (value * 10.0).round() / 10.0
}
