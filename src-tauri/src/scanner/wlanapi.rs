//! Windows WlanApi Scanner
//!
//! Uses Windows WlanApi via the `windows` crate.

use crate::scanner::{RawBeacon, Scanner};
use crate::types::{Band, LinkRates, LocalAdapterCapabilities, ScanError};
use crate::process;

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
                    link_rates: None,
                    local_adapter: None,
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

        let local_adapter = query_local_adapter_capabilities_with_netsh();
        let current_details = query_current_interface_with_netsh();

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
            let connected_bssid = format!(
                "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                bssid[0], bssid[1], bssid[2], bssid[3], bssid[4], bssid[5]
            );
            let channel = current_details.as_ref().and_then(|details| details.channel).unwrap_or(0);
            let band = current_details
                .as_ref()
                .and_then(|details| details.band)
                .unwrap_or_else(|| if channel > 14 { Band::Ghz5 } else { Band::Ghz2_4 });
            let mut beacon = RawBeacon {
                ssid: current_details
                    .as_ref()
                    .and_then(|details| details.ssid.clone())
                    .or_else(|| {
                        ssid_bytes(
                            &attrs.wlanAssociationAttributes.dot11Ssid.ucSSID,
                            attrs.wlanAssociationAttributes.dot11Ssid.uSSIDLength,
                        )
                    }),
                bssid,
                channel,
                band,
                signal_dbm,
                noise_dbm: -100,
                ie_data: Vec::new(),
                beacon_interval: 100,
                timestamp: now,
                uptime_ms: None,
                connected: true,
                link_rates: None,
                local_adapter: local_adapter.clone(),
            };

            beacon.connected = true;
            beacon.signal_dbm = signal_dbm;
            beacon.timestamp = now;
            beacon.local_adapter = local_adapter.clone();
            beacon.link_rates = current_details
                .as_ref()
                .and_then(|details| {
                    if details.bssid.as_deref() == Some(connected_bssid.as_str()) {
                        details.link_rates.clone()
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    Some(LinkRates {
                        rx_rate_mbps: Some(rate_to_mbps(attrs.wlanAssociationAttributes.ulRxRate)),
                        tx_rate_mbps: Some(rate_to_mbps(attrs.wlanAssociationAttributes.ulTxRate)),
                    })
                });

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
fn query_local_adapter_capabilities_with_netsh() -> Option<LocalAdapterCapabilities> {
    let drivers_output = netsh_output(["wlan", "show", "drivers"])?;
    let capabilities_output = netsh_output(["wlan", "show", "wirelesscapabilities"])?;

    if !drivers_output.status.success() || !capabilities_output.status.success() {
        return None;
    }

    let drivers = String::from_utf8_lossy(&drivers_output.stdout);
    let wireless_caps = String::from_utf8_lossy(&capabilities_output.stdout);

    let mut driver_name = String::new();
    let mut supported_standards = Vec::new();

    for raw_line in drivers.lines() {
        let mut parts = raw_line.splitn(2, ':');
        let Some(key) = parts.next() else {
            continue;
        };
        let value = parts.next().unwrap_or("").trim();

        match key.trim() {
            "Driver" => driver_name = value.to_string(),
            "Radio types supported" => {
                for standard in value.split_whitespace().filter_map(standard_from_netsh) {
                    if !supported_standards.iter().any(|item| item == standard) {
                        supported_standards.push(standard.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    let mut tx_spatial_streams = 1u8;
    let mut rx_spatial_streams = 1u8;

    for raw_line in wireless_caps.lines() {
        let mut parts = raw_line.splitn(2, ':');
        let Some(key) = parts.next() else {
            continue;
        };
        let value = parts.next().unwrap_or("").trim();

        match key.trim() {
            "Number of Tx Spatial Streams" => {
                tx_spatial_streams = value.parse::<u8>().ok().filter(|value| *value > 0).unwrap_or(1);
            }
            "Number of Rx Spatial Streams" => {
                rx_spatial_streams = value.parse::<u8>().ok().filter(|value| *value > 0).unwrap_or(1);
            }
            _ => {}
        }
    }

    if driver_name.is_empty() && supported_standards.is_empty() {
        return None;
    }

    Some(LocalAdapterCapabilities {
        driver_name: driver_name.clone(),
        supported_standards,
        tx_spatial_streams,
        rx_spatial_streams,
        max_supported_width: max_width_from_driver_name(&driver_name),
    })
}

#[cfg(target_os = "windows")]
fn standard_from_netsh(token: &str) -> Option<&'static str> {
    match token {
        "802.11b" => Some("b"),
        "802.11g" => Some("g"),
        "802.11n" => Some("n"),
        "802.11a" => Some("a"),
        "802.11ac" => Some("ac"),
        "802.11ax" => Some("ax"),
        "802.11be" => Some("be"),
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn max_width_from_driver_name(driver_name: &str) -> u16 {
    let name = driver_name.to_ascii_lowercase();

    if name.contains("320mhz") {
        320
    } else if name.contains("160mhz") {
        160
    } else if name.contains("80mhz") {
        80
    } else if name.contains("40mhz") {
        40
    } else if name.contains("ax") || name.contains("be") {
        160
    } else if name.contains("ac") {
        80
    } else if name.contains("n") {
        40
    } else {
        20
    }
}

#[cfg(target_os = "windows")]
fn signal_quality_to_dbm(quality: u32) -> i16 {
    (-100 + (quality.min(100) / 2) as i32) as i16
}

#[cfg(target_os = "windows")]
fn rate_to_mbps(raw_rate: u32) -> f32 {
    let rate = raw_rate as f32;
    if raw_rate >= 1_000_000 {
        rate / 1_000_000.0
    } else if raw_rate >= 10_000 {
        rate / 1000.0
    } else {
        rate
    }
}

#[cfg(target_os = "windows")]
struct CurrentInterfaceDetails {
    ssid: Option<Vec<u8>>,
    bssid: Option<String>,
    channel: Option<u8>,
    band: Option<Band>,
    link_rates: Option<LinkRates>,
}

#[cfg(target_os = "windows")]
fn query_current_interface_with_netsh() -> Option<CurrentInterfaceDetails> {
    let output = netsh_output(["wlan", "show", "interfaces"])?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut seen_bssid: Option<String> = None;
    let mut ssid: Option<Vec<u8>> = None;
    let mut channel: Option<u8> = None;
    let mut rx_rate_mbps: Option<f32> = None;
    let mut tx_rate_mbps: Option<f32> = None;

    for raw_line in stdout.lines() {
        let mut parts = raw_line.splitn(2, ':');
        let Some(key) = parts.next() else {
            continue;
        };
        let key = key.trim();
        let value = parts.next().unwrap_or("").trim();

        match key {
            "SSID" if !raw_line.contains("BSSID") => ssid = Some(value.as_bytes().to_vec()),
            "AP BSSID" => seen_bssid = Some(value.to_ascii_uppercase()),
            "Channel" => channel = value.parse::<u8>().ok(),
            "Receive rate (Mbps)" => rx_rate_mbps = value.parse::<f32>().ok(),
            "Transmit rate (Mbps)" => tx_rate_mbps = value.parse::<f32>().ok(),
            _ => {}
        }
    }

    Some(CurrentInterfaceDetails {
        ssid,
        bssid: seen_bssid.clone(),
        channel,
        band: channel.map(|value| if value > 14 { Band::Ghz5 } else { Band::Ghz2_4 }),
        link_rates: Some(LinkRates {
            rx_rate_mbps,
            tx_rate_mbps,
        }),
    })
}

#[cfg(target_os = "windows")]
fn netsh_output<const N: usize>(args: [&str; N]) -> Option<std::process::Output> {
    process::command("netsh")
        .args(args)
        .output()
        .ok()
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
