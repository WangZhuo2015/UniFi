//! IE (Information Element) Parser
//!
//! Pure functions for parsing 802.11 IE data.
//! No external dependencies on platform-specific code.

use crate::types::*;
use crate::vendor::lookup_oui;
use std::collections::HashMap;

// ============================================================================
// Extended IE Names
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

    // Detailed structures
    let mut channel_info: Option<ChannelInfo> = None;
    let mut spatial_stream_info: Option<SpatialStreamInfo> = None;
    let mut ofdma_info: Option<OfdmaInfo> = None;
    let mut twt_info: Option<TwtInfo> = None;
    let mut wifi7_features: Option<Wifi7Features> = None;
    let mut mcs_info: Option<McsInfo> = None;

    // Scan IE data - first pass for basic info
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
                let (ss_info, mcs) = parse_ht_capabilities_detailed(data);
                spatial_stream_info = ss_info.or(spatial_stream_info);
                mcs_info = mcs.or(mcs_info);
                apply_ht_capabilities(data, &mut features);
            }
            48 => {
                // RSN (WPA2/WPA3)
                let (sec, details) = parse_rsn(data);
                security = sec;
                security_details = details;
            }
            61 => {
                // HT Operation
                channel_info = parse_ht_operation(data).or(channel_info);
            }
            70 => {
                // RM Capabilities (802.11k) - detailed parsing
                protocols.rrm = true;
                if len >= 5 {
                    protocols.neighbor_report = (data[0] & 0x01) != 0;
                    protocols.beacon_report = (data[0] & 0x80) != 0;
                }
            }
            127 => {
                // Extended Capabilities - detailed parsing
                parse_extended_capabilities_detailed(data, &mut protocols);
            }
            191 => {
                // VHT Capabilities (WiFi 5)
                if !standards.contains(&"ac".to_string()) {
                    standards.push("ac".to_string());
                }
                let (ss_info, mcs) = parse_vht_capabilities_detailed(data);
                spatial_stream_info = ss_info.or(spatial_stream_info);
                mcs_info = mcs.or(mcs_info);
                apply_vht_capabilities(data, &mut features);
            }
            192 => {
                // VHT Operation
                channel_info = parse_vht_operation(data).or(channel_info);
            }
            221 => {
                // Vendor Specific - check for WMM/WME and WPS
                if len >= 4 {
                    if data[0..3] == [0x00, 0x50, 0xF2] {
                        match data[3] {
                            0x02 => {
                                // WMM/WME
                                protocols.wmm = true;
                                if len >= 6 {
                                    protocols.wmm_uapsd = (data[5] & 0x80) != 0;
                                }
                            }
                            0x04 => wps = true,
                            _ => {}
                        }
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
                        let (ss_info, ofdma, twt, mcs) = parse_he_capabilities_detailed(data);
                        spatial_stream_info = ss_info.or(spatial_stream_info);
                        ofdma_info = ofdma.or(ofdma_info);
                        twt_info = twt.or(twt_info);
                        mcs_info = mcs.or(mcs_info);
                        apply_he_capabilities(data, &mut features);
                    }
                    36 => {
                        // HE Operation - parse for channel info
                        let ch = parse_he_operation(data);
                        channel_info = ch.or(channel_info);
                    }
                    106 => {
                        // EHT Operation
                        channel_info = parse_eht_operation(data).or(channel_info);
                    }
                    107 => {
                        // MLO
                        features.mlo = true;
                        wifi7_features = Some(Wifi7Features {
                            mlo: parse_mlo_element(data),
                            punctured_preamble: false,
                            multi_ru: false,
                        });
                    }
                    108 => {
                        // EHT Capabilities (WiFi 7)
                        if !standards.contains(&"be".to_string()) {
                            standards.push("be".to_string());
                        }
                        let (ss_info, wifi7, mcs) = parse_eht_capabilities_detailed(data);
                        spatial_stream_info = ss_info.or(spatial_stream_info);
                        wifi7_features = wifi7.or(wifi7_features);
                        mcs_info = mcs.or(mcs_info);
                        apply_eht_capabilities(data, &mut features);
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

    // Apply detailed info to features
    features.channel_info = channel_info;
    features.spatial_stream_info = spatial_stream_info;
    features.ofdma_info = ofdma_info;
    features.twt_info = twt_info;
    features.wifi7_features = wifi7_features;
    features.mcs_info = mcs_info;

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

fn update_max_supported_width(features: &mut PerformanceFeatures, width: u16) {
    if width > features.max_supported_width {
        features.max_supported_width = width;
    }
}

// ============================================================================
// Detailed Parsing Functions (return structured data)
// ============================================================================

/// Parse HT Capabilities - returns detailed spatial stream and MCS info
fn parse_ht_capabilities_detailed(data: &[u8]) -> (Option<SpatialStreamInfo>, Option<McsInfo>) {
    if data.len() < 26 {
        return (None, None);
    }

    let caps = u16::from_le_bytes([data[0], data[1]]);
    let mcs = &data[3..19];

    // Count spatial streams from MCS set
    let mut tx_streams = 0u8;
    let mut rx_streams = 0u8;
    for i in 0..4 {
        if mcs[i] != 0 {
            tx_streams = (i + 1) as u8;
            rx_streams = (i + 1) as u8;
        }
    }

    // Find max MCS index
    let max_mcs = if data[3] & 0xFF != 0 {
        // Check which MCS indices are supported in first byte
        let mut max = 0u8;
        for mcs_idx in 0..8 {
            if (mcs[0] >> mcs_idx) & 1 != 0 {
                max = mcs_idx;
            }
        }
        Some(max)
    } else {
        None
    };

    let ss_info = if tx_streams > 0 {
        Some(SpatialStreamInfo {
            tx_streams: Some(tx_streams),
            rx_streams: Some(rx_streams),
            client_tx: None,
            client_rx: None,
            effective_streams: None,
        })
    } else {
        None
    };

    let mcs_info = McsInfo {
        max_mcs,
        current_mcs: None,
        max_modulation: Some(Modulation::QAM64), // HT supports up to 64-QAM
    };

    (ss_info, Some(mcs_info))
}

/// Apply HT capabilities to PerformanceFeatures (legacy fields)
fn apply_ht_capabilities(data: &[u8], features: &mut PerformanceFeatures) {
    if data.len() < 26 {
        return;
    }

    let caps = u16::from_le_bytes([data[0], data[1]]);
    update_max_supported_width(features, if (caps & 0x02) != 0 { 40 } else { 20 });

    let mcs = &data[3..19];
    for i in 0..4 {
        if mcs[i] != 0 {
            features.spatial_streams = (i + 1) as u8;
        }
    }

    features.su_mimo = features.spatial_streams > 1;
    features.su_beamformer = (caps & 0x1000) != 0;

    let short_gi_20 = (caps & 0x0040) != 0;
    let short_gi_40 = (caps & 0x0080) != 0;
    features.guard_interval = if short_gi_20 || short_gi_40 { 400 } else { 800 };

    if data.len() >= 3 {
        features.ampdu_length = (data[2] & 0x03) + 1;
    }
}

/// Parse HT Operation IE (61) for channel info
fn parse_ht_operation(data: &[u8]) -> Option<ChannelInfo> {
    if data.len() < 3 {
        return None;
    }

    let primary_channel = data[0];
    let secondary_offset = match data[1] & 0x03 {
        1 => Some(SecondaryChannelOffset::Above),
        3 => Some(SecondaryChannelOffset::Below),
        _ => None,
    };

    let bandwidth = if secondary_offset.is_some() {
        ChannelBandwidth::MHz40
    } else {
        ChannelBandwidth::MHz20
    };

    let secondary = secondary_offset.map(|offset| {
        match offset {
            SecondaryChannelOffset::Above => primary_channel + 4,
            SecondaryChannelOffset::Below => primary_channel - 4,
        }
    });

    Some(ChannelInfo {
        primary: primary_channel,
        bandwidth,
        secondary,
        secondary_offset,
        center_freq_0: None,
        center_freq_1: None,
        frequency: None,
    })
}

/// Parse VHT Capabilities - returns detailed info
fn parse_vht_capabilities_detailed(data: &[u8]) -> (Option<SpatialStreamInfo>, Option<McsInfo>) {
    if data.len() < 12 {
        return (None, None);
    }

    // Spatial streams from MCS map
    if data.len() >= 8 {
        let rx_mcs = u16::from_le_bytes([data[4], data[5]]);
        let tx_mcs = u16::from_le_bytes([data[6], data[7]]);
        let rx_nss = count_supported_streams_from_mcs_map(rx_mcs);
        let tx_nss = count_supported_streams_from_mcs_map(tx_mcs);

        let ss_info = if rx_nss > 0 {
            Some(SpatialStreamInfo {
                tx_streams: Some(tx_nss),
                rx_streams: Some(rx_nss),
                client_tx: None,
                client_rx: None,
                effective_streams: None,
            })
        } else {
            None
        };

        // Max MCS for VHT is typically 9 (256-QAM)
        let mcs_info = McsInfo {
            max_mcs: Some(9),
            current_mcs: None,
            max_modulation: Some(Modulation::QAM256),
        };

        (ss_info, Some(mcs_info))
    } else {
        (None, None)
    }
}

/// Apply VHT capabilities to PerformanceFeatures
fn apply_vht_capabilities(data: &[u8], features: &mut PerformanceFeatures) {
    if data.len() < 12 {
        return;
    }

    let caps = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let supports_160 = (caps & (1 << 2)) != 0 || (caps & (1 << 3)) != 0;
    update_max_supported_width(features, if supports_160 { 160 } else { 80 });

    features.su_mimo = features.spatial_streams > 1;
    features.mu_mimo = (caps & (1 << 19)) != 0;
    features.ul_mu_mimo = false;

    features.su_beamformer = (caps & (1 << 19)) != 0;
    features.su_beamformee = (caps & (1 << 20)) != 0;
    features.mu_beamformer = (caps & (1 << 21)) != 0;

    features.max_qam = 256;

    let gi_bits = (caps >> 2) & 0x7;
    features.guard_interval = if gi_bits > 0 { 400 } else { 800 };

    if data.len() >= 8 {
        let rx_mcs = u16::from_le_bytes([data[4], data[5]]);
        let nss = count_supported_streams_from_mcs_map(rx_mcs);
        if nss > 0 {
            features.spatial_streams = nss;
            features.su_mimo = nss > 1;
        }
    }
}

/// Parse VHT Operation IE (192) for channel info
fn parse_vht_operation(data: &[u8]) -> Option<ChannelInfo> {
    if data.len() < 3 {
        return None;
    }

    let bandwidth = match data[0] {
        0 => ChannelBandwidth::MHz20, // 20 or 40 (check HT Operation)
        1 => ChannelBandwidth::MHz80,
        2 => ChannelBandwidth::MHz160,
        3 => ChannelBandwidth::MHz80Plus80,
        _ => ChannelBandwidth::MHz20,
    };

    let center_freq_0 = if data[1] > 0 { Some(data[1] as u16) } else { None };
    let center_freq_1 = if data[2] > 0 { Some(data[2] as u16) } else { None };

    Some(ChannelInfo {
        primary: 0, // Will be filled from HT Operation or DS Parameter
        bandwidth,
        secondary: None,
        secondary_offset: None,
        center_freq_0,
        center_freq_1,
        frequency: None,
    })
}

/// Parse HE Capabilities - returns detailed info including OFDMA and TWT
fn parse_he_capabilities_detailed(data: &[u8]) -> (Option<SpatialStreamInfo>, Option<OfdmaInfo>, Option<TwtInfo>, Option<McsInfo>) {
    if data.len() < 21 {
        return (None, None, None, None);
    }

    // HE MAC Capabilities (bytes 2-6)
    let mac_cap = &data[2..7];

    // TWT capabilities from MAC Capabilities
    // Bit 5: TWT Requester support
    // Bit 6: TWT Responder support
    let twt_requester = (mac_cap[0] & 0x20) != 0;
    let twt_responder = (mac_cap[0] & 0x40) != 0;

    let twt_info = if twt_requester || twt_responder {
        Some(TwtInfo {
            broadcast_twt: (mac_cap[1] & 0x01) != 0,  // Bit 8
            individual_twt: true,
            twt_requester,
            twt_responder,
        })
    } else {
        None
    };

    // HE PHY Capabilities (bytes 7-11)
    let phy_cap = u32::from_le_bytes([data[7], data[8], data[9], data[10]]);

    // OFDMA support
    // DL OFDMA: PHY bit 79 (byte 9, bit 7)
    // UL OFDMA: PHY bit 80 (byte 10, bit 0)
    let dl_ofdma = (data[9] & 0x80) != 0;
    let ul_ofdma = (data[10] & 0x01) != 0;

    // RU sizes - HE typically supports 26, 52, 106, 242, 484, 996
    let ru_sizes = if dl_ofdma || ul_ofdma {
        vec![RuSize::R26, RuSize::R52, RuSize::R106, RuSize::R242, RuSize::R484, RuSize::R996]
    } else {
        vec![]
    };

    let ofdma_info = if dl_ofdma || ul_ofdma {
        Some(OfdmaInfo {
            dl_ofdma,
            ul_ofdma,
            ru_sizes,
        })
    } else {
        None
    };

    // Spatial streams from HE MCS map (bytes 18-21)
    let rx_mcs = u16::from_le_bytes([data[18], data[19]]);
    let tx_mcs = u16::from_le_bytes([data[20], data[21]]);
    let rx_nss = count_supported_streams_from_mcs_map(rx_mcs);
    let tx_nss = count_supported_streams_from_mcs_map(tx_mcs);

    let ss_info = if rx_nss > 0 {
        Some(SpatialStreamInfo {
            tx_streams: Some(tx_nss),
            rx_streams: Some(rx_nss),
            client_tx: None,
            client_rx: None,
            effective_streams: None,
        })
    } else {
        None
    };

    // HE supports up to 1024-QAM (MCS 10-11)
    let mcs_info = McsInfo {
        max_mcs: Some(11),
        current_mcs: None,
        max_modulation: Some(Modulation::QAM1024),
    };

    (ss_info, ofdma_info, twt_info, Some(mcs_info))
}

/// Apply HE capabilities to PerformanceFeatures
fn apply_he_capabilities(data: &[u8], features: &mut PerformanceFeatures) {
    if data.len() < 19 {
        return;
    }

    let phy_cap = u32::from_le_bytes([data[7], data[8], data[9], data[10]]);
    let mut max_width = 80;

    features.ofdma = true;
    features.bss_coloring = true;
    features.max_qam = 1024;

    features.su_mimo = features.spatial_streams > 1;
    features.mu_mimo = true;

    features.guard_interval = 800;

    if data.len() >= 11 {
        let phy3 = data[10];
        features.su_beamformer = (phy3 & 0x80) != 0;
        features.su_beamformee = (phy3 & 0x01) != 0;
        features.mu_beamformer = (phy3 & 0x02) != 0;
        features.ul_mu_mimo = (phy3 & 0x20) != 0;
    }

    if (phy_cap & 0x08) != 0 {
        max_width = 160;
    } else if (phy_cap & 0x04) != 0 {
        max_width = 80;
    }
    update_max_supported_width(features, max_width);

    if data.len() >= 21 {
        let rx_mcs = u16::from_le_bytes([data[18], data[19]]);
        let nss = count_supported_streams_from_mcs_map(rx_mcs);
        if nss > 0 {
            features.spatial_streams = nss;
            features.su_mimo = nss > 1;
        }
    }

    if features.spatial_streams == 0 {
        features.spatial_streams = 2;
    }
}

/// Parse HE Operation for channel info
fn parse_he_operation(data: &[u8]) -> Option<ChannelInfo> {
    if data.len() < 8 {
        return None;
    }

    // HE Operation contains BSS color and channel info
    // For now, just extract BSS color info (not full channel info)
    None
}

/// Parse EHT Capabilities - returns detailed info including WiFi 7 features
fn parse_eht_capabilities_detailed(data: &[u8]) -> (Option<SpatialStreamInfo>, Option<Wifi7Features>, Option<McsInfo>) {
    if data.len() < 9 {
        return (None, None, None);
    }

    let phy_cap = u32::from_le_bytes([data[5], data[6], data[7], data[8]]);

    // WiFi 7 features
    // Punctured preamble: PHY bit 21-23
    // Multi-RU: PHY bit 24
    let punctured_preamble = (phy_cap & 0x00200000) != 0;
    let multi_ru = (phy_cap & 0x01000000) != 0;

    // EHT MCS map (bytes 9+)
    // For 320MHz and 4096-QAM
    let supports_320 = (phy_cap & 0x03) == 0x03;
    let supports_4096qam = (phy_cap & 0x000F0000) != 0;

    let wifi7_features = Some(Wifi7Features {
        mlo: None, // MLO info comes from separate element
        punctured_preamble,
        multi_ru,
    });

    // Spatial streams - EHT supports up to 16 streams
    // For simplicity, we check the basic NSS
    let ss_info = Some(SpatialStreamInfo {
        tx_streams: Some(8), // EHT APs typically support 8+ streams
        rx_streams: Some(8),
        client_tx: None,
        client_rx: None,
        effective_streams: None,
    });

    let mcs_info = McsInfo {
        max_mcs: Some(15), // EHT extends MCS range
        current_mcs: None,
        max_modulation: if supports_4096qam { Some(Modulation::QAM4096) } else { Some(Modulation::QAM1024) },
    };

    (ss_info, wifi7_features, Some(mcs_info))
}

/// Apply EHT capabilities to PerformanceFeatures
fn apply_eht_capabilities(data: &[u8], features: &mut PerformanceFeatures) {
    if data.len() < 9 {
        return;
    }

    let phy_cap = u32::from_le_bytes([data[5], data[6], data[7], data[8]]);

    features.ofdma = true;
    features.bss_coloring = true;
    features.su_mimo = features.spatial_streams > 1;
    features.mu_mimo = true;
    features.ul_mu_mimo = true;
    features.max_qam = 4096;

    features.su_beamformer = true;
    features.su_beamformee = true;
    features.mu_beamformer = true;

    let max_width = match phy_cap & 0x03 {
        0x03 => 320,
        0x02 => 160,
        0x01 => 80,
        _ => 40,
    };
    update_max_supported_width(features, max_width);

    if (phy_cap & 0x000F0000) != 0 {
        features.max_qam = 4096;
    }

    if features.spatial_streams == 0 {
        features.spatial_streams = 2;
    }
}

/// Parse EHT Operation for channel info (320MHz)
fn parse_eht_operation(data: &[u8]) -> Option<ChannelInfo> {
    if data.len() < 4 {
        return None;
    }

    // EHT Operation extends VHT Operation
    // Channel width in bits 2-4 of byte 1
    let width_bits = (data[1] >> 2) & 0x07;

    let bandwidth = match width_bits {
        4 => ChannelBandwidth::MHz320,
        3 => ChannelBandwidth::MHz160,
        2 => ChannelBandwidth::MHz80,
        1 => ChannelBandwidth::MHz40,
        _ => ChannelBandwidth::MHz20,
    };

    // Center frequency segments
    let center_freq_0 = if data.len() >= 3 && data[2] > 0 { Some(data[2] as u16) } else { None };
    let center_freq_1 = if data.len() >= 4 && data[3] > 0 { Some(data[3] as u16) } else { None };

    Some(ChannelInfo {
        primary: 0,
        bandwidth,
        secondary: None,
        secondary_offset: None,
        center_freq_0,
        center_freq_1,
        frequency: None,
    })
}

/// Parse MLO element for WiFi 7 Multi-Link Operation
fn parse_mlo_element(data: &[u8]) -> Option<MloInfo> {
    if data.len() < 5 {
        return None;
    }

    // Basic MLO info parsing
    // The structure is complex, but we can extract basic link count
    let mlo_type = (data[1] & 0x07);

    // For now, return basic info
    Some(MloInfo {
        enabled: true,
        num_links: 2, // Typical MLO setup
        links: vec![], // Detailed link parsing would require more complex logic
    })
}

/// Parse Extended Capabilities (IE 127) for detailed roaming info
fn parse_extended_capabilities_detailed(data: &[u8], protocols: &mut ProtocolExtensions) {
    if data.is_empty() {
        return;
    }

    // Byte 0 bits:
    // Bit 0: 20/40 BSS Coexistence Management support
    // Bit 1: Extended Channel Switching

    // Byte 1 bits:
    // Bit 4 (bit 12 overall): 802.11k RRM DMS
    // Bit 5 (bit 13 overall): Neighbor Report
    protocols.rrm = data.len() > 1 && (data[1] & 0x10) != 0;
    protocols.neighbor_report = data.len() > 1 && (data[1] & 0x20) != 0;
    protocols.beacon_report = data.len() > 1 && (data[1] & 0x80) != 0;

    // Byte 2 bits:
    // Bit 3 (bit 19 overall): BSS Transition (802.11v)
    // Bit 6 (bit 22 overall): WNM Sleep Mode
    if data.len() > 2 {
        protocols.bss_transition = (data[2] & 0x08) != 0;
        protocols.wnm_sleep = (data[2] & 0x40) != 0;
    }

    // Byte 3 bits:
    // Bit 4 (bit 28 overall): FT over DS (802.11r)
    // Bit 5 (bit 29 overall): FT Resource Request
    if data.len() > 3 {
        protocols.ft_over_ds = (data[3] & 0x10) != 0;
        protocols.ft_resource_request = (data[3] & 0x20) != 0;
    }

    // FT (Fast BSS Transition) - also check for FT support
    protocols.ft = protocols.ft_over_ds || protocols.ft_resource_request;

    // PMF (802.11w) - check from RSN IE, but also from extended caps
    if data.len() > 4 {
        protocols.pmf = (data[4] & 0x40) != 0;
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
    let group_cipher = match group_cipher_type {
        2 => "tkip".to_string(),
        4 => "ccmp".to_string(),
        8 => "gcmp".to_string(),
        _ => "unknown".to_string(),
    };
    let cipher = group_cipher.clone();

    // Parse all pairwise ciphers
    let mut pairwise_ciphers = Vec::new();
    if data.len() >= 8 {
        let pairwise_count = u16::from_le_bytes([data[6], data[7]]) as usize;
        for i in 0..pairwise_count.min(8) {
            let offset = 8 + i * 4 + 3; // OUI (3 bytes) + cipher type (1 byte)
            if offset < data.len() {
                let cipher_type = data[offset];
                let cipher_name = match cipher_type {
                    2 => "tkip".to_string(),
                    4 => "ccmp".to_string(),
                    8 => "gcmp".to_string(),
                    _ => "unknown".to_string(),
                };
                pairwise_ciphers.push(cipher_name);
            }
        }
    }

    // Pairwise cipher count at offset 6
    if data.len() >= 8 {
        let pairwise_count = u16::from_le_bytes([data[6], data[7]]) as usize;
        let auth_offset = 8 + pairwise_count * 4;

        if auth_offset + 2 <= data.len() {
            let auth_count = u16::from_le_bytes([data[auth_offset], data[auth_offset + 1]]) as usize;
            let auth_suite_offset = auth_offset + 2;

            if auth_suite_offset + 4 <= data.len() {
                // Parse all auth suites for key_mgmt
                let mut key_mgmt = Vec::new();
                let mut has_sae = false;
                let mut has_psk = false;
                let mut has_eap = false;

                for i in 0..auth_count.min(4) {
                    let suite_offset = auth_suite_offset + i * 4;
                    if suite_offset + 4 <= data.len() {
                        let auth_type = data[suite_offset + 3];
                        match auth_type {
                            1 => { key_mgmt.push("eap".to_string()); has_eap = true; }
                            2 => { key_mgmt.push("psk".to_string()); has_psk = true; }
                            4 => { key_mgmt.push("sae".to_string()); has_sae = true; }
                            8 => { key_mgmt.push("eap".to_string()); has_eap = true; }
                            _ => key_mgmt.push("unknown".to_string()),
                        }
                    }
                }

                // Primary auth type (first one)
                let auth_type = data[auth_suite_offset + 3];

                let (sec_type, auth_method) = match auth_type {
                    1 => ("wpa2-ent", "eap"),
                    2 => if has_sae { ("wpa3", "sae") } else { ("wpa2", "psk") },
                    4 => ("wpa3", "sae"),
                    8 => ("wpa3-ent", "eap"),
                    _ => ("wpa2", "psk"),
                };

                // Check for PMF (802.11w) - in RSN capabilities at end
                let caps_offset = auth_suite_offset + auth_count * 4;
                let (pmf_capable, pmf_required) = if caps_offset + 2 <= data.len() {
                    let caps = u16::from_le_bytes([data[caps_offset], data[caps_offset + 1]]);
                    ((caps & 0x0080) != 0, (caps & 0x0100) != 0)
                } else {
                    (false, false)
                };

                // WPA3 Transition mode = WPA2-PSK + PMF capable
                let is_wpa3_transition = has_psk && !has_sae && pmf_capable;

                // Check for OWE (Opportunistic Wireless Encryption)
                // OWE uses auth type 18 (0x12) in the AKM suite
                let has_owe = key_mgmt.iter().any(|k| k == "owe");

                return (sec_type.to_string(), SecurityDetails {
                    security_type: sec_type.to_string(),
                    auth_method: auth_method.to_string(),
                    cipher: cipher.clone(),
                    key_mgmt: key_mgmt,
                    is_enterprise: auth_type == 1 || auth_type == 8,
                    is_wpa3_transition,
                    pmf_required,
                    pmf_capable,
                    group_cipher: Some(group_cipher),
                    pairwise_ciphers,
                    sae: has_sae,
                    owe: has_owe,
                });
            }
        }
    }

    ("wpa2".to_string(), SecurityDetails {
        security_type: "wpa2".to_string(),
        auth_method: "psk".to_string(),
        cipher: cipher.clone(),
        key_mgmt: vec!["psk".to_string()],
        is_enterprise: false,
        is_wpa3_transition: false,
        pmf_required: false,
        pmf_capable: false,
        group_cipher: Some(group_cipher),
        pairwise_ciphers,
        sae: false,
        owe: false,
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // IE Name Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_ie_name_common_elements() {
        assert_eq!(ie_name(0), "SSID");
        assert_eq!(ie_name(1), "Supported Rates");
        assert_eq!(ie_name(3), "DS Parameter Set");
        assert_eq!(ie_name(7), "Country");
        assert_eq!(ie_name(11), "QBSS Load");
        assert_eq!(ie_name(45), "HT Capabilities");
        assert_eq!(ie_name(48), "RSN");
        assert_eq!(ie_name(61), "HT Operation");
        assert_eq!(ie_name(127), "Extended Capabilities");
        assert_eq!(ie_name(191), "VHT Capabilities");
        assert_eq!(ie_name(192), "VHT Operation");
        assert_eq!(ie_name(221), "Vendor Specific");
        assert_eq!(ie_name(255), "Extended Element");
        assert_eq!(ie_name(99), "Unknown");
    }

    #[test]
    fn test_ext_ie_name() {
        assert_eq!(ext_ie_name(35), "HE Capabilities");
        assert_eq!(ext_ie_name(36), "HE Operation");
        assert_eq!(ext_ie_name(106), "EHT Operation");
        assert_eq!(ext_ie_name(107), "EHT Multi-Link");
        assert_eq!(ext_ie_name(108), "EHT Capabilities");
        assert_eq!(ext_ie_name(99), "Unknown Extension");
    }

    // -------------------------------------------------------------------------
    // RSN Parsing Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_rsn_wpa2_psk() {
        // RSN IE with WPA2-PSK, CCMP
        // Format: version(2) + group_cipher(4) + pairwise_count(2) + pairwise(4) + auth_count(2) + auth(4)
        let rsn_data: Vec<u8> = vec![
            0x01, 0x00,  // Version 1
            0x00, 0x0F, 0xAC, 0x04,  // Group cipher: CCMP (OUI 00-0F-AC, type 4)
            0x01, 0x00,  // 1 pairwise cipher
            0x00, 0x0F, 0xAC, 0x04,  // CCMP
            0x01, 0x00,  // 1 auth suite
            0x00, 0x0F, 0xAC, 0x02,  // PSK
            0x00, 0x00,  // RSN capabilities
        ];

        let (security, details) = parse_rsn(&rsn_data);
        assert_eq!(security, "wpa2");
        assert_eq!(details.auth_method, "psk");
        assert_eq!(details.cipher, "ccmp");
        assert!(!details.is_enterprise);
    }

    #[test]
    fn test_parse_rsn_wpa3_sae() {
        // RSN IE with WPA3-SAE
        // Note: Auth type 8 = SAE in WPA3 context
        let rsn_data: Vec<u8> = vec![
            0x01, 0x00,  // Version 1
            0x00, 0x0F, 0xAC, 0x04,  // Group cipher: CCMP
            0x01, 0x00,  // 1 pairwise cipher
            0x00, 0x0F, 0xAC, 0x04,  // CCMP
            0x01, 0x00,  // 1 auth suite
            0x00, 0x0F, 0xAC, 0x04,  // Auth type 4 = SAE for WPA3-Personal
            0x80, 0x00,  // RSN capabilities with PMF capable
        ];

        let (security, details) = parse_rsn(&rsn_data);
        assert_eq!(security, "wpa3");
        assert_eq!(details.auth_method, "sae");
        assert!(details.sae);
    }

    #[test]
    fn test_parse_rsn_short_data() {
        let short_data = vec![0x01, 0x00];
        let (security, _details) = parse_rsn(&short_data);
        assert_eq!(security, "open");
    }

    // -------------------------------------------------------------------------
    // HT Capabilities Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_ht_capabilities_detailed() {
        // Minimal HT Capabilities IE (26 bytes)
        let mut ht_data = vec![0u8; 26];
        ht_data[0] = 0x6C;  // HT Capabilities Info: 40MHz + LDPC
        ht_data[1] = 0x01;  // 40MHz capable
        ht_data[2] = 0x03;  // A-MPDU params
        ht_data[3] = 0xFF;  // MCS set (1 stream, MCS 0-7)

        let (ss_info, mcs_info) = parse_ht_capabilities_detailed(&ht_data);

        assert!(ss_info.is_some());
        let ss = ss_info.unwrap();
        assert_eq!(ss.tx_streams, Some(1));
        assert_eq!(ss.rx_streams, Some(1));

        assert!(mcs_info.is_some());
        let mcs = mcs_info.unwrap();
        assert_eq!(mcs.max_modulation, Some(Modulation::QAM64));
    }

    // -------------------------------------------------------------------------
    // VHT Capabilities Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_vht_capabilities_detailed() {
        // Minimal VHT Capabilities IE (12 bytes)
        let mut vht_data = vec![0u8; 12];
        vht_data[0] = 0x38;  // VHT Capabilities: 160MHz + SU beamformer
        vht_data[4] = 0xFF;  // RX MCS map (1 stream)
        vht_data[6] = 0xFF;  // TX MCS map (1 stream)

        let (_ss_info, mcs_info) = parse_vht_capabilities_detailed(&vht_data);

        assert!(mcs_info.is_some());
        let mcs = mcs_info.unwrap();
        assert_eq!(mcs.max_modulation, Some(Modulation::QAM256));
    }

    // -------------------------------------------------------------------------
    // HE Capabilities Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_he_capabilities_detailed() {
        // Minimal HE Capabilities IE (21+ bytes)
        let mut he_data = vec![0u8; 22];
        he_data[0] = 35;  // Extension ID: HE Capabilities
        he_data[7] = 0x0C;  // PHY capabilities: 80MHz
        he_data[9] = 0x80;  // DL OFDMA
        he_data[10] = 0x03; // UL OFDMA + more

        let (_ss_info, ofdma_info, _twt_info, mcs_info) = parse_he_capabilities_detailed(&he_data);

        assert!(ofdma_info.is_some());
        let ofdma = ofdma_info.unwrap();
        assert!(ofdma.dl_ofdma);

        assert!(mcs_info.is_some());
        let mcs = mcs_info.unwrap();
        assert_eq!(mcs.max_modulation, Some(Modulation::QAM1024));
    }

    // -------------------------------------------------------------------------
    // Channel Width Detection Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_detect_channel_width_ht_40() {
        // IE data with HT Capabilities indicating 40MHz
        // HT Capabilities: bit 1 of byte 0 = 40MHz support
        // 0x02 = 0000 0010, bit 1 is set (40MHz capable)

        // Build proper IE data: ID(45) + Length(26) + 26 bytes of data
        let mut ie_data = vec![45, 26];  // IE header
        ie_data.push(0x02);  // First byte with bit 1 set (40MHz)
        ie_data.extend_from_slice(&[0u8; 25]);  // Remaining 25 bytes

        // Verify data length
        assert_eq!(ie_data.len(), 28, "IE data should be 28 bytes");

        let width = detect_channel_width(&ie_data, 6, Band::Ghz2_4);
        assert_eq!(width, 40);
    }

    #[test]
    fn test_detect_channel_width_vht_80() {
        // IE data with VHT Operation indicating 80MHz
        let ie_data = vec![
            192, 3,  // VHT Operation IE
            1,  // Channel width: 80MHz
            42, 0,  // Center segments
        ];

        let width = detect_channel_width(&ie_data, 36, Band::Ghz5);
        assert_eq!(width, 80);
    }

    #[test]
    fn test_detect_channel_width_vht_160() {
        // IE data with VHT Operation indicating 160MHz
        let ie_data = vec![
            192, 3,  // VHT Operation IE
            2,  // Channel width: 160MHz
            114, 0,  // Center segments
        ];

        let width = detect_channel_width(&ie_data, 100, Band::Ghz5);
        assert_eq!(width, 160);
    }

    #[test]
    fn test_detect_channel_width_default() {
        // Empty IE data should return 20MHz
        let ie_data = vec![];
        let width = detect_channel_width(&ie_data, 6, Band::Ghz2_4);
        assert_eq!(width, 20);
    }

    // -------------------------------------------------------------------------
    // parse_capabilities Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_capabilities_empty() {
        let ie_data = vec![];
        let (standards, features, _, security, _, _, _, _, _) = parse_capabilities(&ie_data);

        assert_eq!(standards, vec!["g"]);
        assert_eq!(security, "open");
        assert_eq!(features.spatial_streams, 0);
    }

    #[test]
    fn test_parse_capabilities_ht() {
        // SSID IE + HT Capabilities IE
        let mut ie_data = vec![
            0, 4, b'T', b'e', b's', b't',  // SSID
        ];
        ie_data.push(45);  // HT Capabilities
        ie_data.push(26);
        ie_data.extend_from_slice(&[0x6C, 0x01]);  // 40MHz
        ie_data.extend_from_slice(&[0u8; 24]);  // Rest of HT caps

        let (standards, _features, _, _, _, _, _, _, _) = parse_capabilities(&ie_data);

        assert!(standards.contains(&"n".to_string()));
    }

    #[test]
    fn test_parse_capabilities_vht() {
        // VHT Capabilities IE
        let mut ie_data = vec![
            191, 12,  // VHT Capabilities
        ];
        ie_data.extend_from_slice(&[0u8; 12]);

        let (standards, features, _, _, _, _, _, _, _) = parse_capabilities(&ie_data);

        assert!(standards.contains(&"ac".to_string()));
        assert_eq!(features.max_qam, 256);
    }

    #[test]
    fn test_parse_capabilities_he() {
        // Extended HE Capabilities IE - need proper format
        // Format: 255 (ext ID) + length + ext_id + data
        let mut ie_data = vec![
            255, 22,  // Extended IE: ID=255, Length=22
            35,  // HE Capabilities extension ID
        ];
        ie_data.extend_from_slice(&[0u8; 21]);  // 21 bytes of HE data

        let (standards, features, _, _, _, _, _, _, _) = parse_capabilities(&ie_data);

        assert!(standards.contains(&"ax".to_string()));
        assert_eq!(features.max_qam, 1024);
    }

    #[test]
    fn test_parse_capabilities_eht() {
        // Extended EHT Capabilities IE
        let mut ie_data = vec![
            255, 14,  // Extended IE
            108,  // EHT Capabilities extension ID
        ];
        ie_data.extend_from_slice(&[0u8; 13]);

        let (standards, features, _, _, _, _, _, _, _) = parse_capabilities(&ie_data);

        assert!(standards.contains(&"be".to_string()));
        assert_eq!(features.max_qam, 4096);
    }

    // -------------------------------------------------------------------------
    // Extended Capabilities Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_extended_capabilities_detailed() {
        // Extended Capabilities IE with various bits set
        let ext_caps = vec![
            0x00,  // Byte 0
            0x10,  // Byte 1: Bit 4 = RRM
            0x08,  // Byte 2: Bit 3 = BSS Transition
            0x10,  // Byte 3: Bit 4 = FT over DS
        ];

        let mut protocols = ProtocolExtensions::default();
        parse_extended_capabilities_detailed(&ext_caps, &mut protocols);

        assert!(protocols.rrm);
        assert!(protocols.bss_transition);
        assert!(protocols.ft_over_ds);
        assert!(protocols.ft);
    }

    // -------------------------------------------------------------------------
    // parse_all_ies Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_all_ies() {
        // SSID IE
        let ie_data = vec![
            0, 4, b'T', b'e', b's', b't',  // SSID = "Test"
        ];

        let details = parse_all_ies(&ie_data);

        assert_eq!(details.total_length, 6);
        assert_eq!(details.elements.len(), 1);
        assert_eq!(details.elements[0].name, "SSID");
    }

    // -------------------------------------------------------------------------
    // HT Operation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_ht_operation_40mhz() {
        // HT Operation with secondary channel above
        let ht_op = vec![
            6,    // Primary channel
            0x01, // Secondary channel above
            0x00, // Other info
        ];

        let channel_info = parse_ht_operation(&ht_op);

        assert!(channel_info.is_some());
        let info = channel_info.unwrap();
        assert_eq!(info.primary, 6);
        assert_eq!(info.bandwidth, ChannelBandwidth::MHz40);
        assert!(info.secondary_offset.is_some());
    }

    #[test]
    fn test_parse_ht_operation_20mhz() {
        // HT Operation with no secondary channel - need at least 3 bytes
        let ht_op = vec![
            6,    // Primary channel
            0x00, // No secondary
            0x00, // Need third byte for length check
        ];

        let channel_info = parse_ht_operation(&ht_op);

        // parse_ht_operation requires len >= 3, returns 20MHz bandwidth
        assert!(channel_info.is_some());
        let info = channel_info.unwrap();
        assert_eq!(info.bandwidth, ChannelBandwidth::MHz20);
    }

    // -------------------------------------------------------------------------
    // VHT Operation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_vht_operation_80mhz() {
        let vht_op = vec![
            1,    // 80MHz
            42,   // Center freq segment 0
            0,    // Center freq segment 1
        ];

        let channel_info = parse_vht_operation(&vht_op);

        assert!(channel_info.is_some());
        let info = channel_info.unwrap();
        assert_eq!(info.bandwidth, ChannelBandwidth::MHz80);
        assert_eq!(info.center_freq_0, Some(42));
    }

    #[test]
    fn test_parse_vht_operation_160mhz() {
        let vht_op = vec![
            2,    // 160MHz
            114,  // Center freq segment 0
            0,
        ];

        let channel_info = parse_vht_operation(&vht_op);

        assert!(channel_info.is_some());
        let info = channel_info.unwrap();
        assert_eq!(info.bandwidth, ChannelBandwidth::MHz160);
    }

    // -------------------------------------------------------------------------
    // EHT Operation Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_eht_operation_320mhz() {
        let eht_op = vec![
            0x00,
            0x10,  // Width bits for 320MHz (bits 2-4 = 4)
            0x00,
            0x00,
        ];

        let channel_info = parse_eht_operation(&eht_op);

        assert!(channel_info.is_some());
        let info = channel_info.unwrap();
        assert_eq!(info.bandwidth, ChannelBandwidth::MHz320);
    }

    // -------------------------------------------------------------------------
    // MCS Map Stream Counting Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_count_supported_streams_from_mcs_map() {
        // MCS map with 2 streams supported
        let mcs_map: u16 = 0x0000;  // All supported up to 2 streams
        let count = count_supported_streams_from_mcs_map(mcs_map);
        assert_eq!(count, 8);  // All 8 streams supported

        // MCS map with only 1 stream
        let mcs_map_1ss: u16 = 0xFFFC;  // Only stream 1 supported (bits 0-1 = 0, rest = 3)
        let count_1ss = count_supported_streams_from_mcs_map(mcs_map_1ss);
        assert_eq!(count_1ss, 1);
    }

    // -------------------------------------------------------------------------
    // Channel Info Integration Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_channel_info_with_secondary() {
        let info = ChannelInfo {
            primary: 6,
            bandwidth: ChannelBandwidth::MHz40,
            secondary: Some(10),
            secondary_offset: Some(SecondaryChannelOffset::Above),
            center_freq_0: None,
            center_freq_1: None,
            frequency: Some(2437),
        };

        assert_eq!(info.primary, 6);
        assert_eq!(info.bandwidth.as_mhz(), 40);
        assert_eq!(info.secondary, Some(10));
    }
}
