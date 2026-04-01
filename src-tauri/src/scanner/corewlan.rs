//! macOS CoreWLAN Scanner
//!
//! Uses CoreWLAN framework for WiFi scanning.
//! This is the App Store compatible scanner.
//! Works on macOS 10.6+ and is the recommended scanner for macOS 26+.
//!
//! # Limitations
//! - BSSID returns null unless the app has Location permission
//! - No IE data available (no WiFi standard detection)
//! - For full functionality, use the Airport or Libpcap scanner
//!
//! # Permissions Required
//! - Location permission for BSSID access
//! - No special entitlements for basic scanning

use crate::scanner::{RawBeacon, Scanner};
use crate::types::{Band, ScanError};

use objc::runtime::{Class, Object, BOOL, NO};
use objc::{msg_send, sel, sel_impl};

pub struct CoreWlanScanner {
    available: bool,
}

impl CoreWlanScanner {
    pub fn new() -> Self {
        // Check if CoreWLAN is available
        let available = Class::get("CWWiFiClient").is_some();
        Self { available }
    }
}

impl Default for CoreWlanScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Scanner for CoreWlanScanner {
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
        if !self.available {
            return Err(ScanError::NotSupported);
        }

        scan_with_corewlan()
    }

    fn current(&self) -> Result<Option<RawBeacon>, ScanError> {
        if !self.available {
            return Err(ScanError::NotSupported);
        }

        get_current_network()
    }

    fn name(&self) -> &'static str {
        "macOS CoreWLAN"
    }

    fn is_available(&self) -> bool {
        self.available
    }
}

fn scan_with_corewlan() -> Result<Vec<RawBeacon>, ScanError> {
    // Get CWWiFiClient shared instance
    let client_class = match Class::get("CWWiFiClient") {
        Some(c) => c,
        None => return Err(ScanError::NotSupported),
    };

    unsafe {
        // [CWWiFiClient sharedWiFiClient]
        let client: *mut Object = msg_send![client_class, sharedWiFiClient];
        if client.is_null() {
            return Err(ScanError::NoInterface);
        }

        // [client interface]
        let interface: *mut Object = msg_send![client, interface];
        if interface.is_null() {
            return Err(ScanError::NoInterface);
        }

        // [interface scanForNetworksWithSSID:nil error:&error]
        let mut error: *mut Object = std::ptr::null_mut();
        let nil: *const () = std::ptr::null();
        let networks: *mut Object = msg_send![interface, scanForNetworksWithSSID:nil error:&mut error];

        if !error.is_null() {
            return Err(ScanError::CommandFailed("Scan failed".into()));
        }

        if networks.is_null() {
            return Ok(Vec::new());
        }

        // CoreWLAN returns NSSet, not NSArray
        // Convert to array using allObjects
        let networks_array: *mut Object = msg_send![networks, allObjects];

        // Process networks
        let count: usize = msg_send![networks_array, count];
        let mut results = Vec::with_capacity(count);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        for i in 0..count {
            let network: *mut Object = msg_send![networks_array, objectAtIndex:i];
            if let Some(beacon) = parse_cwnetwork(network, now) {
                results.push(beacon);
            }
        }

        Ok(results)
    }
}

fn get_current_network() -> Result<Option<RawBeacon>, ScanError> {
    let client_class = match Class::get("CWWiFiClient") {
        Some(c) => c,
        None => return Err(ScanError::NotSupported),
    };

    unsafe {
        let client: *mut Object = msg_send![client_class, sharedWiFiClient];
        if client.is_null() {
            return Ok(None);
        }

        let interface: *mut Object = msg_send![client, interface];
        if interface.is_null() {
            return Ok(None);
        }

        // Check if power is on
        let power_on: BOOL = msg_send![interface, powerOn];
        if power_on == NO {
            return Ok(None);
        }

        // Get current SSID directly from interface
        let ssid_nsstring: *mut Object = msg_send![interface, ssid];
        if ssid_nsstring.is_null() {
            return Ok(None);
        }

        let current_ssid = nsstring_to_string(ssid_nsstring);
        if current_ssid.is_empty() {
            return Ok(None);
        }

        // Get current RSSI and channel from interface
        let rssi: i32 = msg_send![interface, rssiValue];

        let channel_obj: *mut Object = msg_send![interface, wlanChannel];
        let channel = if channel_obj.is_null() {
            0
        } else {
            let ch: i32 = msg_send![channel_obj, channelNumber];
            ch as u8
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Return current network info
        Ok(Some(RawBeacon {
            ssid: Some(current_ssid.as_bytes().to_vec()),
            bssid: [0u8; 6], // BSSID not available without location permission
            channel,
            band: Band::from_channel(channel),
            signal_dbm: rssi as i16,
            noise_dbm: -90,
            ie_data: vec![], // No IE data from CoreWLAN
            beacon_interval: 100,
            timestamp: now,
            uptime_ms: None,
            connected: true,
            link_rates: None,
            local_adapter: None,
        }))
    }
}

unsafe fn parse_cwnetwork(network: *mut Object, timestamp: u64) -> Option<RawBeacon> {
    if network.is_null() {
        return None;
    }

    // SSID
    let ssid_nsstring: *mut Object = msg_send![network, ssid];
    let ssid = if ssid_nsstring.is_null() {
        None
    } else {
        let ssid_str = nsstring_to_string(ssid_nsstring);
        if ssid_str.is_empty() {
            None
        } else {
            Some(ssid_str.as_bytes().to_vec())
        }
    };

    // BSSID - returns a String in format "XX:XX:XX:XX:XX:XX"
    let bssid_nsstring: *mut Object = msg_send![network, bssid];
    let bssid = if bssid_nsstring.is_null() {
        [0u8; 6]
    } else {
        let bssid_str = nsstring_to_string(bssid_nsstring);
        if bssid_str.is_empty() {
            [0u8; 6]
        } else {
            parse_bssid(&bssid_str).unwrap_or([0u8; 6])
        }
    };

    // RSSI
    let rssi: i32 = msg_send![network, rssiValue];

    // Noise (may not be available on all macOS versions)
    let noise: i32 = msg_send![network, noiseMeasurement];

    // Channel - CWChannel object
    let channel_obj: *mut Object = msg_send![network, wlanChannel];
    let channel = if channel_obj.is_null() {
        0
    } else {
        // CWChannel.channelNumber returns NSInteger
        let ch: i32 = msg_send![channel_obj, channelNumber];
        ch as u8
    };

    // Channel band
    let band = Band::from_channel(channel);

    // Beacon Interval
    let beacon_interval: i32 = msg_send![network, beaconInterval];
    let beacon_interval = if beacon_interval > 0 {
        (beacon_interval / 1024) as u16 // TU to ms approximation
    } else {
        100
    };

    // IE Data - CoreWLAN doesn't provide raw IE data, so we'll leave it empty
    // This is a limitation of CoreWLAN scanner
    let ie_data = Vec::new();

    Some(RawBeacon {
        ssid,
        bssid,
        channel,
        band,
        signal_dbm: rssi as i16,
        noise_dbm: noise as i16,
        ie_data,
        beacon_interval,
        timestamp,
        uptime_ms: None,
        connected: false,
        link_rates: None,
        local_adapter: None,
    })
}

unsafe fn nsstring_to_string(obj: *mut Object) -> String {
    if obj.is_null() {
        return String::new();
    }

    let bytes: *const i8 = msg_send![obj, UTF8String];
    if bytes.is_null() {
        return String::new();
    }

    let len: usize = msg_send![obj, lengthOfBytesUsingEncoding:4]; // NSUTF8StringEncoding = 4
    if len == 0 {
        return String::new();
    }

    let slice = std::slice::from_raw_parts(bytes as *const u8, len);
    String::from_utf8_lossy(slice).into_owned()
}

fn parse_bssid(s: &str) -> Option<[u8; 6]> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 6 {
        return None;
    }

    let mut bssid = [0u8; 6];
    for (i, p) in parts.iter().enumerate() {
        bssid[i] = u8::from_str_radix(p, 16).ok()?;
    }

    Some(bssid)
}
