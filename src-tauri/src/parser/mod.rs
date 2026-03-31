//! WiFi Parser - Pure Functions
//!
//! Parse raw beacon data into structured Network.
//! No state, no side effects, just data transformation.

mod ie;

use crate::types::*;
use crate::vendor::{lookup_vendor, lookup_vendor_from_ie};

pub use ie::parse_all_ies;

/// Parse a raw beacon into a Network struct.
pub fn parse_beacon(raw: &RawBeacon) -> Network {
    let bssid = raw.bssid_string();

    // Parse IE data - single pass, structured result
    let parsed = ie::parse_capabilities(&raw.ie_data);
    let standards = normalize_standards_for_band(parsed.standards, raw.band);
    let mut features = parsed.features;
    let channel_width = parsed.channel_width;
    let wifi_generation = detect_wifi_generation(&standards);
    features.max_supported_width = normalize_width_for_band(features.max_supported_width, raw.band);

    // Populate primary channel in channel_info from raw data
    if let Some(ref mut ch_info) = features.channel_info {
        ch_info.primary = raw.channel;
        ch_info.frequency = Some(raw.frequency());
    }

    // For WiFi 7 (EHT), also populate OFDMA info
    if standards.contains(&"be".to_string()) && features.ofdma_info.is_none() {
        features.ofdma_info = Some(OfdmaInfo {
            dl_ofdma: true,
            ul_ofdma: true,
            ru_sizes: vec![RuSize::R26, RuSize::R52, RuSize::R106, RuSize::R242, RuSize::R484, RuSize::R996, RuSize::R996x2],
        });
    }

    let min_data_rate = calculate_min_rate(&standards, channel_width, &parsed.supported_rates);
    let max_data_rate = calculate_max_rate(&standards, channel_width, &features, &parsed.supported_rates);
    let ap_peak_data_rate = calculate_max_rate(
        &standards,
        normalize_width_for_band(features.max_supported_width.max(channel_width), raw.band),
        &features,
        &parsed.supported_rates,
    );
    let client_spatial_streams = raw.local_adapter.as_ref().map(|adapter| {
        adapter
            .tx_spatial_streams
            .min(adapter.rx_spatial_streams)
            .max(1)
            .min(features.spatial_streams.max(1))
    });
    let client_peak_data_rate = raw
        .local_adapter
        .as_ref()
        .and_then(|adapter| calculate_client_peak_data_rate(&standards, channel_width, &features, adapter, &parsed.supported_rates, raw.band));
    features.max_data_rate = max_data_rate.round() as u32;

    // Get SSID from raw beacon or parse from IE data
    let ssid = raw.ssid_string().or_else(|| parse_ssid_from_ie(&raw.ie_data));
    let mut vendor = lookup_vendor(&bssid);
    if vendor == "Unknown" || vendor == "Locally Administered" {
        if let Some(ie_vendor) = lookup_vendor_from_ie(&raw.ie_data) {
            vendor = ie_vendor;
        }
    }
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
        ap_peak_data_rate,
        security: parsed.security,
        security_details: parsed.security_details,
        protocols: parsed.protocols,
        bss_load: parsed.bss_load,
        is_hidden,
        network_group_id: None,
        vendor,
        country_code: parsed.country_code,
        supported_rates: parsed.supported_rates,
        wps_enabled: parsed.wps,
        ap_mode: 0,
        capabilities: 0,
        beacon_interval: raw.beacon_interval,
        first_seen: now,
        last_seen: now,
        seen_age_secs: 0,
        ap_uptime_secs: raw.uptime_ms.map(|uptime| uptime / 1000),
        link_rates: raw.link_rates.clone(),
        local_adapter: raw.local_adapter.clone(),
        client_peak_data_rate,
        client_spatial_streams,
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

fn calculate_client_peak_data_rate(
    ap_standards: &[String],
    channel_width: u16,
    features: &PerformanceFeatures,
    adapter: &LocalAdapterCapabilities,
    supported_rates: &[u32],
    band: Band,
) -> Option<f32> {
    let effective_streams = adapter
        .tx_spatial_streams
        .min(adapter.rx_spatial_streams)
        .max(1)
        .min(features.spatial_streams.max(1));
    let effective_width = normalize_width_for_band(
        adapter
            .max_supported_width
            .max(channel_width)
            .min(features.max_supported_width.max(channel_width)),
        band,
    );
    let common_standards = intersect_standards(ap_standards, &adapter.supported_standards);

    if common_standards.is_empty() {
        return None;
    }

    let mut local_features = features.clone();
    local_features.spatial_streams = effective_streams;

    Some(calculate_max_rate(
        &common_standards,
        effective_width,
        &local_features,
        supported_rates,
    ))
}

fn intersect_standards(ap_standards: &[String], adapter_standards: &[String]) -> Vec<String> {
    let mut common = Vec::new();

    for standard in ["be", "ax", "ac", "n", "g", "a", "b"] {
        if ap_standards.iter().any(|value| value == standard)
            && adapter_standards.iter().any(|value| value == standard)
        {
            common.push(standard.to_string());
        }
    }

    common
}

fn normalize_standards_for_band(mut standards: Vec<String>, band: Band) -> Vec<String> {
    standards.retain(|standard| match band {
        Band::Ghz2_4 => matches!(standard.as_str(), "b" | "g" | "n" | "ax" | "be"),
        Band::Ghz5 => matches!(standard.as_str(), "a" | "n" | "ac" | "ax" | "be"),
        Band::Ghz6 => matches!(standard.as_str(), "ax" | "be"),
    });

    if standards.is_empty() {
        standards.push(match band {
            Band::Ghz2_4 => "g".to_string(),
            Band::Ghz5 => "a".to_string(),
            Band::Ghz6 => "ax".to_string(),
        });
    }

    standards
}

fn normalize_width_for_band(width: u16, band: Band) -> u16 {
    let capped = match band {
        Band::Ghz2_4 => width.min(40),
        Band::Ghz5 => width.min(160),
        Band::Ghz6 => width.min(320),
    };

    if capped == 0 { 20 } else { capped }
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

/// Parse SSID from IE data (IE ID 0)
fn parse_ssid_from_ie(ie_data: &[u8]) -> Option<String> {
    let mut pos = 0;
    while pos + 1 < ie_data.len() {
        let id = ie_data[pos];
        let len = ie_data[pos + 1] as usize;

        if pos + 2 + len > ie_data.len() {
            break;
        }

        if id == 0 && len > 0 {
            let ssid_bytes = &ie_data[pos + 2..pos + 2 + len];
            return String::from_utf8(ssid_bytes.to_vec()).ok();
        }

        pos += 2 + len;
    }
    None
}
