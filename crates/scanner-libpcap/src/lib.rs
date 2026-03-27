//! Cross-platform Libpcap Beacon Scanner Plugin
//!
//! Uses libpcap to capture 802.11 beacon frames directly.
//! Requires monitor mode and root/admin privileges.
//! Provides complete IE data including WiFi 6/7 information.

use scanner_core::{Platform, RawBeacon, ScanError, Scanner, ScannerCapabilities, SecurityType, VendorIE};

/// Libpcap beacon scanner
pub struct LibpcapScanner {
    interface: Option<String>,
    timeout_ms: i32,
}

impl Default for LibpcapScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl LibpcapScanner {
    /// Create a new libpcap scanner
    pub fn new() -> Self {
        Self {
            interface: None,
            timeout_ms: 5000,
        }
    }

    /// Set the interface to use
    pub fn with_interface(mut self, interface: impl Into<String>) -> Self {
        self.interface = Some(interface.into());
        self
    }

    /// Set scan timeout in milliseconds
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms.min(i32::MAX as u64) as i32;
        self
    }
    
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn do_scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
        use pcap::{Device, Capture};
        
        // Check for root privileges
        #[cfg(unix)]
        {
            let uid = unsafe { libc::getuid() };
            if uid != 0 {
                return Err(ScanError::PermissionDenied(
                    "libpcap scanner requires root privileges".to_string()
                ));
            }
        }
        
        // Find the interface
        let interface_name = self.interface.as_ref()
            .ok_or_else(|| ScanError::Config("No interface specified".to_string()))?;
        
        // Find the device
        let device = Device::list()
            .map_err(|e| ScanError::System(format!("Failed to list devices: {}", e)))?
            .into_iter()
            .find(|d| &d.name == interface_name)
            .ok_or_else(|| ScanError::InterfaceNotFound(interface_name.clone()))?;
        
        // Create capture
        let mut cap = Capture::from_device(device)
            .map_err(|e| ScanError::System(format!("Failed to create capture: {}", e)))?
            .promisc(true)
            .timeout(self.timeout_ms)
            .open()
            .map_err(|e| ScanError::System(format!("Failed to open capture: {}", e)))?;
        
        // Set filter for beacon frames
        cap.filter("type mgt subtype beacon", true)
            .map_err(|e| ScanError::Config(format!("Failed to set filter: {}", e)))?;
        
        let mut beacons: std::collections::HashMap<String, RawBeacon> = std::collections::HashMap::new();
        let start = std::time::Instant::now();
        
        while start.elapsed().as_millis() < self.timeout_ms as u128 {
            match cap.next_packet() {
                Ok(packet) => {
                    if let Some(beacon) = self.parse_beacon(&packet.data) {
                        if let Some(bssid) = &beacon.bssid {
                            beacons.entry(bssid.clone()).or_insert(beacon);
                        }
                    }
                }
                Err(pcap::Error::TimeoutExpired) => break,
                Err(_) => continue,
            }
        }
        
        Ok(beacons.into_values().collect())
    }
    
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn parse_beacon(&self, data: &[u8]) -> Option<RawBeacon> {
        // Minimum beacon frame size: radiotap + MAC header + fixed + 2 SSID IE
        if data.len() < 50 {
            return None;
        }
        
        let mut beacon = RawBeacon::new();
        
        // Parse radiotap header
        let (radiotap_len, signal) = self.parse_radiotap(data);
        beacon.signal = signal;
        
        // Skip radiotap header
        let frame_data = &data[radiotap_len as usize..];
        
        // Check frame type (beacon = 0x80)
        if frame_data.is_empty() || frame_data[0] != 0x80 {
            return None;
        }
        
        // Parse MAC header (24 bytes)
        if frame_data.len() < 24 {
            return None;
        }
        
        // BSSID is at offset 16 in the MAC header
        let bssid = &frame_data[16..22];
        beacon.bssid = Some(format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            bssid[0], bssid[1], bssid[2], bssid[3], bssid[4], bssid[5]
        ));
        
        // Skip to IE section (MAC header 24 + fixed params 12 = 36)
        if frame_data.len() < 36 {
            return None;
        }
        
        let ie_data = &frame_data[36..];
        beacon.ie_data = Some(ie_data.to_vec());
        
        // Parse IEs
        self.parse_ies(ie_data, &mut beacon);
        
        Some(beacon)
    }
    
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn parse_radiotap(&self, data: &[u8]) -> (u8, Option<i32>) {
        // Radiotap header: version(1) + pad(1) + len(2) + flags(4)
        if data.len() < 4 {
            return (0, None);
        }
        
        let radiotap_len = u16::from_le_bytes([data[2], data[3]]) as u8;
        let present_flags = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        
        // Check if signal is present (bit 5)
        let has_signal = (present_flags & (1 << 5)) != 0;
        
        let mut offset = 8;
        let mut signal = None;
        
        // Parse present flags to find signal offset
        for bit in 0..32 {
            if bit >= 5 && has_signal {
                break;
            }
            if (present_flags & (1 << bit)) != 0 {
                offset += match bit {
                    0 => 8,   // TSFT
                    1 => 1,   // Flags
                    2 => 1,   // Rate
                    3 => 4,   // Channel
                    4 => 2,   // FHSS
                    5 => 1,   // Antenna signal
                    6 => 1,   // Antenna noise
                    7 => 2,   // Lock quality
                    8 => 2,   // TX attenuation
                    9 => 2,   // TX power
                    10 => 1,  // Antenna
                    11 => 1,  // Antenna signal dB
                    12 => 1,  // Antenna noise dB
                    _ => 0,
                };
            }
        }
        
        if has_signal && (offset as usize) < data.len() {
            signal = Some(data[offset as usize] as i8 as i32);
        }
        
        (radiotap_len, signal)
    }
    
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn parse_ies(&self, data: &[u8], beacon: &mut RawBeacon) {
        let mut pos = 0;
        
        while pos + 2 <= data.len() {
            let ie_type = data[pos];
            let ie_len = data[pos + 1] as usize;
            
            if pos + 2 + ie_len > data.len() {
                break;
            }
            
            let ie_data = &data[pos + 2..pos + 2 + ie_len];
            
            match ie_type {
                0 => { // SSID
                    beacon.ssid = Some(String::from_utf8_lossy(ie_data).to_string());
                }
                1 => { // Supported rates
                    // Parse basic rates
                }
                3 => { // DS Parameter Set (channel)
                    if !ie_data.is_empty() {
                        beacon.channel = Some(ie_data[0] as u32);
                        let ch = ie_data[0];
                        beacon.frequency = if ch <= 14 {
                            Some(2407 + ch as u32 * 5)
                        } else {
                            Some(5000 + ch as u32 * 5)
                        };
                    }
                }
                48 => { // RSN (WPA2)
                    beacon.is_secured = true;
                    beacon.security = Some(SecurityType::WPA2);
                }
                221 => { // Vendor-specific IE
                    if ie_len >= 4 {
                        let oui = [ie_data[0], ie_data[1], ie_data[2]];
                        let vendor_type = ie_data[3];
                        beacon.vendor_ies.push(VendorIE {
                            oui,
                            vendor_type,
                            data: ie_data[4..].to_vec(),
                        });
                    }
                }
                _ => {}
            }
            
            pos += 2 + ie_len;
        }
    }
}

impl Scanner for LibpcapScanner {
    fn name(&self) -> &'static str {
        "libpcap"
    }
    
    fn description(&self) -> &'static str {
        "Libpcap beacon capture (requires monitor mode and root)"
    }
    
    fn scan(&self) -> Result<Vec<RawBeacon>, ScanError> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            self.do_scan()
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Err(ScanError::NotAvailable("libpcap".to_string()))
        }
    }
    
    fn current(&self) -> Result<Option<RawBeacon>, ScanError> {
        // Libpcap can't get current network, use platform-specific API
        Ok(None)
    }
    
    fn platforms(&self) -> &'static [Platform] {
        &[Platform::MacOS, Platform::Linux]
    }
    
    fn capabilities(&self) -> ScannerCapabilities {
        ScannerCapabilities {
            has_ie_data: true,
            has_bssid: true,
            has_signal: true,
            has_security: true,
            app_store_compatible: false,
        }
    }
    
    fn requires_privilege(&self) -> bool {
        true
    }
    
    fn is_available(&self) -> bool {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            // Check for root
            #[cfg(unix)]
            {
                unsafe { libc::getuid() == 0 }
            }
            #[cfg(not(unix))]
            {
                true
            }
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        { false }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scanner_info() {
        let scanner = LibpcapScanner::new();
        assert_eq!(scanner.name(), "libpcap");
        assert!(scanner.capabilities().has_ie_data);
        assert!(scanner.requires_privilege());
    }
    
    #[test]
    fn test_builder() {
        let scanner = LibpcapScanner::new()
            .with_interface("en0")
            .with_timeout(3000);
        
        assert_eq!(scanner.interface, Some("en0".to_string()));
        assert_eq!(scanner.timeout_ms, 3000);
    }
}
