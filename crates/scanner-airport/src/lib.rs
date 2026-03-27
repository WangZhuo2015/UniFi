//! macOS Airport CLI Scanner Plugin
//!
//! Uses the macOS airport CLI tool to scan for WiFi networks.
//! Provides complete IE data but may not work on macOS 26+.

use scanner_core::{Platform, RawBeacon, ScanError, Scanner, ScannerCapabilities};
use std::process::Command;

/// Airport scanner using macOS airport CLI
pub struct AirportScanner {
    /// Path to airport utility
    airport_path: String,
}

impl Default for AirportScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl AirportScanner {
    /// Create a new Airport scanner
    pub fn new() -> Self {
        Self {
            airport_path: "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport".to_string(),
        }
    }
    
    /// Create with custom airport path
    pub fn with_path(path: impl Into<String>) -> Self {
        Self {
            airport_path: path.into(),
        }
    }
    
    /// Check if airport tool exists
    pub fn check_available(&self) -> bool {
        std::path::Path::new(&self.airport_path).exists()
    }
    
    /// Run airport scan
    fn run_scan(&self) -> Result<String, ScanError> {
        let output = Command::new(&self.airport_path)
            .arg("--scan")
            .arg("--xml")
            .output()
            .map_err(|e| ScanError::System(format!("Failed to run airport: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ScanError::System(format!("Airport scan failed: {}", stderr)));
        }
        
        String::from_utf8(output.stdout)
            .map_err(|e| ScanError::Parse(format!("Invalid UTF-8 output: {}", e)))
    }
    
    /// Parse airport XML output
    fn parse_xml(&self, xml: &str) -> Result<Vec<RawBeacon>, ScanError> {
        #[cfg(target_os = "macos")]
        {
            let plist: plist::Dictionary = plist::from_bytes(xml.as_bytes())
                .map_err(|e| ScanError::Parse(format!("Failed to parse plist: {}", e)))?;
            
            let mut networks = Vec::new();
            
            if let Some(array) = plist.get("DYNAMIC_PLIST").and_then(|v| v.as_array()) {
                for item in array {
                    if let Some(dict) = item.as_dictionary() {
                        let mut beacon = RawBeacon::new();
                        
                        if let Some(ssid) = dict.get("SSID_STR").and_then(|v| v.as_string()) {
                            beacon.ssid = Some(ssid.to_string());
                        }
                        
                        if let Some(bssid) = dict.get("BSSID").and_then(|v| v.as_string()) {
                            beacon.bssid = Some(bssid.to_string());
                        }
                        
                        if let Some(rssi) = dict.get("RSSI").and_then(|v| v.as_signed_integer()) {
                            beacon.signal = Some(rssi as i32);
                        }
                        
                        if let Some(channel) = dict.get("CHANNEL").and_then(|v| v.as_unsigned_integer()) {
                            beacon.channel = Some(channel as u32);
                        }
                        
                        // Calculate frequency from channel
                        beacon.frequency = beacon.channel.map(|ch| {
                            if ch <= 14 { 2407 + ch * 5 }
                            else { 5000 + ch * 5 }
                        });
                        
                        // IE data is available in airport output
                        beacon.ie_data = dict.get("IE").and_then(|v| v.as_data()).map(|d| d.to_vec());
                        
                        networks.push(beacon);
                    }
                }
            }
            
            Ok(networks)
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            let _ = xml;
            Err(ScanError::NotAvailable("airport".to_string()))
        }
    }
}

impl Scanner for AirportScanner {
    fn name(&self) -> &'static str {
        "airport"
    }
    
    fn description(&self) -> &'static str {
        "macOS Airport CLI scanner (may not work on macOS 26+)"
    }
    
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
        if !self.check_available() {
            return Err(ScanError::NotAvailable(
                "airport tool not found at expected path".to_string()
            ));
        }
        
        let xml = self.run_scan()?;
        self.parse_xml(&xml)
    }
    
    fn current(&self) -> Result<Option<RawBeacon>, ScanError> {
        // Airport doesn't have a direct "current network" command
        // We would need to use networksetup or similar
        Ok(None)
    }
    
    fn platforms(&self) -> &'static [Platform] {
        &[Platform::MacOS]
    }
    
    fn capabilities(&self) -> ScannerCapabilities {
        ScannerCapabilities {
            has_ie_data: true,
            has_bssid: true,
            has_signal: true,
            has_security: true,
            app_store_compatible: true,
        }
    }
    
    fn requires_privilege(&self) -> bool {
        false
    }
    
    fn is_available(&self) -> bool {
        #[cfg(target_os = "macos")]
        { self.check_available() }
        #[cfg(not(target_os = "macos"))]
        { false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scanner_info() {
        let scanner = AirportScanner::new();
        assert_eq!(scanner.name(), "airport");
        assert!(scanner.capabilities().has_ie_data);
        assert!(!scanner.requires_privilege());
    }
    
    #[test]
    #[cfg(target_os = "macos")]
    fn test_availability() {
        let scanner = AirportScanner::new();
        // Just check it doesn't panic
        let _ = scanner.is_available();
    }
}
