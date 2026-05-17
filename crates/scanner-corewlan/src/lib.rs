//! macOS CoreWLAN Framework Scanner Plugin
//!
//! Uses Apple's CoreWLAN framework for WiFi scanning.
//! App Store compatible, but has limitations:
//! - No IE data available
//! - BSSID requires location permission and may return null
#![allow(unexpected_cfgs)]

use scanner_core::{Platform, RawBeacon, ScanError, Scanner, ScannerCapabilities};

/// CoreWLAN scanner using Apple's framework
pub struct CoreWlanScanner {
    #[cfg(target_os = "macos")]
    _phantom: std::marker::PhantomData<()>,
}

impl Default for CoreWlanScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreWlanScanner {
    /// Create a new CoreWLAN scanner
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "macos")]
            _phantom: std::marker::PhantomData,
        }
    }
    
    #[cfg(target_os = "macos")]
    fn do_scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
        use objc::runtime::{Class, Object};
        use objc::{msg_send, sel, sel_impl};

        unsafe {
            // Get CWWiFiClient shared client
            let client_class = Class::get("CWWiFiClient").ok_or_else(|| {
                ScanError::System("CWWiFiClient class not found".to_string())
            })?;

            let shared_client: *mut Object = msg_send![client_class, sharedWiFiClient];
            if shared_client.is_null() {
                return Err(ScanError::System("Failed to get shared WiFi client".to_string()));
            }

            // Get interface
            let interface: *mut Object = msg_send![shared_client, interface];
            if interface.is_null() {
                return Err(ScanError::System("No WiFi interface available".to_string()));
            }

            // Perform scan with nil SSID (scan all networks)
            let mut error: *mut Object = std::ptr::null_mut();
            let networks: *mut Object = msg_send![interface, scanForNetworksWithSSID:std::ptr::null::<()>() error:&mut error];
            
            if !error.is_null() {
                let desc: *mut Object = msg_send![error, localizedDescription];
                let desc_str: *const i8 = msg_send![desc, UTF8String];
                let desc = std::ffi::CStr::from_ptr(desc_str).to_string_lossy().to_string();
                return Err(ScanError::System(desc));
            }
            
            // Convert NSSet to NSArray
            let networks_array: *mut Object = msg_send![networks, allObjects];
            let count: usize = msg_send![networks_array, count];
            
            let mut beacons = Vec::with_capacity(count);
            
            for i in 0..count {
                let network: *mut Object = msg_send![networks_array, objectAtIndex:i];
                
                let mut beacon = RawBeacon::new();
                
                // Get SSID
                let ssid: *mut Object = msg_send![network, ssid];
                if !ssid.is_null() {
                    let ssid_str: *const i8 = msg_send![ssid, UTF8String];
                    beacon.ssid = Some(std::ffi::CStr::from_ptr(ssid_str).to_string_lossy().to_string());
                }
                
                // Get BSSID (may be null without location permission)
                let bssid: *mut Object = msg_send![network, bssid];
                if !bssid.is_null() {
                    let bssid_str: *const i8 = msg_send![bssid, UTF8String];
                    beacon.bssid = Some(std::ffi::CStr::from_ptr(bssid_str).to_string_lossy().to_string());
                }
                
                // Get RSSI
                let rssi: i32 = msg_send![network, rssiValue];
                beacon.signal = Some(rssi);
                
                // Get channel
                let channel: *mut Object = msg_send![network, wlanChannel];
                if !channel.is_null() {
                    let channel_number: i32 = msg_send![channel, channelNumber];
                    beacon.channel = Some(channel_number as u32);
                    
                    // Get frequency band from channel
                    let band: i32 = msg_send![channel, channelBand];
                    // 1 = 2.4GHz, 2 = 5GHz, 3 = 6GHz
                    beacon.frequency = if band == 1 {
                        Some(2407 + (channel_number as u32) * 5)
                    } else if band == 2 {
                        Some(5000 + (channel_number as u32) * 5)
                    } else if band == 3 {
                        Some(5950 + (channel_number as u32) * 5)
                    } else {
                        None
                    };
                }
                
                beacons.push(beacon);
            }
            
            Ok(beacons)
        }
    }
    
    #[cfg(target_os = "macos")]
    fn get_current(&self) -> Result<Option<RawBeacon>, ScanError> {
        use objc::runtime::{Class, Object};
        use objc::{msg_send, sel, sel_impl};
        
        unsafe {
            let client_class = Class::get("CWWiFiClient").ok_or_else(|| {
                ScanError::System("CWWiFiClient class not found".to_string())
            })?;
            
            let shared_client: *mut Object = msg_send![client_class, sharedWiFiClient];
            if shared_client.is_null() {
                return Ok(None);
            }
            
            let interface: *mut Object = msg_send![shared_client, interface];
            if interface.is_null() {
                return Ok(None);
            }
            
            // Check if associated
            let is_connected: bool = msg_send![interface, serviceActive];
            if !is_connected {
                return Ok(None);
            }
            
            let mut beacon = RawBeacon::new();
            
            // Get SSID directly from interface
            let ssid: *mut Object = msg_send![interface, ssid];
            if !ssid.is_null() {
                let ssid_str: *const i8 = msg_send![ssid, UTF8String];
                beacon.ssid = Some(std::ffi::CStr::from_ptr(ssid_str).to_string_lossy().to_string());
            }
            
            // Get BSSID
            let bssid: *mut Object = msg_send![interface, bssid];
            if !bssid.is_null() {
                let bssid_str: *const i8 = msg_send![bssid, UTF8String];
                beacon.bssid = Some(std::ffi::CStr::from_ptr(bssid_str).to_string_lossy().to_string());
            }
            
            // Get channel
            let channel: *mut Object = msg_send![interface, wlanChannel];
            if !channel.is_null() {
                let channel_number: i32 = msg_send![channel, channelNumber];
                beacon.channel = Some(channel_number as u32);
            }
            
            Ok(Some(beacon))
        }
    }
}

impl Scanner for CoreWlanScanner {
    fn name(&self) -> &'static str {
        "corewlan"
    }
    
    fn description(&self) -> &'static str {
        "macOS CoreWLAN framework scanner (App Store compatible)"
    }
    
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
        #[cfg(target_os = "macos")]
        {
            self.do_scan()
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Err(ScanError::NotAvailable("corewlan".to_string()))
        }
    }
    
    fn current(&self) -> Result<Option<RawBeacon>, ScanError> {
        #[cfg(target_os = "macos")]
        {
            self.get_current()
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Ok(None)
        }
    }
    
    fn platforms(&self) -> &'static [Platform] {
        &[Platform::MacOS]
    }
    
    fn capabilities(&self) -> ScannerCapabilities {
        ScannerCapabilities {
            has_ie_data: false,  // CoreWLAN doesn't provide IE data
            has_bssid: true,     // Requires location permission
            has_signal: true,
            has_security: true,
            app_store_compatible: true,  // Works on App Store
        }
    }
    
    fn requires_privilege(&self) -> bool {
        false
    }
    
    fn is_available(&self) -> bool {
        #[cfg(target_os = "macos")]
        { true }
        #[cfg(not(target_os = "macos"))]
        { false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scanner_info() {
        let scanner = CoreWlanScanner::new();
        assert_eq!(scanner.name(), "corewlan");
        assert!(scanner.capabilities().app_store_compatible);
        assert!(!scanner.capabilities().has_ie_data);
    }
}
