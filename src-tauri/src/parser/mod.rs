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
    let (standards, features, protocols, security, security_details, bss_load, country_code, wps, supported_rates) =
        ie::parse_capabilities(&raw.ie_data);

    // Detect channel width from IE
    let channel_width = ie::detect_channel_width(&raw.ie_data, raw.channel, raw.band);

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
        channel_width,
        center_channel: None,
        secondary_channel: None,
        features,
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
    }
}
