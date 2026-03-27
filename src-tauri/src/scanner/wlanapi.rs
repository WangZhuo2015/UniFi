//! Windows WlanApi Scanner
//!
//! Uses Windows WlanApi via the `windows` crate.

use crate::scanner::{RawBeacon, Scanner};
use crate::types::{Band, ScanError};

pub struct WlanApiScanner;

impl WlanApiScanner {
    pub fn new() -> Self {
        Self
    }
}

impl Scanner for WlanApiScanner {
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
        #[cfg(target_os = "windows")]
        {
            scan_windows()
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            Err(ScanError::NotSupported)
        }
    }
    
    fn current(&self) -> Result<Option<RawBeacon>, ScanError> {
        #[cfg(target_os = "windows")]
        {
            current_windows()
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            Err(ScanError::NotSupported)
        }
    }
    
    fn name(&self) -> &'static str {
        "Windows WlanApi"
    }
    
    fn requires_privilege(&self) -> bool {
        false
    }
}

#[cfg(target_os = "windows")]
fn scan_windows() -> Result<Vec<RawBeacon>, ScanError> {
    use windows::Win32::Foundation::*;
    use windows::Win32::NetworkManagement::WiFi::*;
    use windows::core::Interface;
    
    unsafe {
        // Open WLAN handle
        let mut client_handle: HANDLE = HANDLE::default();
        let mut negotiated_version: u32 = 0;
        
        WlanOpenHandle(2, None, &mut negotiated_version, &mut client_handle)
            .map_err(|e| ScanError::CommandFailed(e.to_string()))?;
        
        // Enumerate interfaces
        let mut interface_list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        
        WlanEnumInterfaces(client_handle, None, &mut interface_list)
            .map_err(|e| ScanError::CommandFailed(e.to_string()))?;
        
        let interfaces = &*interface_list;
        let mut results = Vec::new();
        
        for i in 0..interfaces.dwNumberOfItems {
            let iface = interfaces.InterfaceInfo[i as usize];
            
            // Get BSS list for this interface
            let mut bss_list: *mut WLAN_BSS_LIST = std::ptr::null_mut();
            
            let result = WlanGetNetworkBssList(
                client_handle,
                &iface.InterfaceGuid,
                None,
                dot11_BSS_type_any,
                false,
                None,
                &mut bss_list,
            );
            
            if result.is_err() {
                continue;
            }
            
            let bss = &*bss_list;
            
            for j in 0..bss.dwNumberOfItems {
                let entry = bss.wlanBssEntries[j as usize];
                
                let beacon = RawBeacon {
                    ssid: if entry.dot11Ssid.uSSIDLength > 0 {
                        Some(entry.dot11Ssid.ucSSID[..entry.dot11Ssid.uSSIDLength as usize].to_vec())
                    } else {
                        None
                    },
                    bssid: entry.dot11Bssid,
                    channel: channel_from_frequency(entry.ulChCenterFrequency),
                    band: Band::from_channel(channel_from_frequency(entry.ulChCenterFrequency)),
                    signal_dbm: entry.lRssi,
                    noise_dbm: -100,
                    ie_data: entry.IeBlob.to_vec(),
                    beacon_interval: entry.usBeaconPeriod,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPEC)
                        .unwrap()
                        .as_secs(),
                    connected: false,
                };
                
                results.push(beacon);
            }
            
            WlanFreeMemory(bss_list as *mut _);
        }
        
        WlanFreeMemory(interface_list as *mut _);
        let _ = WlanCloseHandle(client_handle, None);
        
        Ok(results)
    }
}

#[cfg(target_os = "windows")]
fn current_windows() -> Result<Option<RawBeacon>, ScanError> {
    use windows::Win32::Foundation::*;
    use windows::Win32::NetworkManagement::WiFi::*;
    
    unsafe {
        let mut client_handle: HANDLE = HANDLE::default();
        let mut negotiated_version: u32 = 0;
        
        WlanOpenHandle(2, None, &mut negotiated_version, &mut client_handle)
            .map_err(|e| ScanError::CommandFailed(e.to_string()))?;
        
        let mut interface_list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        
        WlanEnumInterfaces(client_handle, None, &mut interface_list)
            .map_err(|e| ScanError::CommandFailed(e.to_string()))?;
        
        let interfaces = &*interface_list;
        let mut result = None;
        
        for i in 0..interfaces.dwNumberOfItems {
            let iface = interfaces.InterfaceInfo[i as usize];
            
            if iface.isState == wlan_interface_state_connected {
                // Get connection attributes
                let mut conn_attrs: *mut WLAN_CONNECTION_ATTRIBUTES = std::ptr::null_mut();
                let mut size: u32 = 0;
                
                if WlanQueryInterface(
                    client_handle,
                    &iface.InterfaceGuid,
                    wlan_intf_opcode_current_connection,
                    None,
                    &mut size,
                    &mut conn_attrs as *mut _ as *mut _,
                    None,
                ).is_ok() {
                    let attrs = &*conn_attrs;
                    
                    result = Some(RawBeacon {
                        ssid: if attrs.wlanAssociationAttributes.dot11Ssid.uSSIDLength > 0 {
                            Some(attrs.wlanAssociationAttributes.dot11Ssid.ucSSID
                                [..attrs.wlanAssociationAttributes.dot11Ssid.uSSIDLength as usize].to_vec())
                        } else {
                            None
                        },
                        bssid: attrs.wlanAssociationAttributes.dot11Bssid,
                        channel: channel_from_frequency(attrs.wlanAssociationAttributes.ulChCenterFrequency),
                        band: Band::from_channel(channel_from_frequency(attrs.wlanAssociationAttributes.ulChCenterFrequency)),
                        signal_dbm: attrs.wlanAssociationAttributes.lRssi,
                        noise_dbm: -100,
                        ie_data: vec![],
                        beacon_interval: 100,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        connected: true,
                    });
                    
                    WlanFreeMemory(conn_attrs as *mut _);
                    break;
                }
            }
        }
        
        WlanFreeMemory(interface_list as *mut _);
        let _ = WlanCloseHandle(client_handle, None);
        
        Ok(result)
    }
}

#[cfg(target_os = "windows")]
fn channel_from_frequency(freq_khz: u32) -> u8 {
    // 2.4 GHz: channels 1-14
    // 5 GHz: channels 36-165
    match freq_khz {
        2412..=2484 => ((freq_khz - 2407) / 5) as u8,
        5170..=5825 => ((freq_khz - 5000) / 5) as u8,
        _ => 0,
    }
}
