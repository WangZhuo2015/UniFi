//! IE (Information Element) Parser
//!
//! Pure functions for parsing 802.11 IE data.
//! No external dependencies on platform-specific code.

use crate::types::*;
use std::collections::HashMap;

// ============================================================================
// IE Names
// ============================================================================

pub fn ie_name(id: u8) -> &'static str {
    match id {
        0 => "SSID",
        1 => "Supported Rates",
        3 => "DS Parameter Set",
        7 => "Country",
        11 => "QBSS Load",
        45 => "HT Capabilities",
        48 => "RSN",
        50 => "Extended Supported Rates",
        61 => "HT Operation",
        70 => "RM Enabled Capabilities",
        127 => "Extended Capabilities",
        191 => "VHT Capabilities",
        192 => "VHT Operation",
        221 => "Vendor Specific",
        255 => "Extended Element",
        _ => "Unknown",
    }
}

fn ext_ie_name(ext_id: u8) -> &'static str {
    match ext_id {
        35 => "HE Capabilities",
        36 => "HE Operation",
        106 => "EHT Operation",
        107 => "EHT Multi-Link",
        108 => "EHT Capabilities",
        _ => "Unknown Extension",
    }
}

// ============================================================================
// Main Parsing Functions
// ============================================================================

/// Parse capabilities from IE data.
/// Returns (standards, features, protocols, security, security_details, bss_load, country_code, wps, rates)
pub fn parse_capabilities(ie_data: &[u8]) -> (
    Vec<String>,
    PerformanceFeatures,
    ProtocolExtensions,
    String,
    SecurityDetails,
    Option<BssLoad>,
    Option<String>,
    bool,
    Vec<u32>,
) {
    let mut standards = Vec::new();
    let mut features = PerformanceFeatures::default();
    let mut protocols = ProtocolExtensions::default();
    let mut security = "open".to_string();
    let mut security_details = SecurityDetails::default();
    let mut bss_load = None;
    let mut country_code = None;
    let mut wps = false;
    let mut supported_rates = Vec::new();
    
    // Scan IE data
    let mut pos = 0;
    while pos + 1 < ie_data.len() {
        let id = ie_data[pos];
        let len = ie_data[pos + 1] as usize;
        
        if pos + 2 + len > ie_data.len() {
            break;
        }
        
        let data = &ie_data[pos + 2..pos + 2 + len];
        
        match id {
            1 | 50 => {
                // Supported rates
                for b in data {
                    let rate = (b & 0x7F) as u32 / 2;
                    supported_rates.push(rate);
                }
            }
            11 => {
                // QBSS Load
                if len >= 5 {
                    bss_load = Some(BssLoad {
                        channel_utilization: data[2],
                        station_count: u16::from_le_bytes([data[0], data[1]]),
                        available_capacity: u16::from_le_bytes([data[3], data[4]]),
                    });
                }
            }
            45 => {
                // HT Capabilities (WiFi 4)
                if !standards.contains(&"n".to_string()) {
                    standards.push("n".to_string());
                }
                parse_ht_capabilities(data, &mut features);
            }
            48 => {
                // RSN (WPA2/WPA3)
                let (sec, details) = parse_rsn(data);
                security = sec;
                security_details = details;
            }
            70 => {
                // RM Capabilities (802.11k)
                protocols.rrm = true;
            }
            127 => {
                // Extended Capabilities
                parse_extended_capabilities(data, &mut protocols);
            }
            191 => {
                // VHT Capabilities (WiFi 5)
                if !standards.contains(&"ac".to_string()) {
                    standards.push("ac".to_string());
                }
                parse_vht_capabilities(data, &mut features);
            }
            221 => {
                // Vendor Specific
                if len >= 4 && data[0..3] == [0x00, 0x50, 0xF2] {
                    if data[3] == 0x04 {
                        wps = true;
                    }
                }
            }
            255 if len >= 1 => {
                // Extended Element
                match data[0] {
                    35 => {
                        // HE Capabilities (WiFi 6)
                        if !standards.contains(&"ax".to_string()) {
                            standards.push("ax".to_string());
                        }
                        parse_he_capabilities(data, &mut features);
                    }
                    108 => {
                        // EHT Capabilities (WiFi 7)
                        standards.clear();
                        standards.push("be".to_string());
                        parse_eht_capabilities(data, &mut features);
                    }
                    107 => {
                        // MLO
                        features.mlo = true;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        
        pos += 2 + len;
    }
    
    // Set defaults
    if standards.is_empty() {
        standards.push("g".to_string());
    }
    if features.max_qam == 0 {
        features.max_qam = if standards.contains(&"be".to_string()) { 4096 }
                          else if standards.contains(&"ax".to_string()) { 1024 }
                          else { 256 };
    }
    
    (standards, features, protocols, security, security_details, bss_load, country_code, wps, supported_rates)
}

/// Detect channel width from IE data.
pub fn detect_channel_width(ie_data: &[u8], channel: u8, band: Band) -> u16 {
    let mut pos = 0;
    let mut ht_40 = false;
    let mut vht_80 = false;
    let mut vht_160 = false;
    let mut eht_320 = false;
    
    while pos + 1 < ie_data.len() {
        let id = ie_data[pos];
        let len = ie_data[pos + 1] as usize;
        
        if pos + 2 + len > ie_data.len() {
            break;
        }
        
        let data = &ie_data[pos + 2..pos + 2 + len];
        
        match id {
            45 if len >= 1 => {
                // HT Capabilities - bit 1 = 40MHz support
                ht_40 = (data[0] & 0x02) != 0;
            }
            192 if len >= 1 => {
                // VHT Operation
                match data[0] {
                    1 => vht_80 = true,
                    2 | 3 => { vht_80 = true; vht_160 = true; }
                    _ => {}
                }
            }
            255 if len >= 4 && data[0] == 108 => {
                // EHT Operation - channel width in bits 2-4 of byte 1
                let width_bits = (data[1] >> 2) & 0x07;
                match width_bits {
                    4 => eht_320 = true,
                    3 => vht_160 = true,
                    2 => vht_80 = true,
                    _ => {}
                }
            }
            _ => {}
        }
        
        pos += 2 + len;
    }
    
    if eht_320 { 320 }
    else if vht_160 { 160 }
    else if vht_80 { 80 }
    else if ht_40 { 40 }
    else { 20 }
}

// ============================================================================
// Helper Parsers
// ============================================================================

fn parse_ht_capabilities(data: &[u8], features: &mut PerformanceFeatures) {
    if data.len() < 26 {
        return;
    }
    
    let caps = u16::from_le_bytes([data[0], data[1]]);
    
    // Count spatial streams from MCS set
    let mcs = &data[2..18];
    for i in 0..4 {
        if mcs[i] != 0 {
            features.spatial_streams = (i + 1) as u8;
        }
    }
    
    // TX beamforming
    features.tx_beamforming = (caps & 0x1000) != 0;
    
    // A-MPDU
    if data.len() >= 20 {
        let ampdu = data[18];
        features.ampdu_length = (ampdu & 0x03) + 1;
    }
}

fn parse_vht_capabilities(data: &[u8], features: &mut PerformanceFeatures) {
    if data.len() < 12 {
        return;
    }
    
    let caps = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    
    // MU-MIMO
    features.mu_mimo = (caps & 0x1000) != 0;
    
    // Max MPDU length indicates 256-QAM support
    features.max_qam = 256;
    
    // Spatial streams from MCS set
    if data.len() >= 8 {
        let rx_mcs = u16::from_le_bytes([data[4], data[5]]);
        let nss = (rx_mcs & 0x7) + 1;
        features.spatial_streams = nss as u8;
    }
}

fn parse_he_capabilities(data: &[u8], features: &mut PerformanceFeatures) {
    if data.len() < 7 {
        return;
    }
    
    let phy_cap = u32::from_le_bytes([data[3], data[4], data[5], data[6]]);
    
    // OFDMA
    features.ofdma = true;
    features.bss_coloring = true;
    features.max_qam = 1024;
    
    // 160MHz support
    if (phy_cap & 0x04) != 0 {
        // Supports 160MHz
    }
    
    // Default spatial streams for WiFi 6
    if features.spatial_streams == 0 {
        features.spatial_streams = 2;
    }
}

fn parse_eht_capabilities(data: &[u8], features: &mut PerformanceFeatures) {
    if data.len() < 9 {
        return;
    }
    
    let phy_cap = u32::from_le_bytes([data[5], data[6], data[7], data[8]]);
    
    features.ofdma = true;
    features.bss_coloring = true;
    features.mu_mimo = true;
    features.max_qam = 4096;
    
    // 320MHz support
    let _320mhz = phy_cap & 0x03;
    
    // 4096-QAM support
    if (phy_cap & 0x000F0000) != 0 {
        features.max_qam = 4096;
    }
    
    if features.spatial_streams == 0 {
        features.spatial_streams = 4;
    }
}

fn parse_extended_capabilities(data: &[u8], protocols: &mut ProtocolExtensions) {
    if data.len() > 1 {
        // Bit 12 = 802.11k RRM
        protocols.rrm = (data[1] & 0x10) != 0;
        // Bit 19 = 802.11v BSS Transition
        if data.len() > 2 {
            protocols.bss_transition = (data[2] & 0x08) != 0;
        }
    }
}

fn parse_rsn(data: &[u8]) -> (String, SecurityDetails) {
    if data.len() < 4 {
        return ("open".to_string(), SecurityDetails::default());
    }
    
    let group_cipher = u16::from_le_bytes([data[2], data[3]]);
    let cipher = match group_cipher {
        2 => "tkip",
        4 => "ccmp",
        8 => "gcmp",
        _ => "unknown",
    };
    
    if data.len() >= 6 {
        let auth_count = u16::from_le_bytes([data[4], data[5]]) as usize;
        let auth_offset = 6;
        
        if auth_offset + auth_count * 4 <= data.len() {
            let auth = u16::from_le_bytes([data[auth_offset], data[auth_offset + 1]]);
            
            let (sec_type, auth_method) = match auth {
                4 => ("wpa3", "sae"),
                1 => ("wpa2-ent", "eap"),
                _ => ("wpa2", "psk"),
            };
            
            return (sec_type.to_string(), SecurityDetails {
                security_type: sec_type.to_string(),
                auth_method: auth_method.to_string(),
                cipher: cipher.to_string(),
                key_mgmt: vec![auth_method.to_string()],
                is_enterprise: auth == 1,
                is_wpa3_transition: false,
                pmf_required: false,
                pmf_capable: auth == 4,
            });
        }
    }
    
    ("wpa2".to_string(), SecurityDetails {
        security_type: "wpa2".to_string(),
        auth_method: "psk".to_string(),
        cipher: cipher.to_string(),
        key_mgmt: vec!["psk".to_string()],
        is_enterprise: false,
        is_wpa3_transition: false,
        pmf_required: false,
        pmf_capable: false,
    })
}

// ============================================================================
// IE Details Parser (for UI display)
// ============================================================================

pub fn parse_all_ies(ie_data: &[u8]) -> IEDetails {
    let mut elements = Vec::new();
    let mut detection = DetectionSummary::default();
    
    let mut pos = 0;
    while pos + 1 < ie_data.len() {
        let id = ie_data[pos];
        let len = ie_data[pos + 1] as usize;
        
        if pos + 2 + len > ie_data.len() {
            break;
        }
        
        let data = &ie_data[pos + 2..pos + 2 + len];
        
        // Update detection
        match id {
            45 => detection.has_ht_capabilities = true,
            61 => detection.has_ht_operation = true,
            191 => detection.has_vht_capabilities = true,
            192 => detection.has_vht_operation = true,
            255 if len >= 1 => match data[0] {
                35 => detection.has_he_capabilities = true,
                36 => detection.has_he_operation = true,
                106 => detection.has_eht_operation = true,
                108 => detection.has_eht_capabilities = true,
                _ => {}
            },
            _ => {}
        }
        
        elements.push(ParsedIE {
            element_id: id,
            element_id_hex: format!("0x{:02x}", id),
            name: if id == 255 && len >= 1 {
                ext_ie_name(data[0]).to_string()
            } else {
                ie_name(id).to_string()
            },
            length: len as u8,
            data_hex: hex::encode(data),
            parsed: parse_ie_content(id, data),
        });
        
        pos += 2 + len;
    }
    
    // Determine standard
    detection.detected_standard = if detection.has_eht_capabilities {
        "WiFi 7 (802.11be)".to_string()
    } else if detection.has_he_capabilities {
        "WiFi 6 (802.11ax)".to_string()
    } else if detection.has_vht_capabilities {
        "WiFi 5 (802.11ac)".to_string()
    } else if detection.has_ht_capabilities {
        "WiFi 4 (802.11n)".to_string()
    } else {
        "Legacy".to_string()
    };
    
    IEDetails {
        raw_hex: hex::encode(ie_data),
        total_length: ie_data.len(),
        elements,
        detection_summary: detection,
    }
}

pub fn parse_ie_content(id: u8, data: &[u8]) -> HashMap<String, serde_json::Value> {
    let mut result = HashMap::new();
    
    match id {
        0 => {
            if let Ok(ssid) = std::str::from_utf8(data) {
                result.insert("ssid".into(), serde_json::Value::String(ssid.into()));
            }
        }
        1 | 50 => {
            let rates: Vec<String> = data.iter().map(|b| {
                let r = (b & 0x7F) as f32 * 0.5;
                format!("{} Mbps", r)
            }).collect();
            result.insert("rates".into(), serde_json::Value::String(rates.join(", ")));
        }
        3 => {
            if !data.is_empty() {
                result.insert("channel".into(), serde_json::Value::Number(data[0].into()));
            }
        }
        11 => {
            if data.len() >= 5 {
                result.insert("stationCount".into(), serde_json::Value::Number(
                    u16::from_le_bytes([data[0], data[1]]).into()
                ));
                result.insert("channelUtilization".into(), serde_json::Value::Number(data[2].into()));
            }
        }
        255 if data.len() >= 1 => {
            result.insert("extensionId".into(), serde_json::Value::Number(data[0].into()));
            result.insert("extensionName".into(), serde_json::Value::String(ext_ie_name(data[0]).into()));
        }
        _ => {}
    }
    
    result
}
