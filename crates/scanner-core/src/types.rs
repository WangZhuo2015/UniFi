//! Raw beacon data types extracted from WiFi scans

use serde::{Deserialize, Serialize};

/// Raw beacon data from a WiFi network
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawBeacon {
    /// Network SSID (may be hidden/empty)
    pub ssid: Option<String>,
    
    /// BSSID (MAC address of the AP)
    pub bssid: Option<String>,
    
    /// Signal strength in dBm (usually negative)
    pub signal: Option<i32>,
    
    /// Channel number
    pub channel: Option<u32>,
    
    /// Frequency in MHz
    pub frequency: Option<u32>,
    
    /// WiFi standard (4, 5, 6, 7)
    pub wifi_standard: Option<u8>,
    
    /// Maximum data rate in Mbps
    pub max_rate: Option<f32>,
    
    /// Security type
    pub security: Option<SecurityType>,
    
    /// Raw IE (Information Elements) data
    pub ie_data: Option<Vec<u8>>,
    
    /// Vendor-specific IEs
    pub vendor_ies: Vec<VendorIE>,
    
    /// Supported MCS indices for 802.11n/ac/ax
    pub mcs_indices: Vec<u8>,
    
    /// Channel width in MHz (20, 40, 80, 160, 320)
    pub channel_width: Option<u32>,
    
    /// Guard interval in nanoseconds (400, 800, 1600, 3200)
    pub guard_interval: Option<u32>,
    
    /// Is the network secured
    pub is_secured: bool,
    
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

impl RawBeacon {
    /// Create a new empty beacon
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Check if SSID is hidden
    pub fn is_hidden(&self) -> bool {
        self.ssid.is_none() || self.ssid.as_ref().map(|s| s.is_empty()).unwrap_or(false)
    }
    
    /// Get band (2.4GHz, 5GHz, or 6GHz)
    pub fn band(&self) -> Option<Band> {
        match self.frequency {
            Some(f) if (2400..2500).contains(&f) => Some(Band::Band24),
            Some(f) if (5000..6000).contains(&f) => Some(Band::Band5),
            Some(f) if (6000..7125).contains(&f) => Some(Band::Band6),
            _ => None,
        }
    }
    
    /// Calculate signal quality percentage (0-100)
    pub fn signal_quality(&self) -> Option<u8> {
        self.signal.map(|s| {
            // Convert dBm to percentage
            // -30 dBm = 100%, -90 dBm = 0%
            let quality = ((s + 90) as f32 / 60.0 * 100.0).clamp(0.0, 100.0);
            quality as u8
        })
    }
}

/// WiFi frequency band
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Band {
    /// 2.4 GHz band (2401-2495 MHz)
    Band24,
    /// 5 GHz band (5150-5895 MHz)
    Band5,
    /// 6 GHz band (5925-7125 MHz)
    Band6,
}

impl std::fmt::Display for Band {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Band::Band24 => write!(f, "2.4 GHz"),
            Band::Band5 => write!(f, "5 GHz"),
            Band::Band6 => write!(f, "6 GHz"),
        }
    }
}

/// Security type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityType {
    /// Open network (no security)
    Open,
    /// WEP (deprecated)
    WEP,
    /// WPA Personal (TKIP)
    WPA,
    /// WPA2 Personal (AES)
    WPA2,
    /// WPA2/WPA3 Transition mode
    WPA2WPA3,
    /// WPA3 Personal
    WPA3,
    /// WPA Enterprise (802.1X)
    WPAEnterprise,
    /// WPA2 Enterprise
    WPA2Enterprise,
    /// WPA3 Enterprise
    WPA3Enterprise,
}

impl std::fmt::Display for SecurityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityType::Open => write!(f, "Open"),
            SecurityType::WEP => write!(f, "WEP"),
            SecurityType::WPA => write!(f, "WPA"),
            SecurityType::WPA2 => write!(f, "WPA2"),
            SecurityType::WPA2WPA3 => write!(f, "WPA2/WPA3"),
            SecurityType::WPA3 => write!(f, "WPA3"),
            SecurityType::WPAEnterprise => write!(f, "WPA-Enterprise"),
            SecurityType::WPA2Enterprise => write!(f, "WPA2-Enterprise"),
            SecurityType::WPA3Enterprise => write!(f, "WPA3-Enterprise"),
        }
    }
}

/// Vendor-specific Information Element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorIE {
    /// OUI (Organizationally Unique Identifier)
    pub oui: [u8; 3],
    
    /// Vendor-specific type
    pub vendor_type: u8,
    
    /// Vendor data
    pub data: Vec<u8>,
}

impl VendorIE {
    /// Check if this is an Apple OUI
    pub fn is_apple(&self) -> bool {
        // Apple's primary OUIs
        matches!(self.oui, [0x00, 0x17, 0xf2] | [0x00, 0x03, 0x93] |
                       [0x00, 0x05, 0xca] | [0x00, 0x0a, 0x27] |
                       [0x00, 0x0d, 0x93] | [0x00, 0x11, 0x24] |
                       [0x00, 0x14, 0x51] | [0x00, 0x16, 0xcb] |
                       [0x00, 0x19, 0xe3] | [0x00, 0x1b, 0x63] |
                       [0xdc, 0xa9, 0x04] | [0xdc, 0xa6, 0x32])
    }
    
    /// Check if this is a Broadcom OUI
    pub fn is_broadcom(&self) -> bool {
        matches!(self.oui, [0x00, 0x10, 0x18] | [0x00, 0x0c, 0xe5] |
                       [0x00, 0x0d, 0xbd] | [0x00, 0x11, 0x93] |
                       [0x00, 0x12, 0x17] | [0x00, 0x13, 0x37])
    }
    
    /// Check if this is a Qualcomm/Atheros OUI
    pub fn is_qualcomm(&self) -> bool {
        matches!(self.oui, [0x00, 0x03, 0x7f] | [0x00, 0x0c, 0xe5] |
                       [0x00, 0x12, 0xbf] | [0x00, 0x13, 0xe8] |
                       [0x00, 0x14, 0x6c] | [0x00, 0x15, 0xe9])
    }
    
    /// Get vendor name from OUI
    pub fn vendor_name(&self) -> &'static str {
        if self.is_apple() { "Apple" }
        else if self.is_broadcom() { "Broadcom" }
        else if self.is_qualcomm() { "Qualcomm" }
        else { "Unknown" }
    }
}

/// WiFi standard
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WiFiStandard {
    /// 802.11a (5 GHz, up to 54 Mbps)
    A,
    /// 802.11b (2.4 GHz, up to 11 Mbps)
    B,
    /// 802.11g (2.4 GHz, up to 54 Mbps)
    G,
    /// 802.11n (WiFi 4, 2.4/5 GHz, up to 600 Mbps)
    N,
    /// 802.11ac (WiFi 5, 5 GHz, up to 6.9 Gbps)
    AC,
    /// 802.11ax (WiFi 6/6E, 2.4/5/6 GHz)
    AX,
    /// 802.11be (WiFi 7, 2.4/5/6 GHz)
    BE,
}

impl std::fmt::Display for WiFiStandard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WiFiStandard::A => write!(f, "802.11a"),
            WiFiStandard::B => write!(f, "802.11b"),
            WiFiStandard::G => write!(f, "802.11g"),
            WiFiStandard::N => write!(f, "WiFi 4 (802.11n)"),
            WiFiStandard::AC => write!(f, "WiFi 5 (802.11ac)"),
            WiFiStandard::AX => write!(f, "WiFi 6 (802.11ax)"),
            WiFiStandard::BE => write!(f, "WiFi 7 (802.11be)"),
        }
    }
}

impl WiFiStandard {
    /// Get the WiFi generation number
    pub fn generation(&self) -> u8 {
        match self {
            WiFiStandard::A => 0,
            WiFiStandard::B => 0,
            WiFiStandard::G => 0,
            WiFiStandard::N => 4,
            WiFiStandard::AC => 5,
            WiFiStandard::AX => 6,
            WiFiStandard::BE => 7,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_beacon_band() {
        let mut beacon = RawBeacon::new();
        
        beacon.frequency = Some(2412);
        assert_eq!(beacon.band(), Some(Band::Band24));
        
        beacon.frequency = Some(5180);
        assert_eq!(beacon.band(), Some(Band::Band5));
        
        beacon.frequency = Some(6100);
        assert_eq!(beacon.band(), Some(Band::Band6));
    }
    
    #[test]
    fn test_signal_quality() {
        let mut beacon = RawBeacon::new();
        
        beacon.signal = Some(-30);
        assert_eq!(beacon.signal_quality(), Some(100));
        
        beacon.signal = Some(-60);
        assert_eq!(beacon.signal_quality(), Some(50));
        
        beacon.signal = Some(-90);
        assert_eq!(beacon.signal_quality(), Some(0));
        
        beacon.signal = Some(-100);
        assert_eq!(beacon.signal_quality(), Some(0));
    }
    
    #[test]
    fn test_hidden_ssid() {
        let mut beacon = RawBeacon::new();
        assert!(beacon.is_hidden());
        
        beacon.ssid = Some("".to_string());
        assert!(beacon.is_hidden());
        
        beacon.ssid = Some("MyNetwork".to_string());
        assert!(!beacon.is_hidden());
    }
    
    #[test]
    fn test_wifi_standard_generation() {
        assert_eq!(WiFiStandard::N.generation(), 4);
        assert_eq!(WiFiStandard::AC.generation(), 5);
        assert_eq!(WiFiStandard::AX.generation(), 6);
        assert_eq!(WiFiStandard::BE.generation(), 7);
    }
}
