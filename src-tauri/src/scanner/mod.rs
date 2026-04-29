//! WiFi Scanner - Platform Abstraction
//!
//! One trait, multiple implementations. Simple.

use crate::types::{Network, RawBeacon, ScanError};

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod channel;

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

#[cfg(all(target_os = "linux", feature = "libpcap"))]
mod libpcap;

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
    /// Libpcap scanner (macOS, requires root)
    #[cfg(target_os = "macos")]
    Libpcap,
    /// Libpcap scanner (Linux, requires root and libpcap feature)
    #[cfg(all(target_os = "linux", feature = "libpcap"))]
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

/// Parse a scanner name into the current platform's supported scanner mode.
pub fn parse_scanner_mode(name: &str) -> ScannerMode {
    let normalized = name.trim().to_ascii_lowercase();

    #[cfg(target_os = "macos")]
    {
        match normalized.as_str() {
            "corewlan" => ScannerMode::CoreWLAN,
            "airport" => ScannerMode::Airport,
            "libpcap" => ScannerMode::Libpcap,
            _ => ScannerMode::Default,
        }
    }

    #[cfg(target_os = "linux")]
    {
        #[cfg(feature = "libpcap")]
        match normalized.as_str() {
            "libpcap" => ScannerMode::Libpcap,
            _ => ScannerMode::Default,
        }

        #[cfg(not(feature = "libpcap"))]
        {
            let _ = normalized;
            ScannerMode::Default
        }
    }

    #[cfg(target_os = "windows")]
    {
        let _ = normalized;
        ScannerMode::Default
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = normalized;
        ScannerMode::Default
    }
}

/// Get scanner with specific mode.
pub fn get_scanner_with_mode(_mode: ScannerMode) -> Box<dyn Scanner> {
    #[cfg(target_os = "macos")]
    {
        match _mode {
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
        #[cfg(feature = "libpcap")]
        match _mode {
            ScannerMode::Default => Box::new(nl80211::Nl80211Scanner::new()),
            ScannerMode::Libpcap => Box::new(libpcap::LibpcapScanner::new()),
        }

        #[cfg(not(feature = "libpcap"))]
        {
            Box::new(nl80211::Nl80211Scanner::new())
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

        #[cfg(feature = "libpcap")]
        {
            let libpcap = libpcap::LibpcapScanner::new();
            result.push(("Libpcap", libpcap.is_available(), true));
        }
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
