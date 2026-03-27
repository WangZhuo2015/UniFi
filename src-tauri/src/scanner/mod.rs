//! WiFi Scanner - Platform Abstraction
//!
//! One trait, multiple implementations. Simple.

use crate::types::{Network, RawBeacon, ScanError};

#[cfg(target_os = "macos")]
mod airport;

#[cfg(target_os = "macos")]
mod corewlan;

#[cfg(target_os = "macos")]
mod libpcap;

#[cfg(target_os = "windows")]
mod wlanapi;

#[cfg(target_os = "linux")]
mod nl80211;

/// Scanner mode selection
#[derive(Clone, Copy, Debug, Default)]
pub enum ScannerMode {
    /// Default scanner for platform
    #[default]
    Default,
    /// CoreWLAN scanner (macOS only, App Store compatible)
    #[cfg(target_os = "macos")]
    CoreWLAN,
    /// Airport scanner (macOS only, legacy)
    #[cfg(target_os = "macos")]
    Airport,
    /// Libpcap scanner (macOS/Linux, requires root)
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    Libpcap,
}

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

    /// Is this scanner available on this system?
    fn is_available(&self) -> bool {
        true
    }
}

/// Get the default scanner for current platform.
pub fn get_scanner() -> Box<dyn Scanner> {
    get_scanner_with_mode(ScannerMode::Default)
}

/// Get scanner with specific mode.
pub fn get_scanner_with_mode(mode: ScannerMode) -> Box<dyn Scanner> {
    #[cfg(target_os = "macos")]
    {
        match mode {
            ScannerMode::Default => {
                // Prefer Airport for full IE data, fall back to CoreWLAN
                let airport = airport::AirportScanner::new();
                if airport.is_available() {
                    Box::new(airport)
                } else {
                    Box::new(corewlan::CoreWlanScanner::new())
                }
            }
            ScannerMode::CoreWLAN => Box::new(corewlan::CoreWlanScanner::new()),
            ScannerMode::Airport => Box::new(airport::AirportScanner::new()),
            ScannerMode::Libpcap => Box::new(libpcap::LibpcapScanner::new()),
        }
    }

    #[cfg(target_os = "windows")]
    { Box::new(wlanapi::WlanApiScanner::new()) }

    #[cfg(target_os = "linux")]
    {
        match mode {
            ScannerMode::Default => Box::new(nl80211::Nl80211Scanner::new()),
            ScannerMode::Libpcap => Box::new(libpcap::LibpcapScanner::new()),
            _ => Box::new(nl80211::Nl80211Scanner::new()),
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    { compile_error!("Unsupported platform") }
}

/// List available scanners for current platform.
pub fn list_scanners() -> Vec<(&'static str, bool, bool)> {
    let mut result = Vec::new();

    #[cfg(target_os = "macos")]
    {
        let corewlan = corewlan::CoreWlanScanner::new();
        result.push(("CoreWLAN", corewlan.is_available(), false));

        let airport = airport::AirportScanner::new();
        result.push(("Airport", airport.is_available(), false));

        let libpcap = libpcap::LibpcapScanner::new();
        result.push(("Libpcap", libpcap.is_available(), true));
    }

    #[cfg(target_os = "windows")]
    {
        result.push(("WlanAPI", true, false));
    }

    #[cfg(target_os = "linux")]
    {
        result.push(("nl80211", true, false));
        result.push(("Libpcap", true, true));
    }

    result
}

/// Scan networks using default scanner and parse results.
pub fn scan_networks() -> Result<Vec<Network>, ScanError> {
    let scanner = get_scanner();
    let beacons = scanner.scan()?;
    Ok(beacons.iter().map(crate::parser::parse_beacon).collect())
}

/// Scan networks with specific scanner mode.
pub fn scan_networks_with_mode(mode: ScannerMode) -> Result<Vec<Network>, ScanError> {
    let scanner = get_scanner_with_mode(mode);
    let beacons = scanner.scan()?;
    Ok(beacons.iter().map(crate::parser::parse_beacon).collect())
}
