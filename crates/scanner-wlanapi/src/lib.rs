//! Windows WlanAPI Scanner Plugin
//!
//! Uses Windows Native WiFi API for WiFi scanning.

use scanner_core::{Platform, RawBeacon, ScanError, Scanner, ScannerCapabilities};

/// Windows WlanAPI scanner
pub struct WlanApiScanner {
    #[cfg(target_os = "windows")]
    _phantom: std::marker::PhantomData<()>,
}

impl Default for WlanApiScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl WlanApiScanner {
    /// Create a new WlanAPI scanner
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            _phantom: std::marker::PhantomData,
        }
    }
    
    #[cfg(target_os = "windows")]
    fn do_scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
        use windows::Win32::NetworkManagement::WiFi::*;
        use windows::Win32::Foundation::*;
        
        unsafe {
            // Open WLAN handle
            let mut client_handle: HANDLE = HANDLE::default();
            let mut negotiated_version: u32 = 0;
            
            WlanOpenHandle(2, None, &mut negotiated_version, &mut client_handle)
                .map_err(|e| ScanError::System(format!("WlanOpenHandle failed: {}", e)))?;
            
            // Get list of interfaces
            let mut interface_list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
            
            WlanEnumInterfaces(client_handle, None, &mut interface_list)
                .map_err(|e| ScanError::System(format!("WlanEnumInterfaces failed: {}", e)))?;
            
            if interface_list.is_null() {
                let _ = WlanCloseHandle(client_handle, None);
                return Err(ScanError::InterfaceNotFound("No WiFi interfaces".to_string()));
            }
            
            let interfaces = &*interface_list;
            let mut beacons = Vec::new();
            
            for i in 0..interfaces.dwNumberOfItems {
                let interface = &interfaces.InterfaceInfo[i as usize];
                
                // Scan for networks
                WlanScan(client_handle, &interface.InterfaceGuid, None, None, None).ok();
                
                // Get available networks
                let mut network_list: *mut WLAN_AVAILABLE_NETWORK_LIST = std::ptr::null_mut();
                
                if WlanGetAvailableNetworkList(
                    client_handle,
                    &interface.InterfaceGuid,
                    WLAN_AVAILABLE_NETWORK_INCLUDE_ALL_MANUAL_HIDDEN_PROFILES,
                    None,
                    &mut network_list,
                ).is_ok() && !network_list.is_null() 
                {
                    let networks = &*network_list;
                    
                    for j in 0..networks.dwNumberOfItems {
                        let network = &networks.Network[j as usize];
                        let mut beacon = RawBeacon::new();
                        
                        // SSID
                        let ssid_len = network.dot11Ssid.uSSIDLength as usize;
                        if ssid_len > 0 && ssid_len <= 32 {
                            let ssid = std::str::from_utf8(&network.dot11Ssid.ucSSID[..ssid_len])
                                .unwrap_or("")
                                .to_string();
                            beacon.ssid = Some(ssid);
                        }
                        
                        // Signal quality (0-100, convert to dBm)
                        beacon.signal = Some(-100 + (network.wlanSignalQuality as i32 / 2));
                        
                        // Security
                        beacon.is_secured = network.bSecurityEnabled.as_bool();
                        
                        // Channel (would need BSS list for this)
                        
                        beacons.push(beacon);
                    }
                    
                    WlanFreeMemory(network_list as *const _);
                }
            }
            
            WlanFreeMemory(interface_list as *const _);
            let _ = WlanCloseHandle(client_handle, None);
            
            Ok(beacons)
        }
    }
}

impl Scanner for WlanApiScanner {
    fn name(&self) -> &'static str {
        "wlanapi"
    }
    
    fn description(&self) -> &'static str {
        "Windows Native WiFi API scanner"
    }
    
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
        #[cfg(target_os = "windows")]
        {
            self.do_scan()
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            Err(ScanError::NotAvailable("wlanapi".to_string()))
        }
    }
    
    fn current(&self) -> Result<Option<RawBeacon>, ScanError> {
        // TODO: Implement using WlanQueryInterface
        Ok(None)
    }
    
    fn platforms(&self) -> &'static [Platform] {
        &[Platform::Windows]
    }
    
    fn capabilities(&self) -> ScannerCapabilities {
        ScannerCapabilities {
            has_ie_data: false,  // WlanAPI doesn't expose raw IE data easily
            has_bssid: true,
            has_signal: true,
            has_security: true,
            app_store_compatible: true,  // Standard Windows API
        }
    }
    
    fn requires_privilege(&self) -> bool {
        false
    }
    
    fn is_available(&self) -> bool {
        #[cfg(target_os = "windows")]
        { true }
        #[cfg(not(target_os = "windows"))]
        { false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scanner_info() {
        let scanner = WlanApiScanner::new();
        assert_eq!(scanner.name(), "wlanapi");
        assert!(!scanner.requires_privilege());
    }
}
