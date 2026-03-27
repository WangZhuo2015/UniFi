//! macOS Airport Scanner
//!
//! Uses the airport CLI tool to scan WiFi networks.
//! Note: This tool may not work on macOS 26+.
//! Use CoreWLAN scanner for better compatibility.

use crate::scanner::{RawBeacon, Scanner};
use crate::types::{Band, ScanError};
use std::io::Cursor;
use std::path::Path;
use std::process::Command;

const AIRPORT_PATH: &str = "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport";

pub struct AirportScanner {
    available: bool,
}

impl AirportScanner {
    pub fn new() -> Self {
        let available = Path::new(AIRPORT_PATH).exists();
        Self { available }
    }
}

impl Default for AirportScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Scanner for AirportScanner {
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
        if !self.available {
            return Err(ScanError::NotSupported);
        }

        let output = Command::new(AIRPORT_PATH)
            .args(["-s", "-x"])
            .output()
            .map_err(|e| ScanError::CommandFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(ScanError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).into()
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_airport_xml(&stdout)
    }

    fn current(&self) -> Result<Option<RawBeacon>, ScanError> {
        if !self.available {
            return Err(ScanError::NotSupported);
        }

        // First get current SSID from -I
        let output = Command::new(AIRPORT_PATH)
            .args(["-I"])
            .output()
            .map_err(|e| ScanError::CommandFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.contains("AirPort: Off") || stdout.is_empty() {
            return Ok(None);
        }

        // Parse current network info
        let current_info = parse_current_info_detailed(&stdout);

        // If we have an SSID, scan to get full IE data
        if let Some(ref ssid) = current_info.ssid {
            if let Ok(beacons) = self.scan() {
                for mut beacon in beacons {
                    if beacon.ssid.as_ref().map(|s| s.as_slice()) == Some(ssid.as_slice()) {
                        beacon.connected = true;
                        beacon.signal_dbm = current_info.signal_dbm;
                        return Ok(Some(beacon));
                    }
                }
            }
        }

        // Fallback to basic info
        Ok(Some(current_info.into_beacon()))
    }

    fn name(&self) -> &'static str {
        "macOS Airport (Legacy)"
    }

    fn is_available(&self) -> bool {
        self.available
    }
}

fn parse_airport_xml(xml: &str) -> Result<Vec<RawBeacon>, ScanError> {
    let plist = plist::Value::from_reader(Cursor::new(xml.as_bytes()))
        .map_err(|e| ScanError::ParseError(e.to_string()))?;
    
    let networks = plist.as_array()
        .ok_or(ScanError::ParseError("Expected array".into()))?;
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let mut results = Vec::new();
    
    for net in networks {
        if let Some(dict) = net.as_dictionary() {
            if let Ok(beacon) = parse_network_dict(dict, now) {
                results.push(beacon);
            }
        }
    }
    
    Ok(results)
}

fn parse_network_dict(dict: &plist::Dictionary, now: u64) -> Result<RawBeacon, ScanError> {
    // BSSID
    let bssid_str = dict.get("BSSID")
        .and_then(|v| v.as_string())
        .ok_or(ScanError::ParseError("Missing BSSID".into()))?;
    
    let bssid = parse_bssid(bssid_str)?;
    
    // SSID
    let ssid = dict.get("SSID_STR")
        .and_then(|v| v.as_string())
        .map(|s| s.as_bytes().to_vec());
    
    // Channel
    let channel = dict.get("CHANNEL")
        .and_then(|v| v.as_signed_integer())
        .map(|v| v as u8)
        .unwrap_or(0);
    
    // Signal
    let signal = dict.get("RSSI")
        .and_then(|v| v.as_signed_integer())
        .map(|v| v as i16)
        .unwrap_or(-100);
    
    // Noise
    let noise = dict.get("NOISE")
        .and_then(|v| v.as_signed_integer())
        .map(|v| v as i16)
        .unwrap_or(-100);
    
    // IE data
    let ie_data = dict.get("IE")
        .and_then(|v| v.as_data())
        .map(|d| d.to_vec())
        .unwrap_or_default();
    
    // Beacon interval
    let beacon_interval = dict.get("BEACON_INT")
        .and_then(|v| v.as_signed_integer())
        .map(|v| v as u16)
        .unwrap_or(100);
    
    let band = Band::from_channel(channel);
    
    Ok(RawBeacon {
        ssid,
        bssid,
        channel,
        band,
        signal_dbm: signal,
        noise_dbm: noise,
        ie_data,
        beacon_interval,
        timestamp: now,
        connected: false,
    })
}

fn parse_bssid(s: &str) -> Result<[u8; 6], ScanError> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 6 {
        return Err(ScanError::ParseError("Invalid BSSID format".into()));
    }
    
    let mut bssid = [0u8; 6];
    for (i, p) in parts.iter().enumerate() {
        bssid[i] = u8::from_str_radix(p, 16)
            .map_err(|_| ScanError::ParseError("Invalid BSSID byte".into()))?;
    }
    
    Ok(bssid)
}

fn parse_current_info(output: &str) -> Option<RawBeacon> {
    let mut ssid: Option<Vec<u8>> = None;
    let mut bssid = [0u8; 6];
    let mut channel: u8 = 0;
    let mut signal: i16 = -100;

    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("SSID:") {
            ssid = Some(line[5..].trim().as_bytes().to_vec());
        } else if line.starts_with("BSSID:") {
            let bssid_str = line[6..].trim();
            if let Ok(parsed) = parse_bssid(bssid_str) {
                bssid = parsed;
            }
        } else if line.starts_with("agrCtlRSSI:") {
            signal = line[11..].trim().parse().unwrap_or(-100);
        } else if line.starts_with("channel:") {
            channel = line[8..].trim().split(',').next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
        }
    }

    let ssid = ssid?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Some(RawBeacon {
        ssid: Some(ssid),
        bssid,
        channel,
        band: Band::from_channel(channel),
        signal_dbm: signal,
        noise_dbm: -90,
        ie_data: vec![],
        beacon_interval: 100,
        timestamp: now,
        connected: true,
    })
}

/// Detailed current network info from airport -I
struct CurrentNetworkInfo {
    ssid: Option<Vec<u8>>,
    signal_dbm: i16,
    noise_dbm: i16,
    channel: u8,
    #[allow(dead_code)]
    tx_rate: u16,
    #[allow(dead_code)]
    mcs: u8,
    #[allow(dead_code)]
    nss: u8,
    #[allow(dead_code)]
    security: String,
}

impl CurrentNetworkInfo {
    fn into_beacon(self) -> RawBeacon {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        RawBeacon {
            ssid: self.ssid,
            bssid: [0u8; 6], // Will be filled from scan
            channel: self.channel,
            band: Band::from_channel(self.channel),
            signal_dbm: self.signal_dbm,
            noise_dbm: self.noise_dbm,
            ie_data: vec![],
            beacon_interval: 100,
            timestamp: now,
            connected: true,
        }
    }
}

fn parse_current_info_detailed(output: &str) -> CurrentNetworkInfo {
    let mut ssid: Option<Vec<u8>> = None;
    let mut signal: i16 = -100;
    let mut noise: i16 = -90;
    let mut channel: u8 = 0;
    let mut tx_rate: u16 = 0;
    let mut mcs: u8 = 0;
    let mut nss: u8 = 1;
    let mut security = "unknown".to_string();

    for line in output.lines() {
        let line = line.trim();
        if line.starts_with("SSID:") {
            let s = line[5..].trim();
            if !s.is_empty() {
                ssid = Some(s.as_bytes().to_vec());
            }
        } else if line.starts_with("agrCtlRSSI:") {
            signal = line[11..].trim().parse().unwrap_or(-100);
        } else if line.starts_with("agrCtlNoise:") {
            noise = line[12..].trim().parse().unwrap_or(-90);
        } else if line.starts_with("channel:") {
            channel = line[8..].trim().split(',')
                .next()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
        } else if line.starts_with("lastTxRate:") {
            tx_rate = line[11..].trim().parse().unwrap_or(0);
        } else if line.starts_with("MCS:") {
            mcs = line[4..].trim().parse().unwrap_or(0);
        } else if line.starts_with("NSS:") {
            nss = line[4..].trim().parse().unwrap_or(1);
        } else if line.starts_with("link auth:") {
            let auth = line[10..].trim();
            security = match auth {
                "wpa2-psk" => "wpa2",
                "wpa3-sae" => "wpa3",
                "wpa2-8021x" => "wpa2-ent",
                "wpa3-8021x" => "wpa3-ent",
                "open" => "open",
                _ => auth,
            }.to_string();
        }
    }

    CurrentNetworkInfo {
        ssid,
        signal_dbm: signal,
        noise_dbm: noise,
        channel,
        tx_rate,
        mcs,
        nss,
        security,
    }
}
