use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::Emitter;

// ============ Types ============

/// IE 解析结果
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

/// WiFi 标准检测摘要
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

/// 完整的 IE 解析数据
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IEDetails {
    pub raw_hex: String,
    pub total_length: usize,
    pub elements: Vec<ParsedIE>,
    pub detection_summary: DetectionSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolExtensions {
    /// 802.11k - Radio Resource Measurement
    pub rrm: bool,
    /// 802.11v - BSS Transition Management
    pub bss_transition: bool,
    /// 802.11r - Fast BSS Transition
    pub ft: bool,
    /// 802.11w - Protected Management Frames
    pub pmf: bool,
    /// 802.11e - WMM/QoS
    pub wmm: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceFeatures {
    /// Multi-User MIMO
    pub mu_mimo: bool,
    /// OFDMA
    pub ofdma: bool,
    /// BSS Coloring (WiFi 6)
    pub bss_coloring: bool,
    /// Target Wake Time
    pub twt: bool,
    /// Spatial streams
    pub spatial_streams: u8,
    /// Max data rate Mbps
    pub max_data_rate: u32,
    /// TX Beamforming
    pub tx_beamforming: bool,
    /// A-MPDU length exponent
    pub ampdu_length: u8,
    /// Multi-Link Operation (WiFi 7 MLO)
    pub mlo: bool,
    /// Max QAM modulation: 256, 1024, or 4096
    pub max_qam: u16,
}

/// BSS Load Information (802.11k)
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BssLoad {
    /// Channel utilization (0-255, scaled to percentage)
    pub channel_utilization: u8,
    /// Number of connected stations
    pub station_count: u16,
    /// Available admission capacity
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Network {
    // Basic
    pub ssid: Option<String>,
    pub bssid: String,
    pub signal: i16,
    pub noise: i16,
    pub snr: u16,
    pub channel: u16,
    pub frequency: u32,
    pub band: String,
    pub connected: bool,

    // WiFi Standard & Performance
    pub standards: Vec<String>,
    pub channel_width: u16,
    pub center_channel: Option<u16>,
    pub secondary_channel: Option<u16>,
    pub features: PerformanceFeatures,

    // Security
    pub security: String,
    pub security_details: SecurityDetails,

    // Protocol Extensions
    pub protocols: ProtocolExtensions,

    // BSS Load (802.11k)
    pub bss_load: Option<BssLoad>,

    // Network Identification
    pub is_hidden: bool,
    pub network_group_id: Option<String>,
    pub vendor: String,
    pub country_code: Option<String>,

    // Additional Info
    pub supported_rates: Vec<u32>,
    pub wps_enabled: bool,
    pub ap_mode: u16,
    pub capabilities: u16,

    // Timing
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

// ============ Platform-specific scanning ============

#[cfg(target_os = "macos")]
mod scanner {
    use super::*;
    use plist::Value;
    use std::io::Cursor;
    use std::process::Command;

    pub fn scan() -> Result<Vec<Network>, String> {
        let output = Command::new("/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport")
            .args(["-s", "-x"])
            .output()
            .map_err(|e| format!("Failed to run airport: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_airport_xml(&stdout)
    }

    fn parse_airport_xml(xml: &str) -> Result<Vec<Network>, String> {
        let plist = Value::from_reader(Cursor::new(xml.as_bytes()))
            .map_err(|e| format!("Failed to parse plist: {}", e))?;

        let networks_array = plist.as_array()
            .ok_or("Invalid plist format: expected array")?;

        let mut networks = Vec::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for network_dict in networks_array {
            if let Some(dict) = network_dict.as_dictionary() {
                if let Ok(net) = parse_network_dict(dict, now) {
                    networks.push(net);
                }
            }
        }

        Ok(networks)
    }

    fn parse_network_dict(dict: &plist::Dictionary, now: u64) -> Result<Network, String> {
        // 基础信息
        let ssid = dict.get("SSID_STR")
            .and_then(|v| v.as_string())
            .map(|s| s.to_string());

        let bssid = dict.get("BSSID")
            .and_then(|v| v.as_string())
            .map(|s| s.to_string().to_uppercase())
            .unwrap_or_default();

        let signal = dict.get("RSSI")
            .and_then(|v| v.as_signed_integer())
            .map(|v| v as i16)
            .unwrap_or(-100);

        let noise = dict.get("NOISE")
            .and_then(|v| v.as_signed_integer())
            .map(|v| v as i16)
            .unwrap_or(-100);

        let snr = (signal - noise).max(0) as u16;

        let channel = dict.get("CHANNEL")
            .and_then(|v| v.as_signed_integer())
            .map(|v| v as u16)
            .unwrap_or(0);

        let beacon_interval = dict.get("BEACON_INT")
            .and_then(|v| v.as_signed_integer())
            .map(|v| v as u16)
            .unwrap_or(100);

        let ap_mode = dict.get("AP_MODE")
            .and_then(|v| v.as_signed_integer())
            .map(|v| v as u16)
            .unwrap_or(0);

        let capabilities = dict.get("CAPABILITIES")
            .and_then(|v| v.as_signed_integer())
            .map(|v| v as u16)
            .unwrap_or(0);

        // 信道标志解析
        let channel_flags = dict.get("CHANNEL_FLAGS")
            .and_then(|v| v.as_signed_integer())
            .unwrap_or(0) as u16;

        let (band, frequency, channel_width) = parse_channel_flags(channel, channel_flags);

        // 解析BSS负载
        let bss_load = parse_bss_load(dict);

        // 解析安全信息
        let (security, security_details) = parse_security(dict);

        // 解析 WiFi 标准和性能特性
        let (standards, features) = parse_wifi_capabilities(dict, channel_width, &band);

        // 解析协议扩展 (k/v/r/w)
        let protocols = parse_protocol_extensions(dict);

        // 国家代码
        let country_code = dict.get("80211D_IE")
            .and_then(|v| v.as_dictionary())
            .and_then(|d| d.get("IE_KEY_80211D_COUNTRY_CODE"))
            .and_then(|v| v.as_string())
            .map(|s| s.to_string());

        // 支持的速率
        let supported_rates = dict.get("RATES")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_signed_integer().map(|r| r as u32)).collect())
            .unwrap_or_default();

        // WPS状态
        let wps_enabled = dict.contains_key("WPS_BEACON_IE");

        // 厂商识别
        let vendor = lookup_vendor_quick(&bssid);

        // 先计算is_hidden
        let is_hidden = ssid.is_none();

        Ok(Network {
            ssid,
            bssid: bssid.clone(),
            signal,
            noise,
            snr,
            channel,
            frequency,
            band,
            connected: false,
            standards,
            channel_width,
            center_channel: None,
            secondary_channel: None,
            features,
            security: security.clone(),
            security_details,
            protocols,
            bss_load,
            is_hidden,
            network_group_id: None,
            vendor,
            country_code,
            supported_rates,
            wps_enabled,
            ap_mode,
            capabilities,
            beacon_interval,
            first_seen: now,
            last_seen: now,
        })
    }

    fn parse_bss_load(dict: &plist::Dictionary) -> Option<BssLoad> {
        dict.get("QBSS_LOAD_IE").and_then(|v| v.as_dictionary()).map(|qbss| {
            BssLoad {
                channel_utilization: qbss.get("QBSS_CHAN_UTIL")
                    .and_then(|v| v.as_signed_integer())
                    .map(|v| v as u8)
                    .unwrap_or(0),
                station_count: qbss.get("QBSS_STA_COUNT")
                    .and_then(|v| v.as_signed_integer())
                    .map(|v| v as u16)
                    .unwrap_or(0),
                available_capacity: qbss.get("QBSS_AAC")
                    .and_then(|v| v.as_signed_integer())
                    .map(|v| v as u16)
                    .unwrap_or(0),
            }
        })
    }

    fn parse_channel_flags(channel: u16, flags: u16) -> (String, u32, u16) {
        // 优先使用信道号判断频段（更可靠）
        // 信道 1-14: 2.4GHz
        // 信道 36-165: 5GHz
        let is_5ghz = channel > 14;

        // 信道宽度从 flags 解析
        // Bit 5 (0x20): 40MHz
        // Bit 7 (0x80): 80MHz (VHT)
        let width = if (flags & 0x0080) != 0 {
            80
        } else if (flags & 0x0020) != 0 {
            40
        } else {
            20
        };

        if is_5ghz {
            ("5".to_string(), 5000 + channel as u32 * 5, width)
        } else {
            ("2.4".to_string(), 2407 + channel as u32 * 5, width)
        }
    }

    fn parse_security(dict: &plist::Dictionary) -> (String, SecurityDetails) {
        // 检查 RSN_IE (WPA2/WPA3)
        let rsn_ie = dict.get("RSN_IE").and_then(|v| v.as_dictionary());
        let wpa_ie = dict.get("WPA_IE").and_then(|v| v.as_dictionary());

        if let Some(rsn) = rsn_ie {
            let authsels = rsn.get("IE_KEY_RSN_AUTHSELS")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_signed_integer()).collect::<Vec<_>>())
                .unwrap_or_default();

            let auth_type = authsels.first().unwrap_or(&2);

            // Auth values: 1=802.1X, 2=PSK, 3=FT-PSK, 4=SAE (WPA3)
            let (security_type, auth_method) = match auth_type {
                4 => ("wpa3".to_string(), "sae".to_string()),
                3 => ("wpa2".to_string(), "ft-psk".to_string()),
                1 => ("wpa2-ent".to_string(), "eap".to_string()),
                _ => ("wpa2".to_string(), "psk".to_string()),
            };

            let mcipher = rsn.get("IE_KEY_RSN_MCIPHER")
                .and_then(|v| v.as_signed_integer())
                .unwrap_or(4);

            let cipher = match mcipher {
                2 => "tkip",
                4 => "ccmp",
                8 => "gcmp",
                _ => "unknown"
            };

            // 解析PMF能力
            let rsn_caps = rsn.get("IE_KEY_RSN_CAPS").and_then(|v| v.as_dictionary());
            let pmf_capable = rsn_caps
                .and_then(|c| c.get("MFP_CAPABLE"))
                .and_then(|v| v.as_boolean())
                .unwrap_or(false);
            let pmf_required = rsn_caps
                .and_then(|c| c.get("MFP_REQUIRED"))
                .and_then(|v| v.as_boolean())
                .unwrap_or(false);

            return (security_type.clone(), SecurityDetails {
                security_type,
                auth_method: auth_method.clone(),
                cipher: cipher.to_string(),
                key_mgmt: vec![auth_method],
                is_enterprise: *auth_type == 1,
                is_wpa3_transition: false,
                pmf_required,
                pmf_capable,
            });
        }

        if let Some(_wpa) = wpa_ie {
            return ("wpa".to_string(), SecurityDetails {
                security_type: "wpa".to_string(),
                auth_method: "psk".to_string(),
                cipher: "tkip".to_string(),
                key_mgmt: vec!["psk".to_string()],
                is_enterprise: false,
                is_wpa3_transition: false,
                pmf_required: false,
                pmf_capable: false,
            });
        }

        ("open".to_string(), SecurityDetails {
            security_type: "open".to_string(),
            auth_method: "open".to_string(),
            cipher: "none".to_string(),
            key_mgmt: vec![],
            is_enterprise: false,
            is_wpa3_transition: false,
            pmf_required: false,
            pmf_capable: false,
        })
    }

    fn parse_wifi_capabilities(dict: &plist::Dictionary, channel_width: u16, band: &str) -> (Vec<String>, PerformanceFeatures) {
        let mut standards = Vec::new();
        let mut features = PerformanceFeatures::default();
        let mut has_eht = false;
        let mut eht_streams: u8 = 0;
        let mut has_mlo = false;
        let mut max_qam: u16 = 256; // 默认 256-QAM

        // 从原始 IE 数据解析 EHT (WiFi 7) 信息
        if let Some(ie_data) = dict.get("IE").and_then(|v| v.as_data()) {
            let (found_eht, streams, supports_160, mlo, qam) = parse_eht_from_ie(ie_data);
            has_eht = found_eht;
            eht_streams = streams;
            has_mlo = mlo;
            max_qam = qam;
            if supports_160 {
                // 更新信道宽度为 160MHz（如果检测到支持）
                // 注意：这里不能修改 channel_width 参数，但可以影响速率计算
            }
        }

        if has_eht {
            standards.push("be".to_string()); // WiFi 7 = 802.11be
            features.ofdma = true;
            features.bss_coloring = true;
            features.mu_mimo = true;
            features.mlo = has_mlo;
            features.max_qam = max_qam;
            if eht_streams > 0 {
                features.spatial_streams = eht_streams;
            } else {
                features.spatial_streams = 4; // WiFi 7 通常 4 流起步
            }
        }

        // HE_CAP = 802.11ax (WiFi 6)
        let has_he = dict.contains_key("HE_CAP");
        if has_he && !has_eht {
            standards.push("ax".to_string());
            features.ofdma = true;
            features.bss_coloring = true;
            features.max_qam = 1024; // WiFi 6 支持 1024-QAM

            // 解析 HE_CAP 数据获取更多信息
            if let Some(he_cap_data) = dict.get("HE_CAP").and_then(|v| v.as_data()) {
                // HE Capabilities IE 格式复杂，简化处理
                let he_streams = parse_he_spatial_streams(he_cap_data);
                features.spatial_streams = he_streams;
            } else if features.spatial_streams == 0 {
                features.spatial_streams = 2; // 默认值
            }
        }

        // 如果还没有设置 QAM，根据标准设置
        if features.max_qam == 0 {
            if has_eht {
                features.max_qam = 4096;
            } else if has_he {
                features.max_qam = 1024;
            } else {
                features.max_qam = 256;
            }
        }

        // VHT_CAPS = 802.11ac (WiFi 5)
        let has_vht = dict.contains_key("VHT_CAPS");
        if has_vht {
            if !standards.contains(&"be".to_string()) && !standards.contains(&"ax".to_string()) {
                standards.push("ac".to_string());
            }

            // VHT 支持 MU-MIMO
            if let Some(vht_caps) = dict.get("VHT_CAPS").and_then(|v| v.as_dictionary()) {
                if let Some(_info) = vht_caps.get("INFO").and_then(|v| v.as_signed_integer()) {
                    // VHT Capabilities Info 字段解析
                    // Bit 0-1: Max MPDU Length
                    // Bit 2-3: Supported Channel Width Set
                    // Bit 4: Rx LDPC
                    // Bit 5: Short GI for 80MHz
                    features.mu_mimo = true; // VHT 通常支持 MU-MIMO

                    // 检测空间流数
                    if features.spatial_streams == 0 {
                        // VHT Capabilities 中 NSS 在特定位置
                        // 简化：从 MCS 信息推断
                    }
                }
            }
        }

        // HT_CAPS_IE = 802.11n (WiFi 4)
        let has_ht = dict.contains_key("HT_CAPS_IE");
        if has_ht {
            if !standards.contains(&"be".to_string()) && !standards.contains(&"ax".to_string()) && !standards.contains(&"ac".to_string()) {
                standards.push("n".to_string());
            }

            if let Some(ht_caps) = dict.get("HT_CAPS_IE").and_then(|v| v.as_dictionary()) {
                // 解析 MCS_SET 获取空间流数
                if let Some(mcs_set) = ht_caps.get("MCS_SET").and_then(|v| v.as_data()) {
                    // MCS set 是 16 字节，每 bit 代表一个 MCS
                    let ht_streams = count_mcs_streams(mcs_set);
                    if features.spatial_streams == 0 {
                        features.spatial_streams = ht_streams;
                    }
                }

                // TXBF (波束成形)
                let txbf_caps = ht_caps.get("TXBF_CAPS")
                    .and_then(|v| v.as_signed_integer())
                    .unwrap_or(0);
                features.tx_beamforming = txbf_caps != 0;

                // A-MPDU 参数
                let ampdu_params = ht_caps.get("AMPDU_PARAMS")
                    .and_then(|v| v.as_signed_integer())
                    .unwrap_or(0);
                features.ampdu_length = ((ampdu_params & 0x03) + 1) as u8; // 2^(exponent+1) KB
            }
        }

        // 基础标准
        if standards.is_empty() {
            standards.push("g".to_string());
            standards.push("b".to_string());
        }

        // 默认空间流
        if features.spatial_streams == 0 {
            features.spatial_streams = 2;
        }

        // 估算最大速率
        features.max_data_rate = estimate_max_rate(&standards, channel_width, band, features.spatial_streams);

        (standards, features)
    }

    /// 从原始 IE 数据解析 EHT (WiFi 7) 信息
    /// EHT IE 使用 Element ID 0xFF (Extended)
    /// - Extension ID 108 (0x6c): EHT Capabilities
    /// - Extension ID 106 (0x6a): EHT Operation
    /// - Extension ID 107: EHT Multi-Link (MLO)
    /// 注意：HE (WiFi 6) 使用 Extension ID 35/36，与 EHT 不同！
    fn parse_eht_from_ie(ie_data: &[u8]) -> (bool, u8, bool, bool, u16) {
        let mut has_eht = false;
        let mut spatial_streams = 0u8;
        let mut supports_160 = false;
        let mut has_mlo = false;
        let mut max_qam = 1024u16; // WiFi 6 默认支持 1024-QAM

        let mut pos = 0;
        while pos + 1 < ie_data.len() {
            let element_id = ie_data[pos];
            let length = ie_data[pos + 1] as usize;

            if pos + 2 + length > ie_data.len() {
                break;
            }

            // Element ID 0xFF (255) = Extended Element
            if element_id == 0xFF && length >= 1 {
                let extension_id = ie_data[pos + 2];

                // EHT Capabilities = Extension ID 108 (0x6c)
                // 注意：Ext ID 35 是 HE Capabilities (WiFi 6)，不是 EHT！
                if extension_id == 108 {
                    has_eht = true;

                    if length >= 5 {
                        // EHT MAC Capabilities (bytes 3-4)
                        // EHT PHY Capabilities (bytes 5-8)
                        supports_160 = true;
                        spatial_streams = 4; // WiFi 7 通常 4 流
                    }

                    // 检测 4096-QAM 支持
                    // EHT PHY Capabilities 在 byte 5-8
                    if length >= 8 {
                        // EHT PHY Capabilities Info 第一个字节在 data[3]
                        // 320MHz support 在 bits 0-1
                        // Rx 1024-QAM and 4096-QAM 在后续位
                        let phy_cap_2 = u32::from_le_bytes([
                            ie_data[pos + 5],
                            ie_data[pos + 6],
                            ie_data[pos + 7],
                            ie_data[pos + 8],
                        ]);
                        // 检查 Rx 4096-QAM 支持位 (bit 16-19)
                        if phy_cap_2 & 0x000F0000 != 0 {
                            max_qam = 4096;
                        }
                    }

                    // 更详细的空间流解析可以从 EHT MCS Set 获取
                    if length >= 8 {
                        // 解析 MCS Set（复杂格式，简化处理）
                    }
                }

                // EHT Operation = Extension ID 106 (0x6a)
                if extension_id == 106 {
                    // 包含操作信道、信道宽度等信息
                }

                // EHT Multi-Link = Extension ID 107 (MLO)
                if extension_id == 107 {
                    has_mlo = true;
                }

                // HE Capabilities = Extension ID 35
                // 检测 WiFi 6 的 1024-QAM 支持
                if extension_id == 35 && length >= 7 {
                    // HE PHY Capabilities
                    let phy_cap = u32::from_le_bytes([
                        ie_data[pos + 5],
                        ie_data[pos + 6],
                        ie_data[pos + 7],
                        if length >= 8 { ie_data[pos + 8] } else { 0 },
                    ]);
                    // HE 1024-QAM 在 HE PHY Capabilities 的特定位
                    // Bit 16-18: HE MCS and NSS for <= 80MHz
                    // 简化：假设 WiFi 6 支持 1024-QAM
                    if max_qam < 1024 {
                        max_qam = 1024;
                    }
                }
            }

            pos += 2 + length;
        }

        (has_eht, spatial_streams, supports_160, has_mlo, max_qam)
    }

    /// 从 HE Capabilities IE 解析空间流数
    fn parse_he_spatial_streams(he_data: &[u8]) -> u8 {
        // HE Capabilities IE 格式:
        // Element ID: 0xFF (Extended)
        // Extension Element ID: 35 (HE Capabilities - 注：WiFi 6 也用这个!)
        // 但 airport 解析后的 HE_CAP 格式不同

        // HE MAC Capabilities Info (2 bytes)
        // HE PHY Capabilities Info (variable, typically 3+ bytes)
        // ...
        // HE MCS Set

        if he_data.len() < 8 {
            return 2; // 默认
        }

        // 简化处理：检查 HE PHY Capabilities
        // HE PHY Capabilities 从 byte 2 开始（假设 airport 格式）
        // 检查 NSS 相关位

        // 对于 802.11ax，MCS Set 在 IE 后部
        // 格式复杂，简化返回 2-4 流
        if he_data.len() >= 16 {
            // 检查是否有更多空间流的指示
            let mcs_byte = he_data[12]; // 假设位置
            if mcs_byte != 0 {
                // 有有效 MCS 数据
                if mcs_byte == 0xFF {
                    return 4;
                } else if mcs_byte >= 0x0F {
                    return 3;
                }
            }
        }

        2
    }

    fn count_mcs_streams(mcs_data: &[u8]) -> u8 {
        // 简化：检查前几个字节中 1 的数量
        if mcs_data.is_empty() { return 2; }

        let first_byte = mcs_data[0];
        if first_byte == 0xFF { return 4; }
        if first_byte == 0x0F { return 2; }
        if first_byte == 0x03 { return 1; }
        2
    }

    fn estimate_max_rate(standards: &[String], width: u16, band: &str, streams: u8) -> u32 {
        let is_5ghz = band == "5";

        if standards.contains(&"be".to_string()) {
            // WiFi 7 (802.11be):
            // 支持 320MHz, 160MHz, 80MHz 等
            // 5GHz: 最高 5764 Mbps @ 160MHz per stream (理论值)
            // 实际速率取决于调制 (4096-QAM)
            if is_5ghz {
                match width {
                    w if w >= 160 => 2882 * streams as u32, // 保守估计
                    w if w >= 80 => 1441 * streams as u32,
                    w if w >= 40 => 720 * streams as u32,
                    _ => 360 * streams as u32,
                }
            } else {
                // 2.4GHz WiFi 7
                match width {
                    w if w >= 40 => 574 * streams as u32,
                    _ => 287 * streams as u32,
                }
            }
        } else if standards.contains(&"ax".to_string()) {
            // WiFi 6:
            // 2.4GHz: 574 Mbps @ 20MHz, 1148 Mbps @ 40MHz per stream
            // 5GHz: 600 Mbps @ 20MHz, 1200 Mbps @ 40MHz, 2401 Mbps @ 80MHz per stream
            if is_5ghz {
                match width {
                    w if w >= 160 => 2401 * streams as u32,
                    w if w >= 80 => 1200 * streams as u32,
                    w if w >= 40 => 600 * streams as u32,
                    _ => 287 * streams as u32,
                }
            } else {
                // 2.4GHz WiFi 6
                match width {
                    w if w >= 40 => 574 * streams as u32,
                    _ => 287 * streams as u32,
                }
            }
        } else if standards.contains(&"ac".to_string()) {
            // WiFi 5 (5GHz only):
            // 433 Mbps @ 80MHz, 867 Mbps @ 160MHz per stream
            match width {
                w if w >= 160 => 867 * streams as u32,
                w if w >= 80 => 433 * streams as u32,
                w if w >= 40 => 200 * streams as u32,
                _ => 54,
            }
        } else if standards.contains(&"n".to_string()) {
            // WiFi 4:
            // 2.4GHz: 72 Mbps @ 20MHz, 150 Mbps @ 40MHz per stream
            // 5GHz: similar
            if is_5ghz {
                match width {
                    w if w >= 40 => 150 * streams as u32,
                    _ => 72 * streams as u32,
                }
            } else {
                match width {
                    w if w >= 40 => 150 * streams as u32,
                    _ => 72 * streams as u32,
                }
            }
        } else {
            // 802.11g/b
            54
        }
    }

    fn parse_protocol_extensions(dict: &plist::Dictionary) -> ProtocolExtensions {
        let mut protocols = ProtocolExtensions::default();
        protocols.wmm = true; // 几乎总是支持

        // EXT_CAPS 包含扩展能力
        if let Some(ext_caps) = dict.get("EXT_CAPS").and_then(|v| v.as_dictionary()) {
            // BSS_TRANS_MGMT = 802.11v
            protocols.bss_transition = ext_caps.get("BSS_TRANS_MGMT")
                .and_then(|v| v.as_signed_integer())
                .map(|v| v == 1)
                .unwrap_or(false);
        }

        // 从原始 IE 数据解析 Extended Capabilities
        // Extended Capabilities IE (ID 127) 包含 RRM 等信息
        if let Some(ie_data) = dict.get("IE").and_then(|v| v.as_data()) {
            // 解析 IE 数据查找 Extended Capabilities (ID 127)
            parse_ext_caps_from_ie(ie_data, &mut protocols);
        }

        // 从 RSN_IE 检查 802.11w (PMF)
        if let Some(rsn) = dict.get("RSN_IE").and_then(|v| v.as_dictionary()) {
            // 检查 RSN Capabilities 中的 PMF 位
            if let Some(rsn_caps) = rsn.get("IE_KEY_RSN_CAPS").and_then(|v| v.as_dictionary()) {
                protocols.pmf = rsn_caps.get("MFP_CAPABLE")
                    .and_then(|v| v.as_boolean())
                    .unwrap_or(false);
            }

            // WPA3 默认支持 PMF
            let authsels = rsn.get("IE_KEY_RSN_AUTHSELS")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_signed_integer()).collect::<Vec<_>>())
                .unwrap_or_default();

            if authsels.contains(&4) { // SAE = WPA3
                protocols.pmf = true;
            }
        }

        // 802.11r (FT) 检查
        // FT 通常在 auth type 3 或通过 MDIE
        if let Some(rsn) = dict.get("RSN_IE").and_then(|v| v.as_dictionary()) {
            protocols.ft = rsn.get("IE_KEY_RSN_AUTHSELS")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().any(|v| v.as_signed_integer() == Some(3)))
                .unwrap_or(false);

            // 也检查 MDIE 的存在 (Mobility Domain IE = FT)
            if dict.contains_key("MDIE") {
                protocols.ft = true;
            }
        }

        protocols
    }

    /// 从原始 IE 数据解析 Extended Capabilities
    fn parse_ext_caps_from_ie(ie_data: &[u8], protocols: &mut ProtocolExtensions) {
        // IE 格式: [Element ID (1 byte)][Length (1 byte)][Data...]
        // Extended Capabilities IE ID = 127 (0x7F)
        let mut pos = 0;
        while pos + 1 < ie_data.len() {
            let element_id = ie_data[pos];
            let length = ie_data[pos + 1] as usize;

            if pos + 2 + length > ie_data.len() {
                break;
            }

            if element_id == 127 {
                // Extended Capabilities IE found
                let ext_caps_data = &ie_data[pos + 2..pos + 2 + length];

                // Bit 12: Radio Measurement (802.11k RRM)
                if ext_caps_data.len() > 1 {
                    // Byte 1 (bits 8-15): bit 12 is in byte 1, bit 4
                    protocols.rrm = (ext_caps_data[1] & 0x10) != 0;
                }

                // Bit 19: BSS Transition (802.11v) - 在 byte 2, bit 3
                if ext_caps_data.len() > 2 {
                    protocols.bss_transition = protocols.bss_transition || (ext_caps_data[2] & 0x08) != 0;
                }

                // Bit 14: FT (Fast BSS Transition) capability
                if ext_caps_data.len() > 1 {
                    protocols.ft = protocols.ft || (ext_caps_data[1] & 0x40) != 0;
                }

                break;
            }

            pos += 2 + length;
        }
    }

    fn lookup_vendor_quick(bssid: &str) -> String {
        let oui = bssid.replace(":", "").replace("-", "").to_uppercase();
        if oui.len() < 6 { return "Unknown".to_string(); }

        let prefix = &oui[0..6];
        let vendors = [
            ("001A2B", "TP-Link"), ("001E58", "ASUSTek"), ("00226B", "Cisco"),
            ("00246C", "Apple"), ("005056", "VMware"), ("04D4C4", "Apple"),
            ("086698", "Apple"), ("0C4DE9", "Apple"), ("10E341", "Huawei"),
            ("18A6F7", "Xiaomi"), ("2034FB", "Apple"), ("240A64", "Xiaomi"),
            ("30074D", "Apple"), ("3423BA", "Apple"), ("38F9D3", "Apple"),
            ("44D884", "Apple"), ("483B38", "Apple"), ("5C5948", "Intel"),
            ("68DBCA", "Apple"), ("6C5C14", "TP-Link"), ("784F43", "Apple"),
            ("7C6D62", "Apple"), ("7CD1C3", "Intel"), ("80EAD2", "Ubiquiti"),
            ("849FAD", "Apple"), ("90B0ED", "Xiaomi"), ("94BF2D", "Cisco"),
            ("94F6A3", "Apple"), ("9C2AA4", "Ubiquiti"), ("9CF48E", "Apple"),
            ("A01828", "Ubiquiti"), ("A4B197", "TP-Link"), ("ACF7F3", "Apple"),
            ("B06EBF", "Ubiquiti"), ("B827EB", "Raspberry Pi"), ("BC52B7", "Apple"),
            ("C069CD", "Apple"), ("D461DA", "Apple"), ("D81C79", "Apple"),
            ("F4D884", "Apple"), ("68DDB7", "Xiaomi"), ("089AC7", "Xiaomi"),
        ];

        for (oui_prefix, vendor) in vendors {
            if prefix == oui_prefix {
                return vendor.to_string();
            }
        }
        "Unknown".to_string()
    }

    pub fn current() -> Result<Option<Network>, String> {
        let output = Command::new("/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport")
            .args(["-I"])
            .output()
            .map_err(|e| format!("Failed to get current network: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.contains("AirPort: Off") || stdout.is_empty() {
            return Ok(None);
        }

        let mut ssid = None;
        let mut bssid = String::new();
        let mut signal: i16 = -100;
        let mut channel: u16 = 0;
        let mut channel_width: u16 = 20;
        let mut nss: u8 = 1;
        let mut _mcs: u8 = 0;

        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("SSID:") {
                ssid = Some(line[5..].trim().to_string());
            } else if line.starts_with("BSSID:") {
                bssid = line[6..].trim().to_uppercase();
            } else if line.starts_with("agrCtlRSSI:") {
                signal = line[11..].trim().parse().unwrap_or(-100);
            } else if line.starts_with("channel:") {
                let ch_str = line[8..].trim();
                if ch_str.contains(',') {
                    let parts: Vec<&str> = ch_str.split(',').collect();
                    channel = parts[0].parse().unwrap_or(0);
                    channel_width = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(40);
                } else {
                    channel = ch_str.parse().unwrap_or(0);
                }
            } else if line.starts_with("NSS:") {
                nss = line[4..].trim().parse().unwrap_or(1);
            } else if line.starts_with("MCS:") {
                _mcs = line[4..].trim().parse().unwrap_or(0);
            }
        }

        if ssid.is_none() {
            return Ok(None);
        }

        let (band, frequency) = if channel <= 14 {
            ("2.4".to_string(), 2407 + channel as u32 * 5)
        } else {
            ("5".to_string(), 5000 + channel as u32 * 5)
        };

        let vendor = lookup_vendor_quick(&bssid);

        // 根据信道宽度推断标准
        let standards = if channel_width >= 80 {
            vec!["ax".to_string(), "ac".to_string(), "n".to_string()]
        } else if channel_width == 40 {
            vec!["n".to_string()]
        } else {
            vec!["n".to_string(), "g".to_string()]
        };

        Ok(Some(Network {
            ssid: ssid.clone(),
            bssid,
            signal,
            noise: -90,
            snr: (signal + 90).max(0) as u16,
            channel,
            frequency,
            band,
            connected: true,
            standards,
            channel_width,
            center_channel: None,
            secondary_channel: None,
            features: PerformanceFeatures {
                mu_mimo: true,
                ofdma: channel_width >= 80,
                bss_coloring: channel_width >= 80,
                twt: false,
                spatial_streams: nss,
                max_data_rate: 1200 * nss as u32,
                tx_beamforming: true,
                ampdu_length: 4,
                mlo: false,
                max_qam: 1024,
            },
            security: "wpa2".to_string(),
            security_details: SecurityDetails {
                security_type: "wpa2".to_string(),
                auth_method: "psk".to_string(),
                cipher: "ccmp".to_string(),
                key_mgmt: vec!["psk".to_string()],
                is_enterprise: false,
                is_wpa3_transition: false,
                pmf_required: false,
                pmf_capable: false,
            },
            protocols: ProtocolExtensions {
                rrm: false,
                bss_transition: false,
                ft: false,
                pmf: false,
                wmm: true,
            },
            bss_load: None,
            is_hidden: ssid.is_none(),
            network_group_id: None,
            vendor,
            country_code: None,
            supported_rates: vec![],
            wps_enabled: false,
            ap_mode: 0,
            capabilities: 0,
            beacon_interval: 100,
            first_seen: 0,
            last_seen: 0,
        }))
    }

    pub fn get_groups(networks: &[Network]) -> Vec<NetworkGroup> {
        let mut groups: HashMap<String, NetworkGroup> = HashMap::new();

        for net in networks {
            let key = net.ssid.clone().unwrap_or_else(|| "[Hidden]".to_string());

            let group = groups.entry(key.clone()).or_insert(NetworkGroup {
                ssid: key,
                networks: Vec::new(),
                total_aps: 0,
                bands: Vec::new(),
                best_signal: -100,
                supports_fast_roaming: false,
                supports_bss_transition: false,
            });

            group.networks.push(net.clone());
            group.total_aps += 1;

            if !group.bands.contains(&net.band) {
                group.bands.push(net.band.clone());
            }

            if net.signal > group.best_signal {
                group.best_signal = net.signal;
            }

            if net.protocols.ft { group.supports_fast_roaming = true; }
            if net.protocols.bss_transition { group.supports_bss_transition = true; }
        }

        groups.into_values().collect()
    }

    pub fn get_stats(networks: &[Network], duration_ms: u64) -> ScanStats {
        let mut stats = ScanStats {
            total_networks: networks.len() as u32,
            hidden_networks: 0,
            network_groups: 0,
            by_band: HashMap::new(),
            by_security: HashMap::new(),
            by_standard: HashMap::new(),
            scan_duration_ms: duration_ms,
        };

        let mut seen_ssids: std::collections::HashSet<String> = std::collections::HashSet::new();

        for net in networks {
            if net.is_hidden { stats.hidden_networks += 1; }
            if let Some(ref ssid) = net.ssid { seen_ssids.insert(ssid.clone()); }

            *stats.by_band.entry(net.band.clone()).or_insert(0) += 1;
            *stats.by_security.entry(net.security.clone()).or_insert(0) += 1;

            for std in &net.standards {
                *stats.by_standard.entry(std.clone()).or_insert(0) += 1;
            }
        }

        stats.network_groups = seen_ssids.len() as u32;
        stats
    }
}

#[cfg(target_os = "windows")]
mod scanner {
    use super::*;

    pub fn scan() -> Result<Vec<Network>, String> {
        Err("Windows scanning not yet implemented".to_string())
    }

    pub fn current() -> Result<Option<Network>, String> {
        Err("Windows scanning not yet implemented".to_string())
    }

    pub fn get_groups(_networks: &[Network]) -> Vec<NetworkGroup> {
        Vec::new()
    }

    pub fn get_stats(_networks: &[Network], _duration_ms: u64) -> ScanStats {
        ScanStats::default()
    }
}

#[cfg(target_os = "linux")]
mod scanner {
    use super::*;

    pub fn scan() -> Result<Vec<Network>, String> {
        Err("Linux scanning not yet implemented".to_string())
    }

    pub fn current() -> Result<Option<Network>, String> {
        Err("Linux scanning not yet implemented".to_string())
    }

    pub fn get_groups(_networks: &[Network]) -> Vec<NetworkGroup> {
        Vec::new()
    }

    pub fn get_stats(_networks: &[Network], _duration_ms: u64) -> ScanStats {
        ScanStats::default()
    }
}

// ============ Monitoring ============

static MONITORING: AtomicBool = AtomicBool::new(false);

// ============ IE Parsing ============

/// IE 名称映射
fn get_ie_name(element_id: u8) -> &'static str {
    match element_id {
        0 => "SSID",
        1 => "Supported Rates",
        2 => "FH Parameter Set",
        3 => "DS Parameter Set",
        4 => "CF Parameter Set",
        5 => "TIM",
        6 => "IBSS Parameter Set",
        7 => "Country",
        11 => "QBSS Load",
        17 => "Power Constraint",
        42 => "ERP Information",
        45 => "HT Capabilities",
        48 => "RSN",
        50 => "Extended Supported Rates",
        51 => "AP Channel Report",
        61 => "HT Operation",
        70 => "RM Enabled Capabilities",
        74 => "Overlapping BSS Scan Parameters",
        127 => "Extended Capabilities",
        191 => "VHT Capabilities",
        192 => "VHT Operation",
        195 => "Transmit Power Envelope",
        221 => "Vendor Specific",
        255 => "Extended Element",
        _ => "Unknown",
    }
}

/// Extended IE 名称映射
fn get_ext_ie_name(extension_id: u8) -> &'static str {
    match extension_id {
        35 => "HE Capabilities (WiFi 6)",
        36 => "HE Operation (WiFi 6)",
        37 => "MU EDCA Parameter Set",
        38 => "Multi-BSSID",
        39 => "Non-Inheritance",
        106 => "EHT Operation (WiFi 7)",
        107 => "EHT Multi-Link",
        108 => "EHT Capabilities (WiFi 7)",
        _ => "Unknown Extension",
    }
}

/// 解析特定 IE 的详细内容
fn parse_ie_content(element_id: u8, data: &[u8]) -> HashMap<String, serde_json::Value> {
    let mut result = HashMap::new();

    match element_id {
        0 => {  // SSID
            if let Ok(ssid) = std::str::from_utf8(data) {
                result.insert("ssid".to_string(), serde_json::Value::String(ssid.to_string()));
                result.insert("ssid_hex".to_string(), serde_json::Value::String(hex::encode(data)));
            }
        }

        1 => {  // Supported Rates
            let rates: Vec<String> = data.iter().map(|b| {
                let rate = (b & 0x7F) as f32 * 0.5;
                let mandatory = if b & 0x80 != 0 { " (B)" } else { "" };
                format!("{}{}", rate, mandatory)
            }).collect();
            result.insert("rates_mbps".to_string(), serde_json::Value::String(rates.join(", ")));
        }

        3 => {  // DS Parameter Set
            if !data.is_empty() {
                let ch = data[0];
                result.insert("channel".to_string(), serde_json::Value::Number(ch.into()));
                let band = if ch <= 14 { "2.4 GHz" } else { "5 GHz" };
                result.insert("band".to_string(), serde_json::Value::String(band.to_string()));
            }
        }

        7 => {  // Country
            if data.len() >= 3 {
                if let Ok(country) = std::str::from_utf8(&data[0..2]) {
                    result.insert("country_code".to_string(), serde_json::Value::String(country.to_string()));
                }
                let env = match data[2] {
                    b' ' => "All",
                    b'I' => "Indoor",
                    b'O' => "Outdoor",
                    b'X' => "Non-countries",
                    _ => "Unknown",
                };
                result.insert("environment".to_string(), serde_json::Value::String(env.to_string()));

                // 解析信道列表
                if data.len() > 3 {
                    let mut channels = Vec::new();
                    let mut i = 3;
                    while i + 2 < data.len() {
                        let first_ch = data[i];
                        let num_ch = data[i + 1];
                        let max_power = data[i + 2];
                        channels.push(format!("CH {}-{} ({} dBm)", first_ch, first_ch + num_ch - 1, max_power));
                        i += 3;
                    }
                    if !channels.is_empty() {
                        result.insert("channel_list".to_string(), serde_json::Value::String(channels.join("; ")));
                    }
                }
            }
        }

        11 => {  // QBSS Load
            if data.len() >= 5 {
                let station_count = u16::from_le_bytes([data[0], data[1]]);
                let utilization = data[2];
                let capacity = u16::from_le_bytes([data[3], data[4]]);
                result.insert("station_count".to_string(), serde_json::Value::Number(station_count.into()));
                result.insert("channel_utilization".to_string(), serde_json::Value::Number(utilization.into()));
                result.insert("utilization_percent".to_string(), serde_json::Value::Number((((utilization as f32 / 255.0) * 100.0) as u64).into()));
                result.insert("available_capacity".to_string(), serde_json::Value::Number(capacity.into()));
            }
        }

        45 => {  // HT Capabilities
            if data.len() >= 26 {
                let caps = u16::from_le_bytes([data[0], data[1]]);
                result.insert("ldpc_coding".to_string(), serde_json::Value::Bool(caps & 0x0001 != 0));
                result.insert("supported_channel_width_40mhz".to_string(), serde_json::Value::Bool(caps & 0x0002 != 0));
                result.insert("sm_power_save".to_string(), serde_json::Value::String(format!("{}", (caps >> 2) & 0x3)));
                result.insert("greenfield".to_string(), serde_json::Value::Bool(caps & 0x0010 != 0));
                result.insert("short_gi_20mhz".to_string(), serde_json::Value::Bool(caps & 0x0020 != 0));
                result.insert("short_gi_40mhz".to_string(), serde_json::Value::Bool(caps & 0x0040 != 0));
                result.insert("tx_stbc".to_string(), serde_json::Value::Bool(caps & 0x0080 != 0));
                result.insert("rx_stbc".to_string(), serde_json::Value::Number(((caps >> 8) & 0x3).into()));
                result.insert("delayed_block_ack".to_string(), serde_json::Value::Bool(caps & 0x0400 != 0));
                result.insert("max_amsdu_length".to_string(), serde_json::Value::Number((if caps & 0x0800 != 0 { 7935_u64 } else { 3839_u64 }).into()));
                result.insert("dsss_cck_40mhz".to_string(), serde_json::Value::Bool(caps & 0x1000 != 0));
                result.insert("forty_mhz_intolerant".to_string(), serde_json::Value::Bool(caps & 0x4000 != 0));
                result.insert("lsig_txop_protection".to_string(), serde_json::Value::Bool(caps & 0x8000 != 0));

                // MCS Set 解析
                let mcs = &data[2..18];
                let mut streams: u8 = 0;
                let mut max_mcs = 0u8;
                for i in 0..4 {
                    if mcs[i] != 0 {
                        streams = (i + 1) as u8;
                        max_mcs = max_mcs.max(mcs[i]);
                    }
                }
                result.insert("rx_spatial_streams".to_string(), serde_json::Value::Number(streams.into()));

                // Highest supported data rate
                if data.len() >= 16 {
                    let highest_rate = u16::from_le_bytes([data[12], data[13]]);
                    result.insert("highest_rx_data_rate".to_string(), serde_json::Value::Number(highest_rate.into()));
                }

                // A-MPDU Parameters
                if data.len() >= 20 {
                    let ampdu = data[18];
                    let exponent = (ampdu & 0x03) as u32;
                    let ampdu_factor = 2u32.pow(13 + exponent);
                    result.insert("max_ampdu_length".to_string(), serde_json::Value::String(format!("{} bytes", ampdu_factor)));
                    result.insert("min_mpdu_start_spacing".to_string(), serde_json::Value::Number(((ampdu >> 2) & 0x07).into()));
                }
            }
        }

        48 => {  // RSN (Robust Security Network)
            if data.len() >= 2 {
                result.insert("version".to_string(), serde_json::Value::Number(u16::from_le_bytes([data[0], data[1]]).into()));

                if data.len() >= 4 {
                    let group_cipher = u16::from_le_bytes([data[2], data[3]]);
                    let cipher_name = match group_cipher {
                        0 => "Use group",
                        1 => "WEP-40",
                        2 => "TKIP",
                        3 => "RESERVED",
                        4 => "CCMP (AES)",
                        5 => "WEP-104",
                        6 => "BIP-CMAC-128",
                        7 => "GCMP",
                        8 => "GCMP-256",
                        9 => "CCMP-256",
                        10 => "BIP-GMAC-128",
                        11 => "BIP-GMAC-256",
                        12 => "BIP-CMAC-256",
                        _ => "Unknown",
                    };
                    result.insert("group_cipher".to_string(), serde_json::Value::String(cipher_name.to_string()));
                }

                if data.len() >= 6 {
                    let pairwise_count = u16::from_le_bytes([data[4], data[5]]) as usize;
                    result.insert("pairwise_cipher_count".to_string(), serde_json::Value::Number((pairwise_count as u64).into()));

                    // Parse pairwise ciphers
                    let mut ciphers = Vec::new();
                    for i in 0..pairwise_count.min(8) {
                        let offset = 6 + i * 4;
                        if offset + 2 <= data.len() {
                            let cipher = u16::from_le_bytes([data[offset], data[offset + 1]]);
                            let name = match cipher {
                                0 => "Use group",
                                1 => "WEP-40",
                                2 => "TKIP",
                                3 => "RESERVED",
                                4 => "CCMP (AES)",
                                5 => "WEP-104",
                                7 => "GCMP",
                                8 => "GCMP-256",
                                9 => "CCMP-256",
                                _ => "Unknown",
                            };
                            ciphers.push(name.to_string());
                        }
                    }
                    if !ciphers.is_empty() {
                        result.insert("pairwise_ciphers".to_string(), serde_json::Value::String(ciphers.join(", ")));
                    }

                    // Auth suites
                    let auth_offset = 6 + pairwise_count * 4;
                    if auth_offset + 2 <= data.len() {
                        let auth_count = u16::from_le_bytes([data[auth_offset], data[auth_offset + 1]]) as usize;
                        let mut auths = Vec::new();
                        for i in 0..auth_count.min(4) {
                            let offset = auth_offset + 2 + i * 4;
                            if offset + 2 <= data.len() {
                                let auth = u16::from_le_bytes([data[offset], data[offset + 1]]);
                                let name = match auth {
                                    0 => "Reserved",
                                    1 => "802.1X (WPA2-Enterprise)",
                                    2 => "PSK (WPA2-Personal)",
                                    3 => "FT-802.1X",
                                    4 => "FT-PSK",
                                    5 => "WPA-SHA256",
                                    6 => "WPA-PSK-SHA256",
                                    7 => "TDLS",
                                    8 => "SAE (WPA3)",
                                    9 => "FT-SAE",
                                    11 => "AP-PEER-KEY",
                                    12 => "WPA-SHA256-SUITE-B",
                                    13 => "WPA-SHA384-SUITE-B",
                                    14 => "FT-802.1X-SHA384",
                                    _ => "Unknown",
                                };
                                auths.push(name.to_string());
                            }
                        }
                        if !auths.is_empty() {
                            result.insert("auth_methods".to_string(), serde_json::Value::String(auths.join(", ")));
                        }

                        // RSN Capabilities
                        let rsn_cap_offset = auth_offset + 2 + auth_count * 4;
                        if rsn_cap_offset + 2 <= data.len() {
                            let rsn_caps = u16::from_le_bytes([data[rsn_cap_offset], data[rsn_cap_offset + 1]]);
                            result.insert("preauth".to_string(), serde_json::Value::Bool(rsn_caps & 0x0001 != 0));
                            result.insert("no_pairwise".to_string(), serde_json::Value::Bool(rsn_caps & 0x0002 != 0));
                            result.insert("ptksa_replay_counter".to_string(), serde_json::Value::Number(((rsn_caps >> 2) & 0x03).into()));
                            result.insert("gtksa_replay_counter".to_string(), serde_json::Value::Number(((rsn_caps >> 4) & 0x03).into()));
                            result.insert("mfpr (pmf_required)".to_string(), serde_json::Value::Bool(rsn_caps & 0x0040 != 0));
                            result.insert("mfpc (pmf_capable)".to_string(), serde_json::Value::Bool(rsn_caps & 0x0080 != 0));
                            result.insert("joint_multiband_rsna".to_string(), serde_json::Value::Bool(rsn_caps & 0x0100 != 0));
                            result.insert("peerkey_enabled".to_string(), serde_json::Value::Bool(rsn_caps & 0x0200 != 0));
                        }
                    }
                }
            }
        }

        50 => {  // Extended Supported Rates
            let rates: Vec<String> = data.iter().map(|b| {
                let rate = (b & 0x7F) as f32 * 0.5;
                let mandatory = if b & 0x80 != 0 { " (B)" } else { "" };
                format!("{}{}", rate, mandatory)
            }).collect();
            result.insert("extended_rates_mbps".to_string(), serde_json::Value::String(rates.join(", ")));
        }

        61 => {  // HT Operation
            if data.len() >= 22 {
                let primary_ch = data[0];
                result.insert("primary_channel".to_string(), serde_json::Value::Number(primary_ch.into()));

                let ht_info = u16::from_le_bytes([data[1], data[2]]);
                result.insert("secondary_channel_offset".to_string(), serde_json::Value::String(match ht_info & 0x03 {
                    0 => "No secondary",
                    1 => "Above primary",
                    2 => "Reserved",
                    3 => "Below primary",
                    _ => "Unknown",
                }.to_string()));
                result.insert("sta_channel_width".to_string(), serde_json::Value::Bool(ht_info & 0x04 != 0));
                result.insert("rifs_mode".to_string(), serde_json::Value::Bool(ht_info & 0x08 != 0));
                result.insert("ht_protection".to_string(), serde_json::Value::String(match (ht_info >> 4) & 0x03 {
                    0 => "No protection",
                    1 => "Non-member protection",
                    2 => "20 MHz protection",
                    3 => "Non-HT mixed",
                    _ => "Unknown",
                }.to_string()));
                result.insert("non_greenfield_stas".to_string(), serde_json::Value::Bool(ht_info & 0x0100 != 0));
                result.insert("obss_non_ht_stas".to_string(), serde_json::Value::Bool(ht_info & 0x0200 != 0));
                result.insert("dual_beacon".to_string(), serde_json::Value::Bool(ht_info & 0x0400 != 0));
                result.insert("dual_cts_protection".to_string(), serde_json::Value::Bool(ht_info & 0x0800 != 0));
            }
        }

        70 => {  // RM Enabled Capabilities (802.11k)
            if data.len() >= 5 {
                let caps = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                result.insert("link_measurement".to_string(), serde_json::Value::Bool(caps & 0x00000001 != 0));
                result.insert("neighbor_report".to_string(), serde_json::Value::Bool(caps & 0x00000002 != 0));
                result.insert("parallel_measurements".to_string(), serde_json::Value::Bool(caps & 0x00000004 != 0));
                result.insert("repeated_measurements".to_string(), serde_json::Value::Bool(caps & 0x00000008 != 0));
                result.insert("beacon_passive_measurement".to_string(), serde_json::Value::Bool(caps & 0x00000010 != 0));
                result.insert("beacon_active_measurement".to_string(), serde_json::Value::Bool(caps & 0x00000020 != 0));
                result.insert("beacon_table_measurement".to_string(), serde_json::Value::Bool(caps & 0x00000040 != 0));
                result.insert("beacon_measurement_reporting".to_string(), serde_json::Value::Bool(caps & 0x00000080 != 0));
                result.insert("frame_measurement".to_string(), serde_json::Value::Bool(caps & 0x00000100 != 0));
                result.insert("channel_load_measurement".to_string(), serde_json::Value::Bool(caps & 0x00000200 != 0));
                result.insert("noise_histogram_measurement".to_string(), serde_json::Value::Bool(caps & 0x00000400 != 0));
                result.insert("statistics_measurement".to_string(), serde_json::Value::Bool(caps & 0x00000800 != 0));
                result.insert("lci_measurement".to_string(), serde_json::Value::Bool(caps & 0x00001000 != 0));
                result.insert("lci_azimuth".to_string(), serde_json::Value::Bool(caps & 0x00002000 != 0));
                result.insert("transmit_stream_measurement".to_string(), serde_json::Value::Bool(caps & 0x00004000 != 0));
                result.insert("triggered_transmit_stream_measurement".to_string(), serde_json::Value::Bool(caps & 0x00008000 != 0));

                // Neighbor Report capability
                if data.len() >= 5 {
                    let nr_caps = data[4];
                    result.insert("neighbor_report_offload".to_string(), serde_json::Value::Bool(nr_caps & 0x01 != 0));
                }
            }
        }

        127 => {  // Extended Capabilities
            if !data.is_empty() {
                let caps = u64::from_le_bytes([
                    if data.len() > 0 { data[0] } else { 0 },
                    if data.len() > 1 { data[1] } else { 0 },
                    if data.len() > 2 { data[2] } else { 0 },
                    if data.len() > 3 { data[3] } else { 0 },
                    if data.len() > 4 { data[4] } else { 0 },
                    if data.len() > 5 { data[5] } else { 0 },
                    if data.len() > 6 { data[6] } else { 0 },
                    if data.len() > 7 { data[7] } else { 0 },
                ]);

                result.insert("802.11k_rrm".to_string(), serde_json::Value::Bool(caps & (1 << 12) != 0));
                result.insert("802.11v_bss_transition".to_string(), serde_json::Value::Bool(caps & (1 << 19) != 0));
                result.insert("802.11r_ft_resource_request".to_string(), serde_json::Value::Bool(caps & (1 << 14) != 0));
                result.insert("802.11w_mfp".to_string(), serde_json::Value::Bool(caps & (1 << 6) != 0));
                result.insert("802.11z_tdls_peer_uapsd_buffer".to_string(), serde_json::Value::Bool(caps & (1 << 4) != 0));
                result.insert("tdls".to_string(), serde_json::Value::Bool(caps & (1 << 1) != 0));
                result.insert("tdls_prohibited".to_string(), serde_json::Value::Bool(caps & (1 << 2) != 0));
                result.insert("tdls_channel_switch_prohibited".to_string(), serde_json::Value::Bool(caps & (1 << 3) != 0));
                result.insert("tdls_wider_bandwidth".to_string(), serde_json::Value::Bool(caps & (1 << 26) != 0));
                result.insert("interworking".to_string(), serde_json::Value::Bool(caps & (1 << 7) != 0));
                result.insert("qos_map".to_string(), serde_json::Value::Bool(caps & (1 << 8) != 0));
                result.insert("extended_channel_switching".to_string(), serde_json::Value::Bool(caps & (1 << 21) != 0));
                result.insert("wmm_notification".to_string(), serde_json::Value::Bool(caps & (1 << 24) != 0));
                result.insert("operating_mode_notification".to_string(), serde_json::Value::Bool(caps & (1 << 25) != 0));
                result.insert("tim_broadcast".to_string(), serde_json::Value::Bool(caps & (1 << 27) != 0));
                result.insert("fils".to_string(), serde_json::Value::Bool(caps & (1 << 38) != 0));
            }
        }

        191 => {  // VHT Capabilities
            if data.len() >= 12 {
                let caps = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                let max_mpdu = match caps & 0x3 {
                    0 => 3895,
                    1 => 7991,
                    2 => 11454,
                    _ => 11454,
                };
                result.insert("max_mpdu_length".to_string(), serde_json::Value::String(format!("{} bytes", max_mpdu)));

                let ch_width = (caps >> 2) & 0x3;
                let width_str = match ch_width {
                    0 => "20/40 MHz",
                    1 => "20/40/80 MHz",
                    2 => "20/40/80/160 MHz",
                    3 => "20/40/80/160/80+80 MHz",
                    _ => "Unknown",
                };
                result.insert("supported_channel_width".to_string(), serde_json::Value::String(width_str.to_string()));

                result.insert("rx_ldpc".to_string(), serde_json::Value::Bool(caps & 0x10 != 0));
                result.insert("short_gi_80mhz".to_string(), serde_json::Value::Bool(caps & 0x20 != 0));
                result.insert("short_gi_160mhz".to_string(), serde_json::Value::Bool(caps & 0x40 != 0));
                result.insert("tx_stbc".to_string(), serde_json::Value::Bool(caps & 0x80 != 0));
                result.insert("rx_stbc".to_string(), serde_json::Value::Number(((caps >> 8) & 0x7).into()));
                result.insert("su_beamformer".to_string(), serde_json::Value::Bool(caps & 0x400 != 0));
                result.insert("su_beamformee".to_string(), serde_json::Value::Bool(caps & 0x800 != 0));
                result.insert("mu_beamformer".to_string(), serde_json::Value::Bool(caps & 0x1000 != 0));
                result.insert("mu_beamformee".to_string(), serde_json::Value::Bool(caps & 0x2000 != 0));
                result.insert("vht_txop_ps".to_string(), serde_json::Value::Bool(caps & 0x4000 != 0));
                result.insert("htc_vht_capable".to_string(), serde_json::Value::Bool(caps & 0x8000 != 0));
                result.insert("max_ampdu_exp".to_string(), serde_json::Value::String(format!("2^{} bytes", ((caps >> 23) & 0x7) + 13)));

                // VHT MCS Set
                if data.len() >= 8 {
                    let rx_mcs = u16::from_le_bytes([data[4], data[5]]);
                    let tx_mcs = u16::from_le_bytes([data[8], data[9]]);

                    // Extract highest MCS and NSS from RX MCS set
                    let rx_highest = (rx_mcs >> 10) & 0x3FF;
                    let rx_nss = rx_mcs & 0x7;
                    result.insert("rx_max_streams".to_string(), serde_json::Value::Number(((rx_nss + 1) as u64).into()));
                    result.insert("rx_max_rate".to_string(), serde_json::Value::Number(rx_highest.into()));

                    let tx_highest = (tx_mcs >> 10) & 0x3FF;
                    let tx_nss = tx_mcs & 0x7;
                    result.insert("tx_max_streams".to_string(), serde_json::Value::Number(((tx_nss + 1) as u64).into()));
                    result.insert("tx_max_rate".to_string(), serde_json::Value::Number(tx_highest.into()));
                }
            }
        }

        192 => {  // VHT Operation
            if data.len() >= 5 {
                let ch_width = data[0];
                result.insert("channel_width".to_string(), serde_json::Value::String(match ch_width {
                    0 => "20/40 MHz",
                    1 => "80 MHz",
                    2 => "160 MHz",
                    3 => "80+80 MHz",
                    _ => "Unknown",
                }.to_string()));
                result.insert("center_freq_seg0".to_string(), serde_json::Value::Number(data[1].into()));
                result.insert("center_freq_seg1".to_string(), serde_json::Value::Number(data[2].into()));
                result.insert("basic_mcs_set".to_string(), serde_json::Value::String(format!("0x{:04x}", u16::from_le_bytes([data[3], data[4]]))));
            }
        }

        221 => {  // Vendor Specific
            if data.len() >= 4 {
                let oui = format!("{:02X}:{:02X}:{:02X}", data[0], data[1], data[2]);
                let oui_name = match oui.as_str() {
                    "00:50:F2" => "Microsoft WMM/WPS",
                    "00:0F:AC" => "IEEE 802.11 (WPA2)",
                    "00:03:7F" => "IEEE 802.11",
                    "00:90:4C" => "Epigram (Broadcom)",
                    "50:6F:9A" => "Wi-Fi Alliance",
                    _ => "Unknown Vendor",
                };
                result.insert("oui".to_string(), serde_json::Value::String(oui));
                result.insert("vendor_name".to_string(), serde_json::Value::String(oui_name.to_string()));
                result.insert("vendor_type".to_string(), serde_json::Value::Number(data[3].into()));

                // WPS 解析 (OUI 00:50:F2, type 0x04)
                if data[0] == 0x00 && data[1] == 0x50 && data[2] == 0xF2 && data[3] == 0x04 {
                    result.insert("wps_detected".to_string(), serde_json::Value::Bool(true));

                    // 解析 WPS 属性
                    let mut i = 4;
                    while i + 4 <= data.len() {
                        let attr_type = u16::from_be_bytes([data[i], data[i + 1]]);
                        let attr_len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;

                        if i + 4 + attr_len > data.len() {
                            break;
                        }

                        let attr_data = &data[i + 4..i + 4 + attr_len];

                        match attr_type {
                            0x104A => { // Version
                                if !attr_data.is_empty() {
                                    result.insert("wps_version".to_string(), serde_json::Value::Number(attr_data[0].into()));
                                }
                            }
                            0x1012 => { // Device Name
                                if let Ok(name) = std::str::from_utf8(attr_data) {
                                    result.insert("wps_device_name".to_string(), serde_json::Value::String(name.to_string()));
                                }
                            }
                            0x1021 => { // Manufacturer
                                if let Ok(mfr) = std::str::from_utf8(attr_data) {
                                    result.insert("wps_manufacturer".to_string(), serde_json::Value::String(mfr.to_string()));
                                }
                            }
                            0x1023 => { // Model Name
                                if let Ok(model) = std::str::from_utf8(attr_data) {
                                    result.insert("wps_model_name".to_string(), serde_json::Value::String(model.to_string()));
                                }
                            }
                            0x1024 => { // Model Number
                                if let Ok(model_num) = std::str::from_utf8(attr_data) {
                                    result.insert("wps_model_number".to_string(), serde_json::Value::String(model_num.to_string()));
                                }
                            }
                            0x1042 => { // Serial Number
                                if let Ok(serial) = std::str::from_utf8(attr_data) {
                                    result.insert("wps_serial_number".to_string(), serde_json::Value::String(serial.to_string()));
                                }
                            }
                            0x1054 => { // Primary Device Type
                                if attr_data.len() >= 8 {
                                    let category = attr_data[0];
                                    let subcategory = u16::from_be_bytes([attr_data[6], attr_data[7]]);
                                    let cat_name = match category {
                                        1 => "Computer",
                                        2 => "Input Device",
                                        3 => "Printer",
                                        4 => "Camera",
                                        5 => "Storage",
                                        6 => "Network Infrastructure (AP/Router)",
                                        7 => "Display",
                                        8 => "Multimedia Device",
                                        9 => "Gaming Device",
                                        10 => "Telephone",
                                        _ => "Unknown",
                                    };
                                    result.insert("wps_device_category".to_string(), serde_json::Value::String(cat_name.to_string()));
                                    result.insert("wps_device_subcategory".to_string(), serde_json::Value::Number(subcategory.into()));
                                }
                            }
                            0x103C => { // RF Bands
                                if !attr_data.is_empty() {
                                    let bands = attr_data[0];
                                    let mut band_list = Vec::new();
                                    if bands & 0x01 != 0 { band_list.push("2.4 GHz"); }
                                    if bands & 0x02 != 0 { band_list.push("5 GHz"); }
                                    if bands & 0x04 != 0 { band_list.push("60 GHz"); }
                                    if bands & 0x08 != 0 { band_list.push("6 GHz"); }
                                    result.insert("wps_rf_bands".to_string(), serde_json::Value::String(band_list.join(", ")));
                                }
                            }
                            0x1044 => { // Selected Registrar
                                if !attr_data.is_empty() {
                                    result.insert("wps_selected_registrar".to_string(), serde_json::Value::Bool(attr_data[0] != 0));
                                }
                            }
                            0x1047 => { // UUID-E
                                result.insert("wps_uuid".to_string(), serde_json::Value::String(hex::encode(attr_data)));
                            }
                            _ => {}
                        }

                        i += 4 + attr_len;
                    }
                }
            }
        }

        255 => {  // Extended Element
            if !data.is_empty() {
                let ext_id = data[0];
                result.insert("extension_id".to_string(), serde_json::Value::Number(ext_id.into()));
                result.insert("extension_id_hex".to_string(), serde_json::Value::String(format!("0x{:02x}", ext_id)));
                result.insert("extension_name".to_string(), serde_json::Value::String(get_ext_ie_name(ext_id).to_string()));

                // HE Capabilities (Ext 35) - WiFi 6
                if ext_id == 35 && data.len() >= 7 {
                    let mac_cap = u16::from_le_bytes([data[1], data[2]]);
                    let phy_cap = u32::from_le_bytes([data[3], data[4], data[5], data[6]]);

                    // HE MAC Capabilities
                    result.insert("he_su_beamformer".to_string(), serde_json::Value::Bool(mac_cap & 0x0001 != 0));
                    result.insert("he_su_beamformee".to_string(), serde_json::Value::Bool(mac_cap & 0x0002 != 0));
                    result.insert("he_mu_beamformer".to_string(), serde_json::Value::Bool(mac_cap & 0x0004 != 0));
                    result.insert("txop_return_response".to_string(), serde_json::Value::Bool(mac_cap & 0x0008 != 0));
                    result.insert("he_link_adaptation".to_string(), serde_json::Value::String(format!("{}", (mac_cap >> 4) & 0x03)));
                    result.insert("all_ack_support".to_string(), serde_json::Value::Bool(mac_cap & 0x0100 != 0));
                    result.insert("ul_mu_response_scheduling".to_string(), serde_json::Value::Bool(mac_cap & 0x0200 != 0));
                    result.insert("a_control".to_string(), serde_json::Value::Bool(mac_cap & 0x0400 != 0));
                    result.insert("bqr".to_string(), serde_json::Value::Bool(mac_cap & 0x0800 != 0));
                    result.insert("srp_responder".to_string(), serde_json::Value::Bool(mac_cap & 0x2000 != 0));

                    // HE PHY Capabilities
                    result.insert("he_40mhz_2.4ghz".to_string(), serde_json::Value::Bool(phy_cap & 0x00000002 != 0));
                    result.insert("he_160mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x00000004 != 0));
                    result.insert("he_160_80_80mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x00000008 != 0));
                    result.insert("he_320mhz_6ghz".to_string(), serde_json::Value::Bool(phy_cap & 0x00000010 != 0));
                    result.insert("he_ldpc".to_string(), serde_json::Value::Bool(phy_cap & 0x00000020 != 0));
                    result.insert("he_su_ppdu_1x_ltf_0_8us".to_string(), serde_json::Value::Bool(phy_cap & 0x00000040 != 0));
                    result.insert("he_ndp_4x_ltf_3_2us".to_string(), serde_json::Value::Bool(phy_cap & 0x00000200 != 0));
                    result.insert("he_stbc_tx_leq_80mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x00000400 != 0));
                    result.insert("he_stbc_rx_leq_80mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x00000800 != 0));
                    result.insert("doppler_tx".to_string(), serde_json::Value::Bool(phy_cap & 0x00001000 != 0));
                    result.insert("doppler_rx".to_string(), serde_json::Value::Bool(phy_cap & 0x00002000 != 0));
                    result.insert("full_bw_ul_mu_mimo".to_string(), serde_json::Value::Bool(phy_cap & 0x00004000 != 0));
                    result.insert("partial_bw_ul_mu_mimo".to_string(), serde_json::Value::Bool(phy_cap & 0x00008000 != 0));
                    result.insert("he_su_beamformer_leq_80mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x00010000 != 0));
                    result.insert("he_su_beamformer_160mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x00020000 != 0));
                    result.insert("he_mu_beamformer_leq_80mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x00080000 != 0));
                    result.insert("he_mu_beamformer_160mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x00100000 != 0));
                    result.insert("he_ppe_thresholds_present".to_string(), serde_json::Value::Bool(phy_cap & 0x08000000 != 0));
                }

                // HE Operation (Ext 36) - WiFi 6
                if ext_id == 36 && data.len() >= 3 {
                    let op_params = data[1];
                    let op_info = u16::from_le_bytes([data[1], data[2]]);

                    result.insert("default_pe_duration".to_string(), serde_json::Value::Number((op_params & 0x03).into()));
                    result.insert("twt_required".to_string(), serde_json::Value::Bool(op_params & 0x08 != 0));
                    result.insert("tx_bssid_indicator".to_string(), serde_json::Value::Bool(op_params & 0x10 != 0));
                    result.insert("bss_color".to_string(), serde_json::Value::Number(((op_info >> 8) as u64).into()));
                    result.insert("partial_bss_color".to_string(), serde_json::Value::Bool(op_info & 0x4000 != 0));
                    result.insert("bss_color_disabled".to_string(), serde_json::Value::Bool(op_info & 0x8000 != 0));

                    if data.len() >= 5 {
                        let center_seg0 = data[4];
                        result.insert("he_center_freq_seg0".to_string(), serde_json::Value::Number(center_seg0.into()));
                    }
                }

                // EHT Capabilities (Ext 108) - WiFi 7
                if ext_id == 108 && data.len() >= 7 {
                    let mac_cap = u16::from_le_bytes([data[1], data[2]]);
                    let phy_cap = u32::from_le_bytes([data[3], data[4], data[5], data[6]]);

                    // EHT MAC Capabilities
                    result.insert("epcs_priority_access".to_string(), serde_json::Value::Bool(mac_cap & 0x0001 != 0));
                    result.insert("om_control".to_string(), serde_json::Value::Bool(mac_cap & 0x0002 != 0));
                    result.insert("txop_return_response".to_string(), serde_json::Value::Bool(mac_cap & 0x0004 != 0));
                    result.insert("two_bqr".to_string(), serde_json::Value::Bool(mac_cap & 0x0008 != 0));
                    result.insert("mscs".to_string(), serde_json::Value::Bool(mac_cap & 0x0010 != 0));
                    result.insert("sel_tra".to_string(), serde_json::Value::Bool(mac_cap & 0x0020 != 0));
                    result.insert("link_adaptation_with_unsolicited_mfb".to_string(), serde_json::Value::Bool(mac_cap & 0x0080 != 0));
                    result.insert("triggered_txop_sharing".to_string(), serde_json::Value::Bool(mac_cap & 0x0100 != 0));
                    result.insert("restricted_twt".to_string(), serde_json::Value::Bool(mac_cap & 0x0200 != 0));
                    result.insert("scs_traffic_description".to_string(), serde_json::Value::Bool(mac_cap & 0x0400 != 0));

                    // EHT PHY Capabilities
                    let support_320 = phy_cap & 0x03;
                    result.insert("320mhz_6ghz_support".to_string(), serde_json::Value::String(match support_320 {
                        0 => "Not supported",
                        1 => "320MHz-1 (no 160+160MHz)",
                        2 => "320MHz-2",
                        3 => "Reserved",
                        _ => "Unknown",
                    }.to_string()));
                    result.insert("ru_242_tone_bw_leq_80mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x04 != 0));
                    result.insert("ndp_4x_eht_ltf_3_2us".to_string(), serde_json::Value::Bool(phy_cap & 0x08 != 0));
                    result.insert("partial_bw_ul_mu_mimo".to_string(), serde_json::Value::Bool(phy_cap & 0x10 != 0));
                    result.insert("su_beamformer".to_string(), serde_json::Value::Bool(phy_cap & 0x20 != 0));
                    result.insert("su_beamformee".to_string(), serde_json::Value::Bool(phy_cap & 0x40 != 0));

                    let bfee_sts_le_80 = (phy_cap >> 7) & 0x07;
                    result.insert("bfee_sts_leq_80mhz".to_string(), serde_json::Value::Number((bfee_sts_le_80 + 1).into()));

                    let bfee_sts_gt_80 = (phy_cap >> 10) & 0x07;
                    result.insert("bfee_sts_gt_80mhz".to_string(), serde_json::Value::Number((bfee_sts_gt_80 + 1).into()));

                    result.insert("rx_1024_4096_qam_leq_80mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x10000 != 0));
                    result.insert("rx_1024_4096_qam_80mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x20000 != 0));
                    result.insert("rx_1024_4096_qam_160mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x40000 != 0));
                    result.insert("rx_1024_4096_qam_320mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x80000 != 0));
                    result.insert("tx_1024_4096_qam_leq_80mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x100000 != 0));
                    result.insert("tx_1024_4096_qam_80mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x200000 != 0));
                    result.insert("tx_1024_4096_qam_160mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x400000 != 0));
                    result.insert("tx_1024_4096_qam_320mhz".to_string(), serde_json::Value::Bool(phy_cap & 0x800000 != 0));
                }

                // EHT Operation (Ext 106) - WiFi 7
                if ext_id == 106 && data.len() >= 3 {
                    let op_params = data[1];
                    let ch_width = (op_params >> 2) & 0x07;
                    result.insert("eht_channel_width".to_string(), serde_json::Value::String(match ch_width {
                        0 => "20 MHz",
                        1 => "40 MHz",
                        2 => "80 MHz",
                        3 => "160 MHz",
                        4 => "320 MHz",
                        _ => "Unknown",
                    }.to_string()));

                    if data.len() >= 4 {
                        let center_seg0 = data[3];
                        result.insert("eht_center_freq_seg0".to_string(), serde_json::Value::Number(center_seg0.into()));
                    }
                    if data.len() >= 5 {
                        let center_seg1 = data[4];
                        result.insert("eht_center_freq_seg1".to_string(), serde_json::Value::Number(center_seg1.into()));
                    }
                }

                // MU EDCA (Ext 37)
                if ext_id == 37 && !data.is_empty() {
                    result.insert("mu_edca_parameter_set".to_string(), serde_json::Value::Bool(true));
                }

                // Multi-BSSID (Ext 38)
                if ext_id == 38 && data.len() >= 1 {
                    let max_bssid = data[1] >> 4;
                    result.insert("max_bssid_indicator".to_string(), serde_json::Value::Number(max_bssid.into()));
                    result.insert("number_of_bssids".to_string(), serde_json::Value::Number((1 << max_bssid).into()));
                }
            }
        }

        _ => {}
    }

    result
}

/// 完整解析所有 IE
fn parse_all_ies(ie_data: &[u8]) -> IEDetails {
    let mut elements = Vec::new();
    let mut detection = DetectionSummary::default();

    let mut pos = 0;
    while pos + 1 < ie_data.len() {
        let element_id = ie_data[pos];
        let length = ie_data[pos + 1] as usize;

        if pos + 2 + length > ie_data.len() {
            break;
        }

        let data = &ie_data[pos + 2..pos + 2 + length];

        // 更新检测摘要
        match element_id {
            45 => detection.has_ht_capabilities = true,
            61 => detection.has_ht_operation = true,
            191 => detection.has_vht_capabilities = true,
            192 => detection.has_vht_operation = true,
            255 if length >= 1 => {
                match data[0] {
                    35 => detection.has_he_capabilities = true,
                    36 => detection.has_he_operation = true,
                    106 => detection.has_eht_operation = true,
                    108 => detection.has_eht_capabilities = true,
                    _ => {}
                }
            }
            _ => {}
        }

        elements.push(ParsedIE {
            element_id,
            element_id_hex: format!("0x{:02x}", element_id),
            name: get_ie_name(element_id).to_string(),
            length: length as u8,
            data_hex: hex::encode(data),
            parsed: parse_ie_content(element_id, data),
        });

        pos += 2 + length;
    }

    // 确定检测到的标准
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

#[cfg(target_os = "macos")]
fn get_ie_details_for_bssid(bssid: &str) -> Option<IEDetails> {
    use std::process::Command;
    use plist::Value;
    use std::io::Cursor;

    let output = Command::new("/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport")
        .args(["-s", "-x"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let plist = Value::from_reader(Cursor::new(stdout.as_bytes())).ok()?;
    let networks = plist.as_array()?;

    for net_dict in networks {
        if let Some(dict) = net_dict.as_dictionary() {
            let current_bssid = dict.get("BSSID")
                .and_then(|v| v.as_string())
                .map(|s| s.to_uppercase())?;

            if current_bssid.to_uppercase() == bssid.to_uppercase() {
                if let Some(ie_data) = dict.get("IE").and_then(|v| v.as_data()) {
                    return Some(parse_all_ies(ie_data));
                }
            }
        }
    }

    None
}

#[cfg(not(target_os = "macos"))]
fn get_ie_details_for_bssid(_bssid: &str) -> Option<IEDetails> {
    None
}

// ============ Tauri Commands ============

#[tauri::command]
fn scan_networks() -> Result<Vec<Network>, String> {
    scanner::scan()
}

#[tauri::command]
fn current_network() -> Result<Option<Network>, String> {
    scanner::current()
}

#[tauri::command]
fn get_network_groups() -> Result<Vec<NetworkGroup>, String> {
    let networks = scanner::scan()?;
    Ok(scanner::get_groups(&networks))
}

#[tauri::command]
fn get_scan_stats() -> Result<ScanStats, String> {
    let start = std::time::Instant::now();
    let networks = scanner::scan()?;
    Ok(scanner::get_stats(&networks, start.elapsed().as_millis() as u64))
}

#[tauri::command]
fn start_monitor(app: tauri::AppHandle) -> Result<(), String> {
    MONITORING.store(true, Ordering::SeqCst);

    let app_handle = app.clone();
    std::thread::spawn(move || {
        while MONITORING.load(Ordering::SeqCst) {
            if let Ok(networks) = scanner::scan() {
                let _ = app_handle.emit("networks-updated", networks);
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });

    Ok(())
}

#[tauri::command]
fn stop_monitor() -> Result<(), String> {
    MONITORING.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn get_ie_details(bssid: String) -> Option<IEDetails> {
    get_ie_details_for_bssid(&bssid)
}

#[tauri::command]
fn lookup_vendor(bssid: String) -> Option<String> {
    let oui = bssid.replace(":", "").replace("-", "").to_uppercase();
    if oui.len() < 6 {
        return None;
    }
    let prefix = &oui[0..6];

    let vendors = [
        ("001A2B", "TP-Link"), ("001E58", "ASUSTek"), ("00226B", "Cisco"),
        ("00246C", "Apple"), ("005056", "VMware"), ("04D4C4", "Apple"),
        ("086698", "Apple"), ("0C4DE9", "Apple"), ("10E341", "Huawei"),
        ("18A6F7", "Xiaomi"), ("2034FB", "Apple"), ("240A64", "Xiaomi"),
        ("30074D", "Apple"), ("3423BA", "Apple"), ("38F9D3", "Apple"),
        ("44D884", "Apple"), ("483B38", "Apple"), ("5C5948", "Intel"),
        ("68DBCA", "Apple"), ("6C5C14", "TP-Link"), ("784F43", "Apple"),
        ("7C6D62", "Apple"), ("7CD1C3", "Intel"), ("80EAD2", "Ubiquiti"),
        ("849FAD", "Apple"), ("90B0ED", "Xiaomi"), ("94BF2D", "Cisco"),
        ("94F6A3", "Apple"), ("9C2AA4", "Ubiquiti"), ("9CF48E", "Apple"),
        ("A01828", "Ubiquiti"), ("A4B197", "TP-Link"), ("ACF7F3", "Apple"),
        ("B06EBF", "Ubiquiti"), ("B827EB", "Raspberry Pi"), ("BC52B7", "Apple"),
        ("C069CD", "Apple"), ("D461DA", "Apple"), ("D81C79", "Apple"),
        ("F4D884", "Apple"), ("68DDB7", "Xiaomi"), ("089AC7", "Xiaomi"),
    ];

    for (oui_prefix, vendor) in vendors {
        if prefix == oui_prefix {
            return Some(vendor.to_string());
        }
    }
    None
}

// ============ Entry Point ============

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            scan_networks,
            current_network,
            get_network_groups,
            get_scan_stats,
            start_monitor,
            stop_monitor,
            get_ie_details,
            lookup_vendor
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
