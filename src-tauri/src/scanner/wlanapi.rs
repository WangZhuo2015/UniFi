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
    use windows::Win32::Foundation::{BOOL, ERROR_SUCCESS, HANDLE};
    use windows::Win32::NetworkManagement::WiFi::{
        dot11_BSS_type_any, WlanCloseHandle, WlanEnumInterfaces, WlanFreeMemory,
        WlanGetNetworkBssList, WlanOpenHandle, WlanScan, WLAN_BSS_ENTRY, WLAN_BSS_LIST,
        WLAN_INTERFACE_INFO, WLAN_INTERFACE_INFO_LIST,
    };

    fn check_wlan_status(status: u32, context: &str) -> Result<(), ScanError> {
        if status == ERROR_SUCCESS.0 {
            Ok(())
        } else {
            Err(ScanError::CommandFailed(format!(
                "{context} failed with Win32 error {status}"
            )))
        }
    }

    unsafe fn interface_entries<'a>(
        list: *const WLAN_INTERFACE_INFO_LIST,
    ) -> &'a [WLAN_INTERFACE_INFO] {
        let count = (*list).dwNumberOfItems as usize;
        let first = std::ptr::addr_of!((*list).InterfaceInfo) as *const WLAN_INTERFACE_INFO;
        std::slice::from_raw_parts(first, count)
    }

    unsafe fn bss_entries<'a>(list: *const WLAN_BSS_LIST) -> &'a [WLAN_BSS_ENTRY] {
        let count = (*list).dwNumberOfItems as usize;
        let first = std::ptr::addr_of!((*list).wlanBssEntries) as *const WLAN_BSS_ENTRY;
        std::slice::from_raw_parts(first, count)
    }

    unsafe fn extract_ie_data(entry: &WLAN_BSS_ENTRY) -> Vec<u8> {
        if entry.ulIeSize == 0 {
            return Vec::new();
        }

        let base = entry as *const WLAN_BSS_ENTRY as *const u8;
        let ie_ptr = base.add(entry.ulIeOffset as usize);
        std::slice::from_raw_parts(ie_ptr, entry.ulIeSize as usize).to_vec()
    }

    fn ssid_bytes(ssid: &[u8; 32], len: u32) -> Option<Vec<u8>> {
        let len = len as usize;
        if len == 0 || len > ssid.len() {
            None
        } else {
            Some(ssid[..len].to_vec())
        }
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    unsafe {
        let mut client_handle: HANDLE = HANDLE::default();
        let mut negotiated_version: u32 = 0;
        check_wlan_status(
            WlanOpenHandle(2, None, &mut negotiated_version, &mut client_handle),
            "WlanOpenHandle",
        )?;

        let mut interface_list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        let enum_result = check_wlan_status(
            WlanEnumInterfaces(client_handle, None, &mut interface_list),
            "WlanEnumInterfaces",
        );
        if let Err(err) = enum_result {
            let _ = WlanCloseHandle(client_handle, None);
            return Err(err);
        }

        if interface_list.is_null() {
            let _ = WlanCloseHandle(client_handle, None);
            return Err(ScanError::NoInterface);
        }

        let interfaces = interface_entries(interface_list);
        if interfaces.is_empty() {
            WlanFreeMemory(interface_list as *mut _);
            let _ = WlanCloseHandle(client_handle, None);
            return Err(ScanError::NoInterface);
        }

        let mut triggered_scan = false;
        for iface in interfaces {
            let status = WlanScan(client_handle, &iface.InterfaceGuid, None, None, None);
            if status == ERROR_SUCCESS.0 {
                triggered_scan = true;
            }
        }

        if triggered_scan {
            std::thread::sleep(std::time::Duration::from_millis(1500));
        }

        let mut results = Vec::new();
        let mut bss_query_errors = Vec::new();
        let mut queried_interfaces = 0usize;

        for iface in interfaces {
            let mut bss_list: *mut WLAN_BSS_LIST = std::ptr::null_mut();
            queried_interfaces += 1;
            let status = WlanGetNetworkBssList(
                client_handle,
                &iface.InterfaceGuid,
                None,
                dot11_BSS_type_any,
                BOOL(0),
                None,
                &mut bss_list,
            );

            if status != ERROR_SUCCESS.0 {
                bss_query_errors.push(format!(
                    "WlanGetNetworkBssList failed with Win32 error {status}"
                ));
                continue;
            }

            if bss_list.is_null() {
                bss_query_errors.push("WlanGetNetworkBssList returned a null BSS list".to_string());
                continue;
            }

            for entry in bss_entries(bss_list) {
                let channel = channel_from_frequency(entry.ulChCenterFrequency);
                results.push(RawBeacon {
                    ssid: ssid_bytes(&entry.dot11Ssid.ucSSID, entry.dot11Ssid.uSSIDLength),
                    bssid: entry.dot11Bssid,
                    channel,
                    band: band_from_frequency(entry.ulChCenterFrequency),
                    signal_dbm: entry.lRssi as i16,
                    noise_dbm: -100,
                    ie_data: extract_ie_data(entry),
                    beacon_interval: entry.usBeaconPeriod,
                    timestamp: now,
                    uptime_ms: Some((entry.ullTimestamp / 1000) as u64),
                    connected: false,
                });
            }

            WlanFreeMemory(bss_list as *mut _);
        }

        WlanFreeMemory(interface_list as *mut _);
        let _ = WlanCloseHandle(client_handle, None);

        if results.is_empty() && queried_interfaces > 0 && bss_query_errors.len() == queried_interfaces {
            return Err(ScanError::CommandFailed(bss_query_errors.join("; ")));
        }

        Ok(results)
    }
}

#[cfg(target_os = "windows")]
fn current_windows() -> Result<Option<RawBeacon>, ScanError> {
    use windows::Win32::Foundation::{ERROR_SUCCESS, HANDLE};
    use windows::Win32::NetworkManagement::WiFi::{
        wlan_interface_state_connected, wlan_intf_opcode_current_connection, WlanCloseHandle,
        WlanEnumInterfaces, WlanFreeMemory, WlanOpenHandle, WlanQueryInterface,
        WLAN_CONNECTION_ATTRIBUTES, WLAN_INTERFACE_INFO, WLAN_INTERFACE_INFO_LIST,
    };

    fn check_wlan_status(status: u32, context: &str) -> Result<(), ScanError> {
        if status == ERROR_SUCCESS.0 {
            Ok(())
        } else {
            Err(ScanError::CommandFailed(format!(
                "{context} failed with Win32 error {status}"
            )))
        }
    }

    unsafe fn interface_entries<'a>(
        list: *const WLAN_INTERFACE_INFO_LIST,
    ) -> &'a [WLAN_INTERFACE_INFO] {
        let count = (*list).dwNumberOfItems as usize;
        let first = std::ptr::addr_of!((*list).InterfaceInfo) as *const WLAN_INTERFACE_INFO;
        std::slice::from_raw_parts(first, count)
    }

    fn ssid_bytes(ssid: &[u8; 32], len: u32) -> Option<Vec<u8>> {
        let len = len as usize;
        if len == 0 || len > ssid.len() {
            None
        } else {
            Some(ssid[..len].to_vec())
        }
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    unsafe {
        let mut client_handle: HANDLE = HANDLE::default();
        let mut negotiated_version: u32 = 0;
        check_wlan_status(
            WlanOpenHandle(2, None, &mut negotiated_version, &mut client_handle),
            "WlanOpenHandle",
        )?;

        let mut interface_list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        let enum_result = check_wlan_status(
            WlanEnumInterfaces(client_handle, None, &mut interface_list),
            "WlanEnumInterfaces",
        );
        if let Err(err) = enum_result {
            let _ = WlanCloseHandle(client_handle, None);
            return Err(err);
        }

        if interface_list.is_null() {
            let _ = WlanCloseHandle(client_handle, None);
            return Err(ScanError::NoInterface);
        }

        let scanned_networks = scan_windows().unwrap_or_default();

        for iface in interface_entries(interface_list) {
            if iface.isState != wlan_interface_state_connected {
                continue;
            }

            let mut conn_attrs_ptr: *mut core::ffi::c_void = std::ptr::null_mut();
            let mut size: u32 = 0;
            let status = WlanQueryInterface(
                client_handle,
                &iface.InterfaceGuid,
                wlan_intf_opcode_current_connection,
                None,
                &mut size,
                &mut conn_attrs_ptr,
                None,
            );

            if status != ERROR_SUCCESS.0 || conn_attrs_ptr.is_null() {
                continue;
            }

            let attrs = &*(conn_attrs_ptr as *const WLAN_CONNECTION_ATTRIBUTES);
            let bssid = attrs.wlanAssociationAttributes.dot11Bssid;
            let signal_dbm = signal_quality_to_dbm(attrs.wlanAssociationAttributes.wlanSignalQuality);
            let mut beacon = scanned_networks
                .iter()
                .find(|candidate| candidate.bssid == bssid)
                .cloned()
                .unwrap_or_else(|| RawBeacon {
                    ssid: ssid_bytes(
                        &attrs.wlanAssociationAttributes.dot11Ssid.ucSSID,
                        attrs.wlanAssociationAttributes.dot11Ssid.uSSIDLength,
                    ),
                    bssid,
                    channel: 0,
                    band: Band::default(),
                    signal_dbm,
                    noise_dbm: -100,
                    ie_data: Vec::new(),
                    beacon_interval: 100,
                    timestamp: now,
                    uptime_ms: None,
                    connected: true,
                });

            beacon.connected = true;
            beacon.signal_dbm = signal_dbm;
            beacon.timestamp = now;

            WlanFreeMemory(conn_attrs_ptr as *mut _);
            WlanFreeMemory(interface_list as *mut _);
            let _ = WlanCloseHandle(client_handle, None);
            return Ok(Some(beacon));
        }

        WlanFreeMemory(interface_list as *mut _);
        let _ = WlanCloseHandle(client_handle, None);
        Ok(None)
    }
}

#[cfg(target_os = "windows")]
fn signal_quality_to_dbm(quality: u32) -> i16 {
    (-100 + (quality.min(100) / 2) as i32) as i16
}

#[cfg(target_os = "windows")]
fn band_from_frequency(freq_khz: u32) -> Band {
    match freq_khz / 1000 {
        5925..=7125 => Band::Ghz6,
        5000..=5895 => Band::Ghz5,
        _ => Band::Ghz2_4,
    }
}

#[cfg(target_os = "windows")]
fn channel_from_frequency(freq_khz: u32) -> u8 {
    let freq_mhz = freq_khz / 1000;
    match freq_mhz {
        2412..=2484 => ((freq_mhz - 2407) / 5) as u8,
        5000..=5895 => ((freq_mhz - 5000) / 5) as u8,
        5925..=7125 => ((freq_mhz - 5950) / 5) as u8,
        _ => 0,
    }
}
