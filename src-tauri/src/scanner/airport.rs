//! macOS Airport Scanner
//!
//! Uses the airport CLI tool to scan WiFi networks.

use crate::scanner::{RawBeacon, Scanner};
use crate::types::{Band, ScanError};
use std::io::Cursor;
use std::process::Command;

const AIRPORT_PATH: &str = "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport";

pub struct AirportScanner;

impl AirportScanner {
    pub fn new() -> Self {
        Self
    }
}

impl Scanner for AirportScanner {
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
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
        let output = Command::new(AIRPORT_PATH)
            .args(["-I"])
            .output()
            .map_err(|e| ScanError::CommandFailed(e.to_string()))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        if stdout.contains("AirPort: Off") || stdout.is_empty() {
            return Ok(None);
        }
        
        // Parse current network from -I output
        Ok(parse_current_info(&stdout))
    }
    
    fn name(&self) -> &'static str {
        "macOS Airport"
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
