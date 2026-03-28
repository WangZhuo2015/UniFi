//! macOS/Linux Libpcap Scanner
//!
//! Captures WiFi beacon frames using libpcap.
//! This scanner requires root privileges and provides full IE data.
//! Cannot be used on App Store - provided as a plugin.
//!
//! ## Usage
//! ```bash
//! sudo unifi-cli scan --scanner libpcap
//! ```
//!
//! ## Features
//! - Captures raw 802.11 beacon frames
//! - Provides complete IE data for WiFi standard detection (4/5/6/7)
//! - Parses radiotap header for signal strength
//! - Works on macOS and Linux
//!
//! ## Requirements
//! - Root/sudo privileges (for monitor mode)
//! - Monitor mode capable WiFi interface
//!
//! ## macOS Notes
//! On macOS, the scanner uses the built-in airport utility to assist with
//! monitor mode setup. You may need to grant additional permissions.

use crate::scanner::{RawBeacon, Scanner};
use crate::types::{Band, ScanError};

use std::collections::HashMap;
use std::process::Command;
use std::time::{Duration, Instant};

/// Scan duration in seconds
const SCAN_DURATION_SECS: u64 = 5;

pub struct LibpcapScanner {
    available: bool,
    interface: Option<String>,
    unavailable_reason: Option<String>,
}

impl LibpcapScanner {
    pub fn new() -> Self {
        let (available, interface, reason) = Self::check_availability();
        Self {
            available,
            interface,
            unavailable_reason: reason,
        }
    }

    fn check_availability() -> (bool, Option<String>, Option<String>) {
        // Must run as root
        if !is_root() {
            return (
                false,
                None,
                Some("Requires root/sudo privileges. Run with: sudo unifi-cli scan --scanner libpcap".to_string()),
            );
        }

        // Find WiFi interface
        match find_wifi_interface() {
            Some(iface) => (true, Some(iface), None),
            None => (
                false,
                None,
                Some("No WiFi interface found".to_string()),
            ),
        }
    }

    /// Get the reason why scanner is unavailable
    pub fn unavailable_reason(&self) -> Option<&str> {
        self.unavailable_reason.as_deref()
    }
}

impl Default for LibpcapScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Scanner for LibpcapScanner {
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
        if !self.available {
            let reason = self.unavailable_reason.as_deref()
                .unwrap_or("Permission denied");
            return Err(ScanError::CommandFailed(reason.to_string()));
        }

        let interface = self.interface.as_ref().ok_or(ScanError::NoInterface)?;
        capture_beacons(interface)
    }

    fn current(&self) -> Result<Option<RawBeacon>, ScanError> {
        // Libpcap can only capture beacon frames, not query current connection
        // Use Airport or CoreWLAN for this
        Err(ScanError::NotSupported)
    }

    fn name(&self) -> &'static str {
        "Libpcap Beacon Capture"
    }

    fn requires_privilege(&self) -> bool {
        true
    }

    fn is_available(&self) -> bool {
        self.available
    }
}

fn is_root() -> bool {
    #[cfg(unix)]
    unsafe {
        libc::getuid() == 0
    }
    #[cfg(not(unix))]
    false
}

/// Find the WiFi interface name
fn find_wifi_interface() -> Option<String> {
    let devices = pcap::Device::list().ok()?;

    #[cfg(target_os = "macos")]
    {
        // On macOS, look for en0, en1 etc.
        // Also check if it's actually a WiFi interface
        for device in devices {
            if device.name.starts_with("en") {
                // Verify it's WiFi by checking if airport can use it
                if is_macos_wifi_interface(&device.name) {
                    return Some(device.name);
                }
            }
        }
        // Default to en0 if nothing found
        Some("en0".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        for device in devices {
            if device.name.starts_with("wlan") || device.name.starts_with("wlp") {
                return Some(device.name);
            }
        }
        None
    }
}

#[cfg(target_os = "macos")]
fn is_macos_wifi_interface(name: &str) -> bool {
    // Check using networksetup
    let output = Command::new("networksetup")
        .args(["-listallhardwareports"])
        .output()
        .ok();

    if let Some(output) = output {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut is_wifi = false;
        for line in stdout.lines() {
            if line.contains("Wi-Fi") || line.contains("AirPort") {
                is_wifi = true;
            }
            if is_wifi && line.contains("Device:") {
                let dev = line.split(':').nth(1).map(|s| s.trim());
                if dev == Some(name) {
                    return true;
                }
                is_wifi = false;
            }
        }
    }
    // Default: en0 is usually WiFi
    name == "en0"
}

/// Enable monitor mode on macOS
#[cfg(target_os = "macos")]
fn enable_monitor_mode(interface: &str) -> Result<(), ScanError> {
    // Use airport to disassociate and enable monitor mode
    let airport_path = "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport";

    // First, disassociate from current network
    let _ = Command::new(airport_path)
        .args(["-z"])
        .output();

    // Note: Full monitor mode requires channel hopping which is complex
    // For basic beacon capture, we'll try pcap's built-in monitor mode support
    Ok(())
}

/// Disable monitor mode on macOS
#[cfg(target_os = "macos")]
fn disable_monitor_mode(interface: &str) {
    let airport_path = "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport";

    // Re-associate with network
    let _ = Command::new(airport_path)
        .args(["-a"])
        .output();
}

fn capture_beacons(interface: &str) -> Result<Vec<RawBeacon>, ScanError> {
    #[cfg(target_os = "macos")]
    enable_monitor_mode(interface)?;

    let result = capture_beacons_inner(interface);

    #[cfg(target_os = "macos")]
    disable_monitor_mode(interface);

    result
}

fn capture_beacons_inner(interface: &str) -> Result<Vec<RawBeacon>, ScanError> {
    // Open device for capture
    let mut cap = pcap::Capture::from_device(interface)
        .map_err(|e| ScanError::CommandFailed(format!("Failed to open device {}: {}", interface, e)))?
        .promisc(true)
        .rfmon(true)  // Monitor mode
        .timeout(1000)
        .immediate_mode(true)
        .snaplen(65535)  // Capture full frames
        .open()
        .map_err(|e| ScanError::CommandFailed(format!("Failed to activate capture: {}", e)))?;

    // Set BPF filter for beacon frames
    // wlan type mgt subtype beacon
    cap.filter("wlan type mgt subtype beacon", true)
        .map_err(|e| ScanError::CommandFailed(format!("Failed to set filter: {}", e)))?;

    let start = Instant::now();
    let timeout = Duration::from_secs(SCAN_DURATION_SECS);

    let mut networks: HashMap<[u8; 6], RawBeacon> = HashMap::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    println!("Capturing beacon frames on {} for {} seconds...", interface, SCAN_DURATION_SECS);

    while start.elapsed() < timeout {
        match cap.next_packet() {
            Ok(packet) => {
                if let Some((beacon, _signal)) = parse_packet(packet.data, now) {
                    networks.entry(beacon.bssid)
                        .and_modify(|existing| {
                            // Keep the one with stronger signal or more IE data
                            if beacon.ie_data.len() > existing.ie_data.len() {
                                *existing = beacon.clone();
                            }
                        })
                        .or_insert(beacon);
                }
            }
            Err(pcap::Error::TimeoutExpired) => {
                // Timeout is normal, continue
                continue;
            }
            Err(e) => {
                eprintln!("Warning: Capture error: {}", e);
                // Don't break on errors, try to continue
            }
        }
    }

    println!("Captured {} unique networks", networks.len());

    Ok(networks.into_values().collect())
}

/// Parse a captured packet, handling radiotap header if present
fn parse_packet(data: &[u8], timestamp: u64) -> Option<(RawBeacon, i16)> {
    if data.is_empty() {
        return None;
    }

    // Check for radiotap header (common on macOS and Linux)
    // Radiotap header starts with a little-endian 16-bit version (usually 0x00)
    // followed by 16-bit pad, then 32-bit presence flags
    let (radio_tap_len, signal_dbm) = parse_radiotap_header(data);

    // Skip radiotap header to get to 802.11 frame
    let frame_data = if radio_tap_len > 0 && radio_tap_len < data.len() {
        &data[radio_tap_len..]
    } else {
        data
    };

    let beacon = parse_beacon_frame(frame_data, timestamp, signal_dbm)?;
    Some((beacon, signal_dbm))
}

/// Parse radiotap header to extract signal strength
/// Returns (header_length, signal_dbm)
fn parse_radiotap_header(data: &[u8]) -> (usize, i16) {
    if data.len() < 8 {
        return (0, -50); // Default signal
    }

    // Radiotap header:
    // 2 bytes: version (should be 0) + pad
    // 2 bytes: header length (little-endian)
    // 4 bytes: present flags

    let version = data[0];
    if version != 0 {
        return (0, -50);
    }

    let header_len = u16::from_le_bytes([data[2], data[3]]) as usize;
    if header_len > data.len() || header_len < 8 {
        return (0, -50);
    }

    // Present flags indicate which fields are present
    let present = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

    // Bit 5 (0x20) = dBm antenna signal
    // We need to find the signal field offset
    let mut offset = 8;
    let mut signal: i16 = -50; // Default

    // Check each present bit in order
    // Bit 0: TSFT (8 bytes)
    if (present & 0x01) != 0 {
        offset += 8;
    }
    // Bit 1: Flags (1 byte)
    if (present & 0x02) != 0 {
        offset += 1;
    }
    // Bit 2: Rate (1 byte)
    if (present & 0x04) != 0 {
        offset += 1;
    }
    // Bit 3: Channel (4 bytes + 2 bytes = 8 bytes total with flags)
    if (present & 0x08) != 0 {
        offset += 4;
        // Check channel flags for more data
        if offset + 2 <= header_len {
            let chan_flags = u16::from_le_bytes([data[offset], data[offset + 1]]);
            offset += 2;
            // If bit 0 is set, there's an additional 2 bytes
            if (chan_flags & 0x01) != 0 {
                offset += 2;
            }
        }
    }
    // Bit 4: FHSS (2 bytes)
    if (present & 0x10) != 0 {
        offset += 2;
    }
    // Bit 5: dBm Antenna Signal (1 byte)
    if (present & 0x20) != 0 {
        if offset < header_len && offset < data.len() {
            signal = data[offset] as i8 as i16; // Signed byte to i16
        }
        offset += 1;
    }
    // Bit 6: dBm Antenna Noise (1 byte)
    if (present & 0x40) != 0 {
        offset += 1;
    }
    // Bit 7: Lock Quality (2 bytes)
    if (present & 0x80) != 0 {
        offset += 2;
    }
    // Bit 8: TX Attenuation (2 bytes)
    if (present & 0x100) != 0 {
        offset += 2;
    }
    // Bit 9: dB TX Attenuation (2 bytes)
    if (present & 0x200) != 0 {
        offset += 2;
    }
    // Bit 10: dBm TX Power (1 byte)
    if (present & 0x400) != 0 {
        offset += 1;
    }
    // Bit 11: Antenna (1 byte)
    if (present & 0x800) != 0 {
        offset += 1;
    }
    // Bit 12: dB Antenna Signal (1 byte)
    if (present & 0x1000) != 0 {
        if offset < header_len && offset < data.len() {
            // This is unsigned, prefer dBm signal if available
            if signal == -50 {
                signal = data[offset] as i16;
            }
        }
        offset += 1;
    }
    // Bit 13: dB Antenna Noise (1 byte)
    if (present & 0x2000) != 0 {
        offset += 1;
    }

    (header_len, signal)
}

/// Parse an 802.11 beacon frame
fn parse_beacon_frame(data: &[u8], timestamp: u64, signal_dbm: i16) -> Option<RawBeacon> {
    // 802.11 Management Frame structure:
    // Frame Control: 2 bytes
    // Duration: 2 bytes
    // Address 1 (DA): 6 bytes
    // Address 2 (SA): 6 bytes
    // Address 3 (BSSID): 6 bytes
    // Sequence Control: 2 bytes
    // -- Fixed fields for beacon --
    // Timestamp: 8 bytes
    // Beacon Interval: 2 bytes
    // Capability: 2 bytes
    // -- Variable fields (IEs) --
    // Tagged Parameters: variable

    if data.len() < 24 + 12 { // 24 byte MAC header + 12 byte fixed beacon fields
        return None;
    }

    // Verify it's a beacon frame
    let frame_control = u16::from_le_bytes([data[0], data[1]]);
    let frame_type = (frame_control >> 2) & 0x3;
    let frame_subtype = (frame_control >> 4) & 0xF;

    // Type 0 = Management, Subtype 8 = Beacon
    if frame_type != 0 || frame_subtype != 8 {
        return None;
    }

    // Extract BSSID (Address 3 in beacon frames)
    let bssid = [
        data[16], data[17], data[18], data[19], data[20], data[21]
    ];

    // Fixed fields start at offset 24
    let _beacon_timestamp = u64::from_le_bytes([
        data[24], data[25], data[26], data[27],
        data[28], data[29], data[30], data[31]
    ]);

    let beacon_interval = u16::from_le_bytes([data[32], data[33]]);

    // IE Data starts at offset 36 (after Capability at 34-35)
    let ie_data = data[36..].to_vec();

    // Parse IE data to extract SSID, channel, etc.
    let (ssid, channel) = parse_ie_data(&ie_data);

    let band = Band::from_channel(channel);

    Some(RawBeacon {
        ssid,
        bssid,
        channel,
        band,
        signal_dbm,
        noise_dbm: -90,
        ie_data,
        beacon_interval,
        timestamp,
        uptime_ms: None,
        connected: false,
    })
}

/// Parse IE data from beacon frame to extract SSID and channel
fn parse_ie_data(data: &[u8]) -> (Option<Vec<u8>>, u8) {
    let mut ssid = None;
    let mut channel = 0;
    let mut has_ds = false; // DS Parameter Set (for 2.4GHz)
    let mut has_ht = false; // HT Operation (may have 5GHz channel)

    let mut pos = 0;
    while pos + 1 < data.len() {
        let id = data[pos];
        let len = data[pos + 1] as usize;

        if pos + 2 + len > data.len() {
            break;
        }

        match id {
            0 => {
                // SSID IE
                if len > 0 && len <= 32 {
                    ssid = Some(data[pos + 2..pos + 2 + len].to_vec());
                }
            }
            3 => {
                // DS Parameter Set (Channel) - for 2.4GHz
                if len >= 1 && !has_ht {
                    channel = data[pos + 2];
                    has_ds = true;
                }
            }
            45 => {
                // HT Capabilities - check for 5GHz
                // This indicates the AP supports HT (802.11n)
            }
            61 => {
                // HT Operation - contains primary channel
                // For 5GHz networks, this is more reliable than DS param
                if len >= 1 {
                    let ht_channel = data[pos + 2];
                    // Only use if we haven't found a channel or if this looks like 5GHz
                    if channel == 0 || ht_channel > 14 {
                        channel = ht_channel;
                        has_ht = true;
                    }
                }
            }
            192 => {
                // VHT Operation - 5GHz channel info
                if len >= 3 {
                    let vht_channel = data[pos + 2];
                    if vht_channel > 0 && vht_channel > 14 {
                        channel = vht_channel;
                    }
                }
            }
            _ => {}
        }

        pos += 2 + len;
    }

    (ssid, channel)
}
