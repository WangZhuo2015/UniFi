//! Linux Nl80211 Scanner
//!
//! Uses `iw` command or netlink socket to scan WiFi networks.
//! SBC-friendly - no GUI dependencies.

use crate::scanner::{RawBeacon, Scanner};
use crate::types::{Band, ScanError};
use std::process::Command;

pub struct Nl80211Scanner {
    interface: Option<String>,
}

impl Nl80211Scanner {
    pub fn new() -> Self {
        // Auto-detect WiFi interface
        let interface = Self::detect_interface();
        Self { interface }
    }

    fn detect_interface() -> Option<String> {
        // Try iw dev first
        let output = Command::new("iw")
            .args(["dev"])
            .output()
            .ok();

        if let Some(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                // "Interface wlan0" format
                if line.starts_with("Interface ") {
                    return line.split_whitespace().nth(1).map(|s| s.to_string());
                }
            }
        }

        // Fallback: check /sys/class/net for wireless
        for entry in std::fs::read_dir("/sys/class/net").ok()? {
            if let Ok(entry) = entry {
                let name = entry.file_name().to_string_lossy().to_string();
                if std::path::Path::new(&format!("/sys/class/net/{}/wireless", name)).exists() {
                    return Some(name);
                }
            }
        }

        // Last fallback
        Some("wlan0".to_string())
    }

    pub fn with_interface(iface: impl Into<String>) -> Self {
        Self { interface: Some(iface.into()) }
    }

    fn find_interface(&self) -> Result<String, ScanError> {
        if let Some(ref iface) = self.interface {
            return Ok(iface.clone());
        }

        // Should have been auto-detected, but try again
        Self::detect_interface().ok_or(ScanError::NoInterface)
    }
}

impl Scanner for Nl80211Scanner {
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
        let iface = self.find_interface()?;
        scan_with_iw(&iface)
    }

    fn current(&self) -> Result<Option<RawBeacon>, ScanError> {
        let iface = self.find_interface()?;
        current_with_iwlink(&iface)
    }

    fn name(&self) -> &'static str {
        "Linux nl80211"
    }

    fn requires_privilege(&self) -> bool {
        true // iw scan requires root
    }
}

fn scan_with_iw(iface: &str) -> Result<Vec<RawBeacon>, ScanError> {
    let output = Command::new("iw")
        .args(["dev", iface, "scan"])
        .output()
        .map_err(|e| ScanError::CommandFailed(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Operation not permitted") {
            return Err(ScanError::PermissionDenied);
        }
        return Err(ScanError::CommandFailed(stderr.into()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_iw_scan(&stdout)
}

fn parse_iw_scan(output: &str) -> Result<Vec<RawBeacon>, ScanError> {
    let mut results = Vec::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut current: Option<RawBeacon> = None;
    let mut ie_data: Vec<u8> = Vec::new();

    for line in output.lines() {
        let line = line.trim();

        // New BSS entry: "BSS 80:2d:1a:4b:8c:07(on wlan0)" - must have BSSID format
        // Avoid matching "BSS Load:" or "BSS Transition" lines
        if line.starts_with("BSS ") && line.contains(':') && line.chars().nth(4).map_or(false, |c| c.is_ascii_hexdigit()) {
            // Save previous beacon
            if let Some(mut beacon) = current.take() {
                beacon.ie_data = ie_data.clone();
                results.push(beacon);
            }
            ie_data.clear();

            // Parse BSSID - extract before "(on" or end
            let bssid_part = line.split('(').next().unwrap_or("")
                .strip_prefix("BSS ")
                .unwrap_or("")
                .trim();
            current = Some(RawBeacon {
                bssid: parse_bssid(bssid_part).unwrap_or([0; 6]),
                timestamp: now,
                ..Default::default()
            });
            continue;
        }

        let beacon = match &mut current {
            Some(b) => b,
            None => continue,
        };

        if line.starts_with("SSID:") {
            let ssid = line[5..].trim();
            beacon.ssid = Some(ssid.as_bytes().to_vec());
            // Add SSID IE (ID 0)
            ie_data.push(0);
            ie_data.push(ssid.len() as u8);
            ie_data.extend_from_slice(ssid.as_bytes());
        } else if line.starts_with("freq:") {
            // Parse "freq: 5240.0" - extract number before decimal
            let freq_str = line.split(':').nth(1).unwrap_or("0")
                .split('.')
                .next()
                .unwrap_or("0");
            let freq: u32 = freq_str.trim().parse().unwrap_or(0);
            beacon.channel = freq_to_channel(freq);
            beacon.band = Band::from_channel(beacon.channel);
            // Add DS Parameter Set IE
            ie_data.push(3);
            ie_data.push(1);
            ie_data.push(beacon.channel);
        } else if line.starts_with("signal:") {
            // Parse "signal: -62.00 dBm"
            let sig_full = line.split(':').nth(1).unwrap_or("-100 dBm");
            let sig_clean: String = sig_full.replace(" dBm", "");
            // Handle decimal: -62.00 -> -62
            let sig_int: String = sig_clean.trim().split('.').next().unwrap_or("-100").to_string();
            beacon.signal_dbm = sig_int.parse().unwrap_or(-100);
        } else if line.starts_with("beacon interval:") {
            beacon.beacon_interval = line.split(':').nth(1)
                .unwrap_or("100")
                .trim()
                .split(' ')
                .next()
                .unwrap_or("100")
                .parse()
                .unwrap_or(100);
        } else if line.contains("primary channel:") {
            // HT operation contains primary channel
            if let Some(ch) = line.split(':').nth(1) {
                beacon.channel = ch.trim().parse().unwrap_or(beacon.channel);
            }
        } else if line.contains("Supported rates:") {
            let rates_part = line.split(':').nth(1).unwrap_or("");
            let mut rates = Vec::new();
            for rate_str in rates_part.split_whitespace() {
                let rate_clean = rate_str.replace('*', "");
                if let Ok(rate) = rate_clean.parse::<f32>() {
                    let rate_unit = (rate * 2.0) as u8;
                    // Mark basic rates with MSB
                    if rate_str.contains('*') {
                        rates.push(rate_unit | 0x80);
                    } else {
                        rates.push(rate_unit);
                    }
                }
            }
            if !rates.is_empty() && rates.len() <= 8 {
                ie_data.push(1);
                ie_data.push(rates.len() as u8);
                ie_data.extend_from_slice(&rates);
            }
        } else if line.starts_with("HT capabilities:") {
            // Mark as WiFi 4 - actual HT caps are in subsequent lines
            // We'll set a flag but won't add fake IE data
            beacon.has_ht = true;
        } else if line.starts_with("VHT capabilities:") {
            // Mark as WiFi 5 - actual VHT caps are in subsequent lines
            beacon.has_vht = true;
        } else if line.starts_with("HE capabilities:") {
            // Mark as WiFi 6
            beacon.has_he = true;
        } else if line.starts_with("EHT capabilities:") {
            // Mark as WiFi 7
            beacon.has_eht = true;
        } else if line.contains("MCS rate indexes supported:") {
            // Parse spatial streams from "HT TX/RX MCS rate indexes supported: 0-31"
            // 0-7 = 1 stream, 0-15 = 2 streams, 0-23 = 3 streams, 0-31 = 4 streams
            if let Some(range) = line.split(':').nth(1) {
                let range = range.trim();
                if let Some(max_mcs) = range.split('-').nth(1) {
                    if let Ok(max) = max_mcs.trim().parse::<u8>() {
                        beacon.spatial_streams = Some((max / 8 + 1) as u8);
                    }
                }
            }
        }
    }

    // Save last beacon
    if let Some(mut beacon) = current {
        beacon.ie_data = ie_data;
        results.push(beacon);
    }

    Ok(results)
}

fn current_with_iwlink(iface: &str) -> Result<Option<RawBeacon>, ScanError> {
    let output = Command::new("iw")
        .args(["dev", iface, "link"])
        .output()
        .map_err(|e| ScanError::CommandFailed(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.contains("Not connected") {
        return Ok(None);
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut beacon = RawBeacon {
        timestamp: now,
        connected: true,
        link_rates: None,
        ..Default::default()
    };

    for line in stdout.lines() {
        let line = line.trim();

        if line.starts_with("Connected to ") {
            // Parse "Connected to 80:2d:1a:4b:8c:07 (on wlan0)"
            let bssid_part = line.split('(').next().unwrap_or("")
                .strip_prefix("Connected to ")
                .unwrap_or("")
                .trim();
            beacon.bssid = parse_bssid(bssid_part).unwrap_or([0; 6]);
        } else if line.starts_with("SSID:") {
            beacon.ssid = Some(line[5..].trim().as_bytes().to_vec());
        } else if line.starts_with("freq:") {
            // Parse "freq: 5240.0"
            let freq_str = line.split(':').nth(1).unwrap_or("0")
                .split('.')
                .next()
                .unwrap_or("0");
            let freq: u32 = freq_str.trim().parse().unwrap_or(0);
            beacon.channel = freq_to_channel(freq);
            beacon.band = Band::from_channel(beacon.channel);
        } else if line.starts_with("signal:") {
            // Parse "signal: -59 dBm"
            let sig_str = line.split(':').nth(1).unwrap_or("-100 dBm")
                .replace(" dBm", "");
            beacon.signal_dbm = sig_str.trim().parse().unwrap_or(-100);
        } else if line.starts_with("tx bitrate:") {
            // Parse "tx bitrate: 292.5 MBit/s"
            let rate_str = line.split(':').nth(1).unwrap_or("0 MBit/s")
                .replace(" MBit/s", "");
            let tx_rate: f32 = rate_str.trim().parse().unwrap_or(0.0);
            beacon.link_rates = Some(crate::types::LinkRates {
                rx_rate_mbps: None,
                tx_rate_mbps: Some(tx_rate),
            });
        }
    }

    Ok(Some(beacon))
}

fn parse_bssid(s: &str) -> Result<[u8; 6], ScanError> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 6 {
        return Err(ScanError::ParseError("Invalid BSSID".into()));
    }
    
    let mut bssid = [0u8; 6];
    for (i, p) in parts.iter().enumerate() {
        bssid[i] = u8::from_str_radix(p, 16)
            .map_err(|_| ScanError::ParseError("Invalid BSSID byte".into()))?;
    }
    
    Ok(bssid)
}

fn freq_to_channel(freq: u32) -> u8 {
    match freq {
        2412..=2484 => ((freq - 2407) / 5) as u8,
        5170..=5825 => ((freq - 5000) / 5) as u8,
        _ => 0,
    }
}
