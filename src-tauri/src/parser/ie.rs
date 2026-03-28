//! IE (Information Element) Parser
//!
//! Pure functions for parsing 802.11 IE data.
//! No external dependencies on platform-specific code.

use crate::types::*;
use crate::vendor::lookup_oui;
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
            7 => {
                if len >= 2 {
                    let code = String::from_utf8_lossy(&data[0..2]).trim().to_string();
                    if !code.is_empty() {
                        country_code = Some(code);
                    }
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
pub fn detect_channel_width(ie_data: &[u8], _channel: u8, _band: Band) -> u16 {
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

    // HT MCS set starts after HT Cap Info (2 bytes) and A-MPDU params (1 byte)
    let mcs = &data[3..19];
    for i in 0..4 {
        if mcs[i] != 0 {
            features.spatial_streams = (i + 1) as u8;
        }
    }

    // TX beamforming
    features.tx_beamforming = (caps & 0x1000) != 0;

    // Guard Interval: bit 6 = Short GI for 20MHz, bit 7 = Short GI for 40MHz
    // Short GI = 400ns, Long GI = 800ns
    let short_gi_20 = (caps & 0x0040) != 0;
    let short_gi_40 = (caps & 0x0080) != 0;
    if short_gi_20 || short_gi_40 {
        features.guard_interval = 400;
    } else {
        features.guard_interval = 800;
    }

    // A-MPDU
    if data.len() >= 3 {
        let ampdu = data[2];
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

    // Guard Interval: bits 2-4 = Maximum VHT GI
    // 0 = long GI only (800ns), 1 = short GI (400ns)
    let gi_bits = (caps >> 2) & 0x7;
    features.guard_interval = if gi_bits > 0 { 400 } else { 800 };

    // Spatial streams from MCS set
    if data.len() >= 8 {
        let rx_mcs = u16::from_le_bytes([data[4], data[5]]);
        let nss = count_supported_streams_from_mcs_map(rx_mcs);
        if nss > 0 {
            features.spatial_streams = nss;
        }
    }
}

fn parse_he_capabilities(data: &[u8], features: &mut PerformanceFeatures) {
    if data.len() < 19 {
        return;
    }

    let phy_cap = u32::from_le_bytes([data[7], data[8], data[9], data[10]]);

    // OFDMA
    features.ofdma = true;
    features.bss_coloring = true;
    features.max_qam = 1024;

    // HE supports multiple guard intervals: 0.8, 1.6, 3.2 us
    // Default to 800ns, but HE typically supports all
    features.guard_interval = 800;

    // 160MHz support
    if (phy_cap & 0x04) != 0 {
        // Supports 160MHz
    }

    // Default spatial streams for WiFi 6
    if data.len() >= 21 {
        let rx_mcs = u16::from_le_bytes([data[18], data[19]]);
        let nss = count_supported_streams_from_mcs_map(rx_mcs);
        if nss > 0 {
            features.spatial_streams = nss;
        }
    }

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
        features.spatial_streams = 2;
    }
}

fn count_supported_streams_from_mcs_map(mcs_map: u16) -> u8 {
    let mut count = 0u8;
    for index in 0..8 {
        let bits = ((mcs_map >> (index * 2)) & 0x03) as u8;
        if bits != 0x03 {
            count += 1;
        }
    }
    count
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
    // RSN IE format:
    // 2 bytes: version
    // 4 bytes: group cipher (OUI 3 bytes + type 1 byte) - 00-0F-AC + cipher
    // 2 bytes: pairwise cipher count
    // n * 4 bytes: pairwise ciphers (OUI + type each)
    // 2 bytes: auth suite count
    // n * 4 bytes: auth suites (OUI + type each)
    // 2 bytes: RSN capabilities (optional)

    if data.len() < 8 {
        return ("open".to_string(), SecurityDetails::default());
    }

    // Version should be 1
    let _version = u16::from_le_bytes([data[0], data[1]]);

    // Group cipher: OUI (3 bytes at [2,3,4]) + cipher type (1 byte at [5])
    // OUI 00-0F-AC is Microsoft OUI for WPA/WPA2
    let group_cipher_type = data[5];
    let cipher = match group_cipher_type {
        2 => "tkip",
        4 => "ccmp",
        8 => "gcmp",
        _ => "unknown",
    };

    // Pairwise cipher count at offset 6
    if data.len() >= 8 {
        let pairwise_count = u16::from_le_bytes([data[6], data[7]]) as usize;
        let auth_offset = 8 + pairwise_count * 4;

        if auth_offset + 2 <= data.len() {
            let auth_count = u16::from_le_bytes([data[auth_offset], data[auth_offset + 1]]) as usize;
            let auth_suite_offset = auth_offset + 2;

            if auth_suite_offset + 4 <= data.len() {
                // Auth suite: OUI (3 bytes) + auth type (1 byte at position +3)
                let auth_type = data[auth_suite_offset + 3];

                let (sec_type, auth_method) = match auth_type {
                    1 => ("wpa2-ent", "eap"),
                    2 => ("wpa2", "psk"),
                    4 => ("wpa3", "sae"),
                    8 => ("wpa3-ent", "eap"),
                    _ => ("wpa2", "psk"),
                };

                // Check for PMF (802.11w) - in RSN capabilities at end
                let caps_offset = auth_suite_offset + 4 + (auth_count - 1) * 4;
                let (pmf_capable, pmf_required) = if caps_offset + 2 <= data.len() {
                    let caps = u16::from_le_bytes([data[caps_offset], data[caps_offset + 1]]);
                    ((caps & 0x0080) != 0, (caps & 0x0100) != 0)
                } else {
                    (false, false)
                };

                return (sec_type.to_string(), SecurityDetails {
                    security_type: sec_type.to_string(),
                    auth_method: auth_method.to_string(),
                    cipher: cipher.to_string(),
                    key_mgmt: vec![auth_method.to_string()],
                    is_enterprise: auth_type == 1 || auth_type == 8,
                    is_wpa3_transition: auth_type == 2 && pmf_capable,
                    pmf_required,
                    pmf_capable,
                });
            }
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
        
        let parsed = parse_ie_content(id, data);
        let (summary, vendor_name, display_fields) = describe_ie(id, data, &parsed);

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
            summary,
            vendor_name,
            display_fields,
            parsed,
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

fn field(label: &str, value: impl Into<String>) -> ParsedField {
    ParsedField {
        label: label.to_string(),
        value: value.into(),
        highlighted: false,
    }
}

fn highlighted_field(label: &str, value: impl Into<String>) -> ParsedField {
    ParsedField {
        label: label.to_string(),
        value: value.into(),
        highlighted: true,
    }
}

fn format_vendor_oui(data: &[u8]) -> Option<String> {
    if data.len() < 3 {
        None
    } else {
        Some(format!("{:02X}:{:02X}:{:02X}", data[0], data[1], data[2]))
    }
}

fn bool_text(value: bool) -> &'static str {
    if value { "Supported" } else { "Not supported" }
}

fn describe_ie(
    id: u8,
    data: &[u8],
    parsed: &HashMap<String, serde_json::Value>,
) -> (String, Option<String>, Vec<ParsedField>) {
    match id {
        0 => {
            let ssid = parsed
                .get("ssid")
                .and_then(|value| value.as_str())
                .filter(|value| !value.is_empty())
                .unwrap_or("<hidden>");
            (
                format!("SSID {}", ssid),
                None,
                vec![highlighted_field("SSID", ssid)],
            )
        }
        1 | 50 => {
            let rates = parsed
                .get("rates")
                .and_then(|value| value.as_str())
                .unwrap_or("No rate information");
            (
                "Supported legacy/basic rates".to_string(),
                None,
                vec![field("Rates", rates)],
            )
        }
        3 => {
            let channel = parsed
                .get("channel")
                .and_then(|value| value.as_u64())
                .unwrap_or_default();
            (
                format!("Primary channel {}", channel),
                None,
                vec![highlighted_field("Primary channel", channel.to_string())],
            )
        }
        7 => {
            let mut fields = Vec::new();
            if data.len() >= 2 {
                let country = String::from_utf8_lossy(&data[0..2]).trim().to_string();
                if !country.is_empty() {
                    fields.push(highlighted_field("Country", country.clone()));
                    if data.len() >= 6 {
                        fields.push(field("Triplet", format!("CH {}-{} / {} dBm", data[2], data[2] + data[3].saturating_sub(1), data[4])));
                    }
                    return (format!("Country {}", country), None, fields);
                }
            }
            ("Country information".to_string(), None, fields)
        }
        11 => {
            let stations = parsed
                .get("stationCount")
                .and_then(|value| value.as_u64())
                .unwrap_or_default();
            let utilization = parsed
                .get("channelUtilization")
                .and_then(|value| value.as_u64())
                .map(|raw| ((raw as f32 / 255.0) * 100.0).round() as u64)
                .unwrap_or_default();
            (
                format!("{} stations, {}% channel utilization", stations, utilization),
                None,
                vec![
                    highlighted_field("Associated stations", stations.to_string()),
                    field("Channel utilization", format!("{}%", utilization)),
                ],
            )
        }
        45 => describe_ht_capabilities(data),
        48 => describe_rsn_information(data),
        61 => describe_ht_operation(data),
        70 => (
            "802.11k radio measurement enabled".to_string(),
            None,
            vec![highlighted_field("RRM", "Enabled")],
        ),
        127 => describe_extended_capabilities(data),
        191 => describe_vht_capabilities(data),
        192 => describe_vht_operation(data),
        221 => describe_vendor_specific(data),
        255 if !data.is_empty() => match data[0] {
            35 => describe_he_capabilities(data),
            36 => describe_he_operation(data),
            107 => (
                "Multi-Link Operation element".to_string(),
                None,
                vec![highlighted_field("MLO", "Present")],
            ),
            108 => describe_eht_capabilities(data),
            _ => (
                format!("Extension element {}", data[0]),
                None,
                vec![field("Extension ID", data[0].to_string())],
            ),
        },
        _ => (
            "Raw information element".to_string(),
            None,
            vec![field("Bytes", data.len().to_string())],
        ),
    }
}

fn describe_ht_capabilities(data: &[u8]) -> (String, Option<String>, Vec<ParsedField>) {
    if data.len() < 3 {
        return ("HT capabilities".to_string(), None, Vec::new());
    }

    let caps = u16::from_le_bytes([data[0], data[1]]);
    let width = if (caps & 0x02) != 0 { "20/40 MHz" } else { "20 MHz" };
    let short_gi = (caps & 0x0040) != 0 || (caps & 0x0080) != 0;
    let txbf = (caps & 0x1000) != 0;
    let streams = if data.len() >= 19 {
        let mcs = &data[3..19];
        (0..4).filter(|index| mcs[*index] != 0).count().max(1)
    } else {
        1
    };

    (
        format!("Wi-Fi 4 PHY, {} stream(s), {}", streams, width),
        None,
        vec![
            highlighted_field("Channel width", width),
            highlighted_field("Spatial streams", streams.to_string()),
            field("Short GI", bool_text(short_gi)),
            field("Tx beamforming", bool_text(txbf)),
        ],
    )
}

fn describe_rsn_information(data: &[u8]) -> (String, Option<String>, Vec<ParsedField>) {
    let (security, details) = parse_rsn(data);
    (
        format!("{} security", security.to_uppercase()),
        None,
        vec![
            highlighted_field("Security", security.to_uppercase()),
            field("Authentication", details.auth_method),
            field("Cipher", details.cipher),
            field("PMF", if details.pmf_required { "Required".to_string() } else if details.pmf_capable { "Capable".to_string() } else { "Not advertised".to_string() }),
        ],
    )
}

fn describe_ht_operation(data: &[u8]) -> (String, Option<String>, Vec<ParsedField>) {
    if data.len() < 2 {
        return ("HT operation".to_string(), None, Vec::new());
    }

    let secondary_offset = match data[1] & 0x03 {
        1 => "above",
        3 => "below",
        _ => "none",
    };
    let width = if secondary_offset == "none" { "20 MHz" } else { "40 MHz" };

    (
        format!("Primary CH {}, {}", data[0], width),
        None,
        vec![
            highlighted_field("Primary channel", data[0].to_string()),
            field("Secondary channel", secondary_offset),
            field("Operating width", width),
        ],
    )
}

fn describe_extended_capabilities(data: &[u8]) -> (String, Option<String>, Vec<ParsedField>) {
    let bss_transition = data.get(2).map(|byte| (byte & 0x08) != 0).unwrap_or(false);
    let wnm_sleep = data.get(2).map(|byte| (byte & 0x40) != 0).unwrap_or(false);

    (
        "Extended roaming and management capabilities".to_string(),
        None,
        vec![
            highlighted_field("BSS transition (11v)", bool_text(bss_transition)),
            field("WNM sleep", bool_text(wnm_sleep)),
        ],
    )
}

fn describe_vht_capabilities(data: &[u8]) -> (String, Option<String>, Vec<ParsedField>) {
    if data.len() < 8 {
        return ("VHT capabilities".to_string(), None, Vec::new());
    }

    let caps = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let rx_mcs = u16::from_le_bytes([data[4], data[5]]);
    let streams = count_supported_streams_from_mcs_map(rx_mcs).max(1);
    let su_bfer = (caps & (1 << 19)) != 0;
    let su_bfee = (caps & (1 << 20)) != 0;
    let mu_bfer = (caps & (1 << 21)) != 0;
    let short_gi_80 = (caps & (1 << 5)) != 0;
    let short_gi_160 = (caps & (1 << 6)) != 0;

    (
        format!("Wi-Fi 5 PHY, {} stream(s), VHT beamforming", streams),
        None,
        vec![
            highlighted_field("Spatial streams", streams.to_string()),
            field("Short GI 80 MHz", bool_text(short_gi_80)),
            field("Short GI 160 MHz", bool_text(short_gi_160)),
            field("SU beamformer", bool_text(su_bfer)),
            field("SU beamformee", bool_text(su_bfee)),
            field("MU beamformer", bool_text(mu_bfer)),
        ],
    )
}

fn describe_vht_operation(data: &[u8]) -> (String, Option<String>, Vec<ParsedField>) {
    if data.len() < 3 {
        return ("VHT operation".to_string(), None, Vec::new());
    }

    let width = match data[0] {
        1 => "80 MHz",
        2 => "160 MHz",
        3 => "80+80 MHz",
        _ => "20/40 MHz",
    };

    (
        format!("{} operation, center segment {}", width, data[1]),
        None,
        vec![
            highlighted_field("Operating width", width),
            field("Center segment 0", data[1].to_string()),
            field("Center segment 1", data[2].to_string()),
        ],
    )
}

fn describe_vendor_specific(data: &[u8]) -> (String, Option<String>, Vec<ParsedField>) {
    let vendor_oui = format_vendor_oui(data);
    let vendor_name = vendor_oui
        .as_deref()
        .and_then(lookup_oui)
        .map(|value| value.to_string());
    let subtype = data.get(3).copied();

    let mut fields = Vec::new();
    if let Some(name) = &vendor_name {
        fields.push(highlighted_field("Vendor", name.clone()));
    }
    if let Some(oui) = &vendor_oui {
        fields.push(field("OUI", oui.clone()));
    }
    if let Some(subtype) = subtype {
        fields.push(field("Subtype", format!("0x{:02X}", subtype)));
    }

    if matches!(vendor_oui.as_deref(), Some("00:50:F2")) {
        match subtype {
            Some(0x02) => {
                return (
                    "Microsoft WMM/WME information".to_string(),
                    Some("Microsoft".to_string()),
                    {
                        fields.push(highlighted_field("Feature", "WMM / WME"));
                        fields
                    },
                );
            }
            Some(0x04) => {
                return (
                    "Microsoft WPS information".to_string(),
                    Some("Microsoft".to_string()),
                    {
                        fields.push(highlighted_field("Feature", "WPS"));
                        fields
                    },
                );
            }
            _ => {}
        }
    }

    (
        format!(
            "{} vendor information",
            vendor_name.clone().unwrap_or_else(|| "Vendor-specific".to_string())
        ),
        vendor_name,
        fields,
    )
}

fn describe_he_capabilities(data: &[u8]) -> (String, Option<String>, Vec<ParsedField>) {
    if data.len() < 20 {
        return ("HE capabilities".to_string(), None, Vec::new());
    }

    let phy0 = data[7];
    let phy1 = data[8];
    let phy3 = data[10];
    let rx_mcs = u16::from_le_bytes([data[18], data[19]]);
    let streams = count_supported_streams_from_mcs_map(rx_mcs).max(1);

    let width = if (phy0 & 0x08) != 0 {
        "20/40/80/160 MHz"
    } else if (phy0 & 0x04) != 0 {
        "20/40/80 MHz"
    } else {
        "20/40 MHz"
    };

    (
        format!("Wi-Fi 6 PHY, {} stream(s), {}", streams, width),
        None,
        vec![
            highlighted_field("Channel widths", width),
            highlighted_field("Spatial streams", streams.to_string()),
            field("LDPC coding", bool_text((phy1 & 0x20) != 0)),
            field("STBC Rx <= 80 MHz", bool_text((phy1 & 0x80) != 0)),
            field("Full-bandwidth UL MU-MIMO", bool_text((phy3 & 0x20) != 0)),
            field("SU beamformer", bool_text((phy3 & 0x80) != 0)),
            field("SU beamformee", bool_text((phy3 & 0x01) != 0)),
            field("MU beamformer", bool_text((phy3 & 0x02) != 0)),
        ],
    )
}

fn describe_he_operation(data: &[u8]) -> (String, Option<String>, Vec<ParsedField>) {
    if data.len() < 8 {
        return ("HE operation".to_string(), None, Vec::new());
    }

    let bss_color = data.get(6).map(|byte| byte & 0x3f).unwrap_or_default();
    (
        format!("HE operation, BSS color {}", bss_color),
        None,
        vec![
            highlighted_field("BSS color", bss_color.to_string()),
            field("Default PE duration", data.get(3).copied().unwrap_or_default().to_string()),
        ],
    )
}

fn describe_eht_capabilities(data: &[u8]) -> (String, Option<String>, Vec<ParsedField>) {
    if data.len() < 9 {
        return ("EHT capabilities".to_string(), None, Vec::new());
    }

    let phy_cap = u32::from_le_bytes([data[5], data[6], data[7], data[8]]);
    let width = match (phy_cap & 0x03) as u8 {
        3 => "Up to 320 MHz",
        2 => "Up to 160 MHz",
        1 => "Up to 80 MHz",
        _ => "20/40 MHz",
    };
    let qam_4096 = (phy_cap & 0x000F0000) != 0;

    (
        format!("Wi-Fi 7 PHY, {}", width),
        None,
        vec![
            highlighted_field("Channel widths", width),
            field("4096-QAM", bool_text(qam_4096)),
            field("Multi-link support", "Check MLO extension element"),
        ],
    )
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
