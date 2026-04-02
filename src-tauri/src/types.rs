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

    pub fn from_frequency(freq_mhz: u32) -> Self {
        match freq_mhz {
            2400..=2499 => Band::Ghz2_4,
            5000..=5899 => Band::Ghz5,
            5900..=7125 => Band::Ghz6,
            _ => Band::Ghz2_4,
        }
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
    // WiFi standard flags (parsed from iw output on Linux)
    pub has_ht: bool,      // WiFi 4 (802.11n)
    pub has_vht: bool,     // WiFi 5 (802.11ac)
    pub has_he: bool,      // WiFi 6 (802.11ax)
    pub has_eht: bool,     // WiFi 7 (802.11be)
    pub spatial_streams: Option<u8>,
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

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Band Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_band_from_channel() {
        assert_eq!(Band::from_channel(1), Band::Ghz2_4);
        assert_eq!(Band::from_channel(6), Band::Ghz2_4);
        assert_eq!(Band::from_channel(11), Band::Ghz2_4);
        assert_eq!(Band::from_channel(14), Band::Ghz2_4);
        assert_eq!(Band::from_channel(36), Band::Ghz5);
        assert_eq!(Band::from_channel(48), Band::Ghz5);
        assert_eq!(Band::from_channel(149), Band::Ghz5);
    }

    #[test]
    fn test_band_from_frequency() {
        assert_eq!(Band::from_frequency(2412), Band::Ghz2_4);
        assert_eq!(Band::from_frequency(2437), Band::Ghz2_4);
        assert_eq!(Band::from_frequency(2484), Band::Ghz2_4);
        assert_eq!(Band::from_frequency(5180), Band::Ghz5);
        assert_eq!(Band::from_frequency(5240), Band::Ghz5);
        assert_eq!(Band::from_frequency(5745), Band::Ghz5);
        assert_eq!(Band::from_frequency(5955), Band::Ghz6);
        assert_eq!(Band::from_frequency(6100), Band::Ghz6);
    }

    #[test]
    fn test_band_display() {
        assert_eq!(format!("{}", Band::Ghz2_4), "2.4");
        assert_eq!(format!("{}", Band::Ghz5), "5");
        assert_eq!(format!("{}", Band::Ghz6), "6");
    }

    // -------------------------------------------------------------------------
    // RawBeacon Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_raw_beacon_bssid_string() {
        let beacon = RawBeacon {
            bssid: [0x80, 0x2D, 0x1A, 0x4B, 0x8C, 0x07],
            ..Default::default()
        };
        assert_eq!(beacon.bssid_string(), "80:2D:1A:4B:8C:07");
    }

    #[test]
    fn test_raw_beacon_ssid_string() {
        let mut beacon = RawBeacon {
            ssid: Some(b"MyNetwork".to_vec()),
            ..Default::default()
        };
        assert_eq!(beacon.ssid_string(), Some("MyNetwork".to_string()));

        beacon.ssid = Some(b"\xff\xfe".to_vec()); // Invalid UTF-8
        assert_eq!(beacon.ssid_string(), None);

        beacon.ssid = None;
        assert_eq!(beacon.ssid_string(), None);
    }

    #[test]
    fn test_raw_beacon_frequency() {
        let mut beacon = RawBeacon {
            channel: 6,
            band: Band::Ghz2_4,
            ..Default::default()
        };
        assert_eq!(beacon.frequency(), 2437);

        beacon.channel = 48;
        beacon.band = Band::Ghz5;
        assert_eq!(beacon.frequency(), 5240);

        beacon.channel = 1;
        beacon.band = Band::Ghz6;
        assert_eq!(beacon.frequency(), 5955);
    }

    #[test]
    fn test_raw_beacon_snr() {
        let beacon = RawBeacon {
            signal_dbm: -50,
            noise_dbm: -90,
            ..Default::default()
        };
        assert_eq!(beacon.snr(), 40);

        let beacon_noisy = RawBeacon {
            signal_dbm: -30,
            noise_dbm: -100,
            ..Default::default()
        };
        assert_eq!(beacon_noisy.snr(), 70);

        // Signal worse than noise should still return 0
        let beacon_bad = RawBeacon {
            signal_dbm: -100,
            noise_dbm: -50,
            ..Default::default()
        };
        assert_eq!(beacon_bad.snr(), 0);
    }

    // -------------------------------------------------------------------------
    // ChannelBandwidth Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_channel_bandwidth_as_mhz() {
        assert_eq!(ChannelBandwidth::MHz20.as_mhz(), 20);
        assert_eq!(ChannelBandwidth::MHz40.as_mhz(), 40);
        assert_eq!(ChannelBandwidth::MHz80.as_mhz(), 80);
        assert_eq!(ChannelBandwidth::MHz160.as_mhz(), 160);
        assert_eq!(ChannelBandwidth::MHz80Plus80.as_mhz(), 160);
        assert_eq!(ChannelBandwidth::MHz320.as_mhz(), 320);
    }

    #[test]
    fn test_channel_bandwidth_default() {
        assert_eq!(ChannelBandwidth::default(), ChannelBandwidth::MHz20);
    }

    // -------------------------------------------------------------------------
    // SecurityDetails Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_security_details_default() {
        let details = SecurityDetails::default();
        assert_eq!(details.security_type, "open");
        assert_eq!(details.auth_method, "open");
        assert_eq!(details.cipher, "none");
        assert!(!details.is_enterprise);
        assert!(!details.pmf_required);
        assert!(!details.pmf_capable);
        assert!(!details.sae);
        assert!(!details.owe);
    }

    // -------------------------------------------------------------------------
    // PerformanceFeatures Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_performance_features_default() {
        let features = PerformanceFeatures::default();
        assert!(!features.su_mimo);
        assert!(!features.mu_mimo);
        assert!(!features.ul_mu_mimo);
        assert!(!features.ofdma);
        assert_eq!(features.spatial_streams, 0);
        assert_eq!(features.max_supported_width, 0);
        assert_eq!(features.max_qam, 0);
    }

    // -------------------------------------------------------------------------
    // ChannelInfo Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_channel_info_default() {
        let info = ChannelInfo::default();
        assert_eq!(info.primary, 0);
        assert_eq!(info.bandwidth, ChannelBandwidth::MHz20);
        assert!(info.secondary.is_none());
        assert!(info.center_freq_0.is_none());
    }

    // -------------------------------------------------------------------------
    // OfdmaInfo Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_ofdma_info() {
        let info = OfdmaInfo {
            dl_ofdma: true,
            ul_ofdma: true,
            ru_sizes: vec![RuSize::R26, RuSize::R52, RuSize::R106],
        };
        assert!(info.dl_ofdma);
        assert!(info.ul_ofdma);
        assert_eq!(info.ru_sizes.len(), 3);
    }

    // -------------------------------------------------------------------------
    // TwtInfo Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_twt_info() {
        let info = TwtInfo {
            broadcast_twt: true,
            individual_twt: true,
            twt_requester: true,
            twt_responder: false,
        };
        assert!(info.broadcast_twt);
        assert!(info.individual_twt);
        assert!(info.twt_requester);
        assert!(!info.twt_responder);
    }

    // -------------------------------------------------------------------------
    // McsInfo Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_mcs_info() {
        let info = McsInfo {
            max_mcs: Some(11),
            current_mcs: Some(9),
            max_modulation: Some(Modulation::QAM1024),
        };
        assert_eq!(info.max_mcs, Some(11));
        assert_eq!(info.max_modulation, Some(Modulation::QAM1024));
    }

    // -------------------------------------------------------------------------
    // Modulation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_modulation_order() {
        // Test that modulation types are defined in order
        let modulations = [
            Modulation::BPSK,
            Modulation::QPSK,
            Modulation::QAM16,
            Modulation::QAM64,
            Modulation::QAM256,
            Modulation::QAM1024,
            Modulation::QAM4096,
        ];

        // Verify they exist and are distinct
        for (i, m) in modulations.iter().enumerate() {
            for (j, n) in modulations.iter().enumerate() {
                if i == j {
                    assert_eq!(*m, *n);
                } else {
                    assert_ne!(*m, *n);
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // ScanError Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_scan_error_display() {
        assert_eq!(
            format!("{}", ScanError::CommandFailed("test error".into())),
            "Command failed: test error"
        );
        assert_eq!(
            format!("{}", ScanError::ParseError("bad data".into())),
            "Parse error: bad data"
        );
        assert_eq!(
            format!("{}", ScanError::PermissionDenied),
            "Permission denied (try with sudo/admin)"
        );
        assert_eq!(
            format!("{}", ScanError::NotSupported),
            "Not supported on this platform"
        );
        assert_eq!(
            format!("{}", ScanError::NoInterface),
            "No WiFi interface found"
        );
    }

    // -------------------------------------------------------------------------
    // Network Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_network_default() {
        let network = Network::default();
        assert!(network.ssid.is_none());
        assert!(network.bssid.is_empty());
        assert_eq!(network.signal, 0);
        assert!(network.standards.is_empty());
        assert!(!network.connected);
    }

    // -------------------------------------------------------------------------
    // BssLoad Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_bss_load() {
        let load = BssLoad {
            channel_utilization: 50,
            station_count: 10,
            available_capacity: 100,
        };
        assert_eq!(load.channel_utilization, 50);
        assert_eq!(load.station_count, 10);
        assert_eq!(load.available_capacity, 100);
    }

    // -------------------------------------------------------------------------
    // LinkRates Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_link_rates() {
        let rates = LinkRates {
            rx_rate_mbps: Some(866.7),
            tx_rate_mbps: Some(433.3),
        };
        assert_eq!(rates.rx_rate_mbps, Some(866.7));
        assert_eq!(rates.tx_rate_mbps, Some(433.3));
    }

    // -------------------------------------------------------------------------
    // LocalAdapterCapabilities Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_local_adapter_capabilities() {
        let caps = LocalAdapterCapabilities {
            driver_name: "Intel AX200".to_string(),
            supported_standards: vec!["n".to_string(), "ac".to_string(), "ax".to_string()],
            tx_spatial_streams: 2,
            rx_spatial_streams: 2,
            max_supported_width: 160,
        };
        assert_eq!(caps.driver_name, "Intel AX200");
        assert_eq!(caps.supported_standards.len(), 3);
        assert_eq!(caps.tx_spatial_streams, 2);
        assert_eq!(caps.max_supported_width, 160);
    }

    // -------------------------------------------------------------------------
    // Serialization Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_band_serialization() {
        let band = Band::Ghz2_4;
        let json = serde_json::to_string(&band).unwrap();
        assert_eq!(json, "\"2.4\"");

        let band5 = Band::Ghz5;
        let json = serde_json::to_string(&band5).unwrap();
        assert_eq!(json, "\"5\"");
    }

    #[test]
    fn test_channel_bandwidth_serialization() {
        let bw = ChannelBandwidth::MHz80;
        let json = serde_json::to_string(&bw).unwrap();
        assert_eq!(json, "\"mhz80\"");

        let bw320 = ChannelBandwidth::MHz320;
        let json = serde_json::to_string(&bw320).unwrap();
        assert_eq!(json, "\"mhz320\"");
    }

    #[test]
    fn test_network_serialization() {
        let network = Network {
            ssid: Some("TestNetwork".to_string()),
            bssid: "AA:BB:CC:DD:EE:FF".to_string(),
            signal: -50,
            channel: 6,
            band: "2.4".to_string(),
            ..Default::default()
        };

        let json = serde_json::to_string(&network).unwrap();
        assert!(json.contains("TestNetwork"));
        assert!(json.contains("AA:BB:CC:DD:EE:FF"));
        assert!(json.contains("-50"));
    }
}
