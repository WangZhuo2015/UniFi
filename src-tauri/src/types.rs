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
    pub uptime_ms: Option<u64>,
    pub connected: bool,
    pub link_rates: Option<LinkRates>,
    pub local_adapter: Option<LocalAdapterCapabilities>,
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
    pub summary: String,
    pub vendor_name: Option<String>,
    pub display_fields: Vec<ParsedField>,
    pub parsed: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ParsedField {
    pub label: String,
    pub value: String,
    pub highlighted: bool,
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
// Channel Information (Detailed)
// ============================================================================

/// Channel bandwidth modes
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelBandwidth {
    #[default]
    MHz20,
    MHz40,
    MHz80,
    MHz160,
    MHz80Plus80,  // 80+80 non-contiguous (WiFi 5)
    MHz320,       // WiFi 7
}

impl ChannelBandwidth {
    pub fn as_mhz(&self) -> u16 {
        match self {
            ChannelBandwidth::MHz20 => 20,
            ChannelBandwidth::MHz40 => 40,
            ChannelBandwidth::MHz80 => 80,
            ChannelBandwidth::MHz160 => 160,
            ChannelBandwidth::MHz80Plus80 => 160,  // Total 160MHz
            ChannelBandwidth::MHz320 => 320,
        }
    }
}

/// Secondary channel position
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecondaryChannelOffset {
    Above,  // Secondary channel above primary (higher frequency)
    Below,  // Secondary channel below primary (lower frequency)
}

/// Detailed channel information
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelInfo {
    // Basic info - all scanners can provide
    pub primary: u8,
    pub bandwidth: ChannelBandwidth,

    // Extended info - from IE parsing
    pub secondary: Option<u8>,
    pub secondary_offset: Option<SecondaryChannelOffset>,
    pub center_freq_0: Option<u16>,  // Center frequency segment 0 (MHz)
    pub center_freq_1: Option<u16>,  // Center frequency segment 1 (for 80+80)
    pub frequency: Option<u32>,
}

// ============================================================================
// Spatial Stream Information (Detailed)
// ============================================================================

/// Spatial stream information for MIMO
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialStreamInfo {
    // AP capabilities - from IE
    pub tx_streams: Option<u8>,  // AP transmit streams
    pub rx_streams: Option<u8>,  // AP receive streams

    // Client capabilities - from local adapter
    pub client_tx: Option<u8>,
    pub client_rx: Option<u8>,

    // Effective streams for this connection
    pub effective_streams: Option<u8>,  // min(AP, Client)
}

// ============================================================================
// OFDMA Information (WiFi 6+)
// ============================================================================

/// Resource Unit sizes for OFDMA
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuSize {
    R26,
    R52,
    R106,
    R242,
    R484,
    R996,
    R996x2,
}

/// OFDMA capabilities (WiFi 6+)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfdmaInfo {
    pub dl_ofdma: bool,        // Downlink OFDMA (AP to multiple clients)
    pub ul_ofdma: bool,        // Uplink OFDMA (multiple clients to AP)
    pub ru_sizes: Vec<RuSize>, // Supported RU sizes
}

// ============================================================================
// TWT - Target Wake Time (WiFi 6+ Power Save)
// ============================================================================

/// TWT power saving capabilities
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwtInfo {
    pub broadcast_twt: bool,    // Broadcast TWT support
    pub individual_twt: bool,   // Individual TWT support
    pub twt_requester: bool,    // Can request TWT
    pub twt_responder: bool,    // Can respond to TWT requests
}

// ============================================================================
// WiFi 7 Specific Features
// ============================================================================

/// Multi-Link Operation link info
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MloLink {
    pub band: String,
    pub channel: u16,
    pub frequency: u32,
}

/// Multi-Link Operation (WiFi 7)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MloInfo {
    pub enabled: bool,
    pub num_links: u8,
    pub links: Vec<MloLink>,
}

/// WiFi 7 enhanced features
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wifi7Features {
    pub mlo: Option<MloInfo>,
    pub punctured_preamble: bool,  // Punctured preamble for interference handling
    pub multi_ru: bool,            // Multiple RU support
}

// ============================================================================
// MCS & Modulation Details
// ============================================================================

/// Modulation type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Modulation {
    BPSK,
    QPSK,
    QAM16,
    QAM64,
    QAM256,
    QAM1024,
    QAM4096,
}

/// MCS and modulation details
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McsInfo {
    pub max_mcs: Option<u8>,
    pub current_mcs: Option<u8>,
    pub max_modulation: Option<Modulation>,
}

// ============================================================================
// Security Details (Enhanced)
// ============================================================================

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

    // Enhanced security info
    pub group_cipher: Option<String>,     // Group (multicast) cipher
    pub pairwise_ciphers: Vec<String>,    // Supported pairwise ciphers
    pub sae: bool,                        // SAE (WPA3 authentication)
    pub owe: bool,                        // OWE (Open Wireless Encryption)
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
            group_cipher: None,
            pairwise_ciphers: vec![],
            sae: false,
            owe: false,
        }
    }
}

// ============================================================================
// Roaming & Protocols (Enhanced)
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RoamingInfo {
    // 802.11k - Radio Resource Measurement
    pub rrm: bool,
    pub neighbor_report: bool,  // Neighbor Report capability
    pub beacon_report: bool,    // Beacon Report capability

    // 802.11r - Fast BSS Transition
    pub ft: bool,
    pub ft_over_ds: bool,       // FT over DS (Distribution System)
    pub ft_resource_request: bool,

    // 802.11v - BSS Transition Management
    pub bss_transition: bool,
    pub wnm_sleep: bool,        // WNM Sleep Mode

    // 802.11w - Protected Management Frames
    pub pmf: bool,

    // WMM/QoS
    pub wmm: bool,
    pub wmm_uapsd: bool,        // WMM Unscheduled APSD
}

// Legacy alias for backward compatibility
pub type ProtocolExtensions = RoamingInfo;

// ============================================================================
// Network - Final Output
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceFeatures {
    // MIMO capabilities
    pub su_mimo: bool,
    pub mu_mimo: bool,
    pub ul_mu_mimo: bool,

    // Beamforming capabilities
    pub su_beamformer: bool,
    pub su_beamformee: bool,
    pub mu_beamformer: bool,

    // Detailed MIMO info
    pub spatial_streams: u8,
    pub spatial_stream_info: Option<SpatialStreamInfo>,

    // OFDMA
    pub ofdma: bool,
    pub ofdma_info: Option<OfdmaInfo>,

    // TWT Power Save
    pub twt: bool,
    pub twt_info: Option<TwtInfo>,

    // Channel/Bandwidth
    pub max_supported_width: u16,
    pub channel_info: Option<ChannelInfo>,

    // MCS/Modulation
    pub max_qam: u16,
    pub mcs_info: Option<McsInfo>,

    // Other
    pub bss_coloring: bool,
    pub ampdu_length: u8,
    pub guard_interval: u16,
    pub mcs_index: Option<u8>,

    // WiFi 7 specific
    pub mlo: bool,
    pub wifi7_features: Option<Wifi7Features>,

    // Legacy fields for backward compatibility
    pub max_data_rate: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BssLoad {
    pub channel_utilization: u8,
    pub station_count: u16,
    pub available_capacity: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LinkRates {
    pub rx_rate_mbps: Option<f32>,
    pub tx_rate_mbps: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LocalAdapterCapabilities {
    pub driver_name: String,
    pub supported_standards: Vec<String>,
    pub tx_spatial_streams: u8,
    pub rx_spatial_streams: u8,
    pub max_supported_width: u16,
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
    pub wifi_generation: u8,
    pub channel_width: u16,
    pub center_channel: Option<u16>,
    pub secondary_channel: Option<u16>,
    pub features: PerformanceFeatures,
    pub min_data_rate: f32,
    pub max_data_rate: f32,
    pub ap_peak_data_rate: f32,
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
    pub seen_age_secs: u64,
    pub ap_uptime_secs: Option<u64>,
    pub link_rates: Option<LinkRates>,
    pub local_adapter: Option<LocalAdapterCapabilities>,
    pub client_peak_data_rate: Option<f32>,
    pub client_spatial_streams: Option<u8>,
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
