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
        Self { interface: None }
    }
    
    pub fn with_interface(iface: impl Into<String>) -> Self {
        Self { interface: Some(iface.into()) }
    }
    
    fn find_interface(&self) -> Result<String, ScanError> {
        if let Some(ref iface) = self.interface {
            return Ok(iface.clone());
        }
        
        // Try to find a wireless interface
        let output = Command::new("iw")
            .args(["dev"])
            .output()
            .map_err(|e| ScanError::CommandFailed(e.to_string()))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parse output: "Interface wlan0"
        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("Interface\t") {
                return Ok(line.split('\t').nth(1).unwrap_or("wlan0").to_string());
            }
        }
        
        // Fallback
        Err(ScanError::NoInterface)
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
    
    for line in output.lines() {
        let line = line.trim();
        
        // New BSS entry
        if line.starts_with("BSS ") {
            if let Some(beacon) = current.take() {
                results.push(beacon);
            }
            
            // Parse BSSID from "BSS xx:xx:xx:xx:xx:xx"
            let bssid_str = line.split(' ').nth(1).unwrap_or("");
            current = Some(RawBeacon {
                bssid: parse_bssid(bssid_str).unwrap_or([0; 6]),
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
            beacon.ssid = Some(line[5..].trim().as_bytes().to_vec());
        } else if line.starts_with("freq:") {
            let freq: u32 = line[5..].trim().parse().unwrap_or(0);
            beacon.channel = freq_to_channel(freq);
            beacon.band = Band::from_channel(beacon.channel);
        } else if line.starts_with("signal:") {
            let sig_str = line[7..].trim();
            beacon.signal_dbm = sig_str.replace(" dBm", "").parse().unwrap_or(-100);
        } else if line.starts_with("beacon interval:") {
            beacon.beacon_interval = line[16..].trim().parse().unwrap_or(100);
        }
    }
    
    if let Some(beacon) = current {
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
        ..Default::default()
    };
    
    for line in stdout.lines() {
        let line = line.trim();
        
        if line.starts_with("Connected to ") {
            let bssid_str = line[12..].split(' ').next().unwrap_or("");
            beacon.bssid = parse_bssid(bssid_str).unwrap_or([0; 6]);
        } else if line.starts_with("SSID:") {
            beacon.ssid = Some(line[5..].trim().as_bytes().to_vec());
        } else if line.starts_with("freq:") {
            let freq: u32 = line[5..].trim().parse().unwrap_or(0);
            beacon.channel = freq_to_channel(freq);
            beacon.band = Band::from_channel(beacon.channel);
        } else if line.starts_with("signal:") {
            let sig_str = line[7..].trim();
            beacon.signal_dbm = sig_str.replace(" dBm", "").parse().unwrap_or(-100);
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
