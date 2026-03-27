//! UniFi - Core Type Definitions
//!
//! Data structures define the code. Keep them simple and clear.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Band Type
// ============================================================================

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Band {
    #[default]
    #[serde(rename = "2.4")]
    Ghz2_4,
    #[serde(rename = "5")]
    Ghz5,
    #[serde(rename = "6")]
    Ghz6,
}

impl Band {
    pub fn from_channel(ch: u8) -> Self {
        if ch > 14 { Band::Ghz5 } else { Band::Ghz2_4 }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            Band::Ghz2_4 => "2.4",
            Band::Ghz5 => "5",
            Band::Ghz6 => "6",
        }
    }
}

impl std::fmt::Display for Band {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// Raw Data - From Scanner
// ============================================================================

/// Raw beacon frame data from scanner.
/// This is the minimal common data all platforms can provide.
#[derive(Clone, Debug, Default)]
pub struct RawBeacon {
    pub ssid: Option<Vec<u8>>,
    pub bssid: [u8; 6],
    pub channel: u8,
    pub band: Band,
    pub signal_dbm: i16,
    pub noise_dbm: i16,
    pub ie_data: Vec<u8>,
    pub beacon_interval: u16,
    pub timestamp: u64,
    pub connected: bool,
}

impl RawBeacon {
    pub fn bssid_string(&self) -> String {
        format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.bssid[0], self.bssid[1], self.bssid[2],
            self.bssid[3], self.bssid[4], self.bssid[5]
        )
    }

    pub fn ssid_string(&self) -> Option<String> {
        self.ssid.as_ref().and_then(|s| String::from_utf8(s.clone()).ok())
    }
    
    pub fn frequency(&self) -> u32 {
        match self.band {
            Band::Ghz2_4 => 2407 + self.channel as u32 * 5,
            Band::Ghz5 => 5000 + self.channel as u32 * 5,
            Band::Ghz6 => 5950 + self.channel as u32 * 5,
        }
    }
    
    pub fn snr(&self) -> u16 {
        (self.signal_dbm - self.noise_dbm).max(0) as u16
    }
}

// ============================================================================
// Parsed IE
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParsedIE {
    pub element_id: u8,
    pub element_id_hex: String,
    pub name: String,
    pub length: u8,
    pub data_hex: String,
    pub parsed: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DetectionSummary {
    pub has_eht_capabilities: bool,
    pub has_eht_operation: bool,
    pub has_he_capabilities: bool,
    pub has_he_operation: bool,
    pub has_vht_capabilities: bool,
    pub has_vht_operation: bool,
    pub has_ht_capabilities: bool,
    pub has_ht_operation: bool,
    pub detected_standard: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IEDetails {
    pub raw_hex: String,
    pub total_length: usize,
    pub elements: Vec<ParsedIE>,
    pub detection_summary: DetectionSummary,
}

// ============================================================================
// Network - Final Output
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolExtensions {
    pub rrm: bool,
    pub bss_transition: bool,
    pub ft: bool,
    pub pmf: bool,
    pub wmm: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceFeatures {
    pub mu_mimo: bool,
    pub ofdma: bool,
    pub bss_coloring: bool,
    pub twt: bool,
    pub spatial_streams: u8,
    pub max_data_rate: u32,
    pub tx_beamforming: bool,
    pub ampdu_length: u8,
    pub mlo: bool,
    pub max_qam: u16,
    pub guard_interval: u16,  // in nanoseconds: 400, 800, 1600, 3200
    pub mcs_index: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BssLoad {
    pub channel_utilization: u8,
    pub station_count: u16,
    pub available_capacity: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityDetails {
    #[serde(rename = "type")]
    pub security_type: String,
    pub auth_method: String,
    pub cipher: String,
    pub key_mgmt: Vec<String>,
    pub is_enterprise: bool,
    pub is_wpa3_transition: bool,
    pub pmf_required: bool,
    pub pmf_capable: bool,
}

impl Default for SecurityDetails {
    fn default() -> Self {
        Self {
            security_type: "open".into(),
            auth_method: "open".into(),
            cipher: "none".into(),
            key_mgmt: vec![],
            is_enterprise: false,
            is_wpa3_transition: false,
            pmf_required: false,
            pmf_capable: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Network {
    pub ssid: Option<String>,
    pub bssid: String,
    pub signal: i16,
    pub noise: i16,
    pub snr: u16,
    pub channel: u16,
    pub frequency: u32,
    pub band: String,
    pub connected: bool,
    pub standards: Vec<String>,
    pub channel_width: u16,
    pub center_channel: Option<u16>,
    pub secondary_channel: Option<u16>,
    pub features: PerformanceFeatures,
    pub security: String,
    pub security_details: SecurityDetails,
    pub protocols: ProtocolExtensions,
    pub bss_load: Option<BssLoad>,
    pub is_hidden: bool,
    pub network_group_id: Option<String>,
    pub vendor: String,
    pub country_code: Option<String>,
    pub supported_rates: Vec<u32>,
    pub wps_enabled: bool,
    pub ap_mode: u16,
    pub capabilities: u16,
    pub beacon_interval: u16,
    pub first_seen: u64,
    pub last_seen: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkGroup {
    pub ssid: String,
    pub networks: Vec<Network>,
    pub total_aps: u32,
    pub bands: Vec<String>,
    pub best_signal: i16,
    pub supports_fast_roaming: bool,
    pub supports_bss_transition: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStats {
    pub total_networks: u32,
    pub hidden_networks: u32,
    pub network_groups: u32,
    pub by_band: HashMap<String, u32>,
    pub by_security: HashMap<String, u32>,
    pub by_standard: HashMap<String, u32>,
    pub scan_duration_ms: u64,
}

// ============================================================================
// Error
// ============================================================================

#[derive(Debug)]
pub enum ScanError {
    CommandFailed(String),
    ParseError(String),
    PermissionDenied,
    NotSupported,
    NoInterface,
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanError::CommandFailed(s) => write!(f, "Command failed: {}", s),
            ScanError::ParseError(s) => write!(f, "Parse error: {}", s),
            ScanError::PermissionDenied => write!(f, "Permission denied (try with sudo/admin)"),
            ScanError::NotSupported => write!(f, "Not supported on this platform"),
            ScanError::NoInterface => write!(f, "No WiFi interface found"),
        }
    }
}

impl std::error::Error for ScanError {}
