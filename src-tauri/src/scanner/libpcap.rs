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
//! - Channel hopping for better coverage
//! - Works on macOS and Linux
//!
//! ## Requirements
//! - Root/sudo privileges (for monitor mode)
//! - Monitor mode capable WiFi interface

use crate::scanner::{RawBeacon, Scanner};
use crate::types::{Band, ScanError};

use std::collections::HashMap;
use std::io::{self, Write};
use std::process::Command;
use std::time::{Duration, Instant};

/// Scan duration in seconds
const SCAN_DURATION_SECS: u64 = 20;

/// Time to spend on each channel (ms) - increased for better capture
const CHANNEL_DWELL_MS: u64 = 1000;

/// Channels to scan
#[cfg(target_os = "macos")]
const SCAN_CHANNELS_2GHZ: &[u8] = &[1, 6, 11];  // Use most common channels
#[cfg(target_os = "macos")]
const SCAN_CHANNELS_5GHZ: &[u8] = &[36, 44, 48, 52, 60, 100, 112, 124, 136, 149, 157, 165];

#[cfg(target_os = "linux")]
const SCAN_CHANNELS_2GHZ: &[u8] = &[1, 6, 11];
#[cfg(target_os = "linux")]
const SCAN_CHANNELS_5GHZ: &[u8] = &[36, 44, 52, 60, 100, 108, 116, 124, 132, 140, 149, 157, 165];

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
        if !is_root() {
            return (
                false,
                None,
                Some("Requires root/sudo privileges. Run with: sudo unifi-cli scan --scanner libpcap".to_string()),
            );
        }

        match find_wifi_interface() {
            Some(iface) => (true, Some(iface), None),
            None => (
                false,
                None,
                Some("No WiFi interface found".to_string()),
            ),
        }
    }

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
        capture_beacons_with_channel_hopping(interface)
    }

    fn current(&self) -> Result<Option<RawBeacon>, ScanError> {
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

fn find_wifi_interface() -> Option<String> {
    let devices = pcap::Device::list().ok()?;

    #[cfg(target_os = "macos")]
    {
        for device in devices {
            if device.name.starts_with("en") {
                if is_macos_wifi_interface(&device.name) {
                    return Some(device.name);
                }
            }
        }
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
    name == "en0"
}

/// Set WiFi channel on macOS using airport
#[cfg(target_os = "macos")]
fn set_channel(_interface: &str, channel: u8) -> bool {
    let airport_path = "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport";

    // Use -c<channel> syntax (no space)
    let result = Command::new(airport_path)
        .arg(format!("-c{}", channel))
        .output();

    match result {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

#[cfg(target_os = "linux")]
fn set_channel(interface: &str, channel: u8) -> bool {
    let result = Command::new("iw")
        .args(["dev", interface, "set", "channel", &channel.to_string()])
        .output();

    match result {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Capture beacons with channel hopping
fn capture_beacons_with_channel_hopping(interface: &str) -> Result<Vec<RawBeacon>, ScanError> {
    // Focus on most common channels
    let channels: Vec<u8> = vec![
        1, 6, 11,           // 2.4 GHz
        36, 44, 48,         // 5 GHz UNII-1
        52, 60,             // 5 GHz UNII-2
        149, 157, 165       // 5 GHz UNII-3 (skip DFS channels 100-144)
    ];

    println!("Scanning {} channels on {}...", channels.len(), interface);

    let mut networks: HashMap<[u8; 6], RawBeacon> = HashMap::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let airport_path = "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport";

    // Disassociate from current network
    let _ = Command::new(airport_path).args(["-z"]).output();
    std::thread::sleep(Duration::from_millis(300));

    let start = Instant::now();
    let total_duration = Duration::from_secs(SCAN_DURATION_SECS);
    let mut total_packets = 0;

    for &ch in &channels {
        if start.elapsed() >= total_duration {
            break;
        }

        // Set channel using airport
        if !set_channel(interface, ch) {
            continue;
        }

        std::thread::sleep(Duration::from_millis(100));

        // Open capture for this channel
        let mut cap = match pcap::Capture::from_device(interface)
            .map_err(|e| ScanError::CommandFailed(format!("Failed to open device: {}", e)))?
            .promisc(true)
            .rfmon(true)
            .timeout(CHANNEL_DWELL_MS as i32)
            .immediate_mode(true)
            .snaplen(4096)
            .open()
        {
            Ok(cap) => cap,
            Err(_) => continue,
        };

        let _ = cap.filter("wlan type mgt subtype beacon", true);

        let channel_start = Instant::now();
        let dwell = Duration::from_millis(CHANNEL_DWELL_MS);
        let mut channel_packets = 0;

        while channel_start.elapsed() < dwell {
            match cap.next_packet() {
                Ok(packet) => {
                    channel_packets += 1;
                    total_packets += 1;

                    if let Some((beacon, _signal)) = parse_packet(packet.data, now) {
                        networks.entry(beacon.bssid)
                            .and_modify(|existing| {
                                if beacon.ie_data.len() > existing.ie_data.len() {
                                    *existing = beacon.clone();
                                }
                            })
                            .or_insert(beacon);
                    }
                }
                Err(pcap::Error::TimeoutExpired) => break,
                Err(_) => continue,
            }
        }

        println!("  Ch{}: {} packets, {} networks", ch, channel_packets, networks.len());
        drop(cap);
    }

    // Re-associate with network
    let _ = Command::new(airport_path).args(["-a"]).output();

    println!("Captured {} unique networks ({} packets)", networks.len(), total_packets);
    Ok(networks.into_values().collect())
}

/// Parse a captured packet
fn parse_packet(data: &[u8], timestamp: u64) -> Option<(RawBeacon, i16)> {
    if data.is_empty() {
        return None;
    }

    let (radio_tap_len, signal_dbm) = parse_radiotap_header(data);

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
        return (0, -50);
    }

    let version = data[0];
    if version != 0 {
        return (0, -50);
    }

    let header_len = u16::from_le_bytes([data[2], data[3]]) as usize;
    if header_len > data.len() || header_len < 8 {
        return (0, -50);
    }

    let present = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

    // Print debug info once
    static DEBUG_ONCE: std::sync::Once = std::sync::Once::new();
    DEBUG_ONCE.call_once(|| {
        print!("\n[RADIOTAP] header_len={} present=0x{:08X}\n", header_len, present);
        print!("[RADIOTAP] bits: ");
        for bit in 0..14 {
            if present & (1 << bit) != 0 {
                print!("{} ", bit);
            }
        }
        print!("\n");
        let _ = io::stdout().flush();
    });

    let mut offset = 8;
    let mut signal: i16 = -50;

    for bit in 0..32 {
        if (present & (1 << bit)) == 0 {
            continue;
        }

        if offset >= header_len {
            break;
        }

        match bit {
            0 => { // TSFT (8 bytes)
                offset = align(offset, 8);
                offset += 8;
            }
            1 => { // Flags (1 byte)
                offset += 1;
            }
            2 => { // Rate (1 byte)
                offset += 1;
            }
            3 => { // Channel (2 bytes frequency + 2 bytes flags = 4 bytes)
                offset = align(offset, 2);
                offset += 4;
            }
            4 => { // FHSS (2 bytes)
                offset = align(offset, 2);
                offset += 2;
            }
            5 => { // dBm Antenna Signal
                if offset < data.len() {
                    signal = data[offset] as i8 as i16;
                }
                offset += 1;
            }
            6 => { // dBm Antenna Noise
                offset += 1;
            }
            7 | 8 | 9 => { // Lock Quality, TX Attenuation, dB TX Attenuation
                offset = align(offset, 2);
                offset += 2;
            }
            10 | 11 => { // dBm TX Power, Antenna
                offset += 1;
            }
            12 => { // dB Antenna Signal
                if signal == -50 && offset < data.len() {
                    signal = data[offset] as i16;
                }
                offset += 1;
            }
            13 => { // dB Antenna Noise
                offset += 1;
            }
            14 | 15 => { // Rx/Tx Flags
                offset = align(offset, 2);
                offset += 2;
            }
            _ => {
                offset += 4;
            }
        }
    }

    (header_len, signal)
}

fn align(offset: usize, alignment: usize) -> usize {
    (offset + alignment - 1) & !(alignment - 1)
}

/// Parse an 802.11 beacon frame
fn parse_beacon_frame(data: &[u8], timestamp: u64, signal_dbm: i16) -> Option<RawBeacon> {
    if data.len() < 24 + 12 {
        return None;
    }

    let frame_control = u16::from_le_bytes([data[0], data[1]]);
    let frame_type = (frame_control >> 2) & 0x3;
    let frame_subtype = (frame_control >> 4) & 0xF;

    if frame_type != 0 || frame_subtype != 8 {
        return None;
    }

    let bssid = [
        data[16], data[17], data[18], data[19], data[20], data[21]
    ];

    let beacon_interval = u16::from_le_bytes([data[32], data[33]]);

    let ie_data = data[36..].to_vec();

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
        link_rates: None,
        local_adapter: None,
    })
}

/// Parse IE data from beacon frame to extract SSID and channel
fn parse_ie_data(data: &[u8]) -> (Option<Vec<u8>>, u8) {
    let mut ssid = None;
    let mut channel = 0;
    let mut has_ht = false;

    let mut pos = 0;
    while pos + 1 < data.len() {
        let id = data[pos];
        let len = data[pos + 1] as usize;

        if pos + 2 + len > data.len() {
            break;
        }

        match id {
            0 => {
                if len > 0 && len <= 32 {
                    ssid = Some(data[pos + 2..pos + 2 + len].to_vec());
                }
            }
            3 => {
                if len >= 1 && !has_ht {
                    channel = data[pos + 2];
                }
            }
            61 => {
                if len >= 1 {
                    let ht_channel = data[pos + 2];
                    if channel == 0 || ht_channel > 14 {
                        channel = ht_channel;
                        has_ht = true;
                    }
                }
            }
            192 => {
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
