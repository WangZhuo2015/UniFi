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

/// Check if running on macOS 26 (Tahoe) or later where Apple80211 is removed
#[cfg(target_os = "macos")]
fn is_macos_26_or_later() -> bool {
    use std::process::Command;

    // Get macOS version
    let output = Command::new("sw_vers")
        .arg("-productVersion")
        .output();

    match output {
        Ok(o) => {
            let version = String::from_utf8_lossy(&o.stdout).trim().to_string();
            // Parse major version (26.x.x -> 26)
            if let Some(major) = version.split('.').next() {
                if let Ok(v) = major.parse::<u32>() {
                    return v >= 26;
                }
            }
            false
        }
        Err(_) => false,
    }
}

#[cfg(not(target_os = "macos"))]
fn is_macos_26_or_later() -> bool {
    false
}

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
                // On macOS 26+, Airport is not available - use CoreWLAN
                if is_macos_26_or_later() {
                    Box::new(corewlan::CoreWlanScanner::new())
                } else {
                    // On older macOS, prefer Airport for full IE data, fall back to CoreWLAN
                    let airport = airport::AirportScanner::new();
                    if airport.is_available() {
                        Box::new(airport)
                    } else {
                        Box::new(corewlan::CoreWlanScanner::new())
                    }
                }
            }
            ScannerMode::CoreWLAN => Box::new(corewlan::CoreWlanScanner::new()),
            ScannerMode::Airport => {
                // On macOS 26+, this will return NotSupported error
                Box::new(airport::AirportScanner::new())
            }
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
        // CoreWLAN is always available on macOS
        result.push(("CoreWLAN", true, false));

        // Airport availability depends on macOS version
        let airport_available = !is_macos_26_or_later() && {
            let airport = airport::AirportScanner::new();
            airport.is_available()
        };
        result.push(("Airport", airport_available, false));

        // Libpcap requires root and has limitations on macOS 26+
        let libpcap = libpcap::LibpcapScanner::new();
        let libpcap_available = libpcap.is_available();
        result.push(("Libpcap", libpcap_available, true));
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
