//! Linux nl80211 Scanner Plugin
//!
//! Uses Linux nl80211 netlink interface for WiFi scanning.
//! Provides comprehensive WiFi information on Linux systems.

use scanner_core::{Platform, RawBeacon, ScanError, Scanner, ScannerCapabilities};

/// Linux nl80211 scanner
pub struct Nl80211Scanner {
    interface: Option<String>,
}

impl Default for Nl80211Scanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Nl80211Scanner {
    /// Create a new nl80211 scanner
    pub fn new() -> Self {
        Self { interface: None }
    }
    
    /// Set the interface to use
    pub fn with_interface(mut self, interface: impl Into<String>) -> Self {
        self.interface = Some(interface.into());
        self
    }
    
    #[cfg(target_os = "linux")]
    fn do_scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
        // TODO: Implement using netlink-packet-route
        // This is a placeholder implementation
        
        // For now, we can use the iw CLI tool as a fallback
        use std::process::Command;
        
        let interface = self.interface.as_ref()
            .ok_or_else(|| ScanError::Config("No interface specified".to_string()))?;
        
        let output = Command::new("iw")
            .arg("dev")
            .arg(interface)
            .arg("scan")
            .output()
            .map_err(|e| ScanError::System(format!("Failed to run iw: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Operation not permitted") {
                return Err(ScanError::PermissionDenied(
                    "nl80211 scan requires root privileges or CAP_NET_ADMIN".to_string()
                ));
            }
            return Err(ScanError::System(format!("iw scan failed: {}", stderr)));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_iw_output(&stdout)
    }
    
    #[cfg(target_os = "linux")]
    fn parse_iw_output(&self, output: &str) -> Result<Vec<RawBeacon>, ScanError> {
        let mut beacons = Vec::new();
        let mut current_beacon: Option<RawBeacon> = None;
        
        for line in output.lines() {
            let line = line.trim();
            
            if line.starts_with("BSS ") {
                // Save previous beacon
                if let Some(beacon) = current_beacon.take() {
                    beacons.push(beacon);
                }
                
                // Parse BSSID
                let bssid = line.strip_prefix("BSS ")
                    .and_then(|s| s.split('(').next())
                    .map(|s| s.trim().to_string());
                
                current_beacon = Some(RawBeacon {
                    bssid,
                    ..RawBeacon::new()
                });
            } else if let Some(ref mut beacon) = current_beacon {
                if line.starts_with("SSID:") {
                    beacon.ssid = Some(line.strip_prefix("SSID:").unwrap_or("").trim().to_string());
                } else if line.starts_with("signal:") {
                    let signal_str = line.strip_prefix("signal:").unwrap_or("").trim();
                    let signal = signal_str.split_whitespace().next()
                        .and_then(|s| s.parse::<i32>().ok());
                    beacon.signal = signal;
                } else if line.starts_with("DS Parameter set:") {
                    let channel_str = line.strip_prefix("DS Parameter set:").unwrap_or("").trim();
                    let channel = channel_str.split_whitespace().next()
                        .and_then(|s| s.parse::<u32>().ok());
                    beacon.channel = channel;
                    if let Some(ch) = channel {
                        beacon.frequency = if ch <= 14 {
                            Some(2407 + ch * 5)
                        } else {
                            Some(5000 + ch * 5)
                        };
                    }
                } else if line.contains("primary channel:") {
                    let channel = line.split(':')
                        .nth(1)
                        .and_then(|s| s.trim().parse::<u32>().ok());
                    beacon.channel = channel;
                }
            }
        }
        
        // Don't forget the last beacon
        if let Some(beacon) = current_beacon {
            beacons.push(beacon);
        }
        
        Ok(beacons)
    }
}

impl Scanner for Nl80211Scanner {
    fn name(&self) -> &'static str {
        "nl80211"
    }
    
    fn description(&self) -> &'static str {
        "Linux nl80211 netlink scanner"
    }
    
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
        #[cfg(target_os = "linux")]
        {
            self.do_scan()
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            Err(ScanError::NotAvailable("nl80211".to_string()))
        }
    }
    
    fn current(&self) -> Result<Option<RawBeacon>, ScanError> {
        // TODO: Use NL80211_CMD_GET_INTERFACE to get current connection
        Ok(None)
    }
    
    fn platforms(&self) -> &'static [Platform] {
        &[Platform::Linux]
    }
    
    fn capabilities(&self) -> ScannerCapabilities {
        ScannerCapabilities {
            has_ie_data: true,   // Can be parsed from scan results
            has_bssid: true,
            has_signal: true,
            has_security: true,
            app_store_compatible: true,
        }
    }
    
    fn requires_privilege(&self) -> bool {
        false  // Can work with CAP_NET_ADMIN or specific group membership
    }
    
    fn is_available(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            // Check if nl80211 is available
            std::path::Path::new("/proc/net/nl80211").exists() ||
            std::process::Command::new("iw").arg("list").output().is_ok()
        }
        
        #[cfg(not(target_os = "linux"))]
        { false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scanner_info() {
        let scanner = Nl80211Scanner::new();
        assert_eq!(scanner.name(), "nl80211");
        assert!(scanner.capabilities().has_ie_data);
    }
    
    #[test]
    fn test_builder() {
        let scanner = Nl80211Scanner::new()
            .with_interface("wlan0");
        
        assert_eq!(scanner.interface, Some("wlan0".to_string()));
    }
}
