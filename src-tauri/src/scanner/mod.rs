//! WiFi Scanner - Platform Abstraction
//!
//! One trait, multiple implementations. Simple.

use crate::types::{Network, RawBeacon, ScanError};

#[cfg(target_os = "macos")]
mod airport;

#[cfg(target_os = "macos")]
mod libpcap;

#[cfg(target_os = "windows")]
mod wlanapi;

#[cfg(target_os = "linux")]
mod nl80211;

/// Scanner trait - the only interface for WiFi scanning.
pub trait Scanner: Send + Sync {
    /// Scan and return raw beacon data.
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError>;
    
    /// Get current connected network.
    fn current(&self) -> Result<Option<RawBeacon>, ScanError>;
    
    /// Scanner name for display.
    fn name(&self) -> &'static str;
    
    /// Does this scanner require root/admin?
    fn requires_privilege(&self) -> bool {
        false
    }
}

/// Get the default scanner for current platform.
pub fn get_scanner() -> Box<dyn Scanner> {
    #[cfg(target_os = "macos")]
    { Box::new(airport::AirportScanner::new()) }
    
    #[cfg(target_os = "windows")]
    { Box::new(wlanapi::WlanApiScanner::new()) }
    
    #[cfg(target_os = "linux")]
    { Box::new(nl80211::Nl80211Scanner::new()) }
    
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    { compile_error!("Unsupported platform") }
}

/// Scan networks using default scanner and parse results.
pub fn scan_networks() -> Result<Vec<Network>, ScanError> {
    let scanner = get_scanner();
    let beacons = scanner.scan()?;
    Ok(beacons.iter().map(crate::parser::parse_beacon).collect())
}
