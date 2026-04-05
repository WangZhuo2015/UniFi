//! Integration tests for IE parser with real beacon frame data

use unifi_lib::{parse_beacon, RawBeacon, Band};

/// Create a minimal RawBeacon for testing
fn create_test_beacon(ie_data: Vec<u8>, channel: u8, band: Band) -> RawBeacon {
    RawBeacon {
        ssid: None,
        bssid: [0x80, 0x2D, 0x1A, 0x4B, 0x8C, 0x07],
        channel,
        band,
        signal_dbm: -50,
        noise_dbm: -100,
        ie_data,
        beacon_interval: 100,
        timestamp: 0,
        uptime_ms: None,
        connected: false,
        link_rates: None,
        local_adapter: None,
        // WiFi standard flags - will be parsed from IE data
        has_ht: false,
        has_vht: false,
        has_he: false,
        has_eht: false,
        spatial_streams: None,
    }
}

// =============================================================================
// WiFi 4 (802.11n) Tests
// =============================================================================

#[test]
fn test_wifi4_beacon_parsing() {
    // Real WiFi 4 beacon IE data (simplified)
    // Contains: SSID, DS Params, HT Capabilities
    let ie_data = vec![
        // SSID IE
        0x00, 0x08, b'W', b'i', b'F', b'i', b'4', b'T', b'e', b's',
        // Supported Rates
        0x01, 0x08, 0x8C, 0x12, 0x98, 0x24, 0xB0, 0x48, 0x60, 0x6C,
        // DS Parameter Set (channel 6)
        0x03, 0x01, 0x06,
        // HT Capabilities (WiFi 4)
        0x2D, 0x1A, 0x6C, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ];

    let beacon = create_test_beacon(ie_data, 6, Band::Ghz2_4);
    let network = parse_beacon(&beacon);

    assert_eq!(network.ssid, Some("WiFi4Tes".to_string()));
    assert!(network.standards.contains(&"n".to_string()));
    assert_eq!(network.channel, 6);
    assert_eq!(network.wifi_generation, 4);
}

// =============================================================================
// WiFi 5 (802.11ac) Tests
// =============================================================================

#[test]
fn test_wifi5_beacon_parsing() {
    // Real WiFi 5 beacon IE data (simplified)
    let mut ie_data = Vec::new();

    // SSID
    ie_data.extend_from_slice(&[0x00, 0x08]);
    ie_data.extend_from_slice(b"WiFi5-AC");

    // DS Parameter Set (channel 36)
    ie_data.extend_from_slice(&[0x03, 0x01, 0x24]);

    // HT Capabilities
    ie_data.extend_from_slice(&[
        0x2D, 0x1A, 0x6C, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ]);

    // VHT Capabilities (WiFi 5)
    ie_data.extend_from_slice(&[
        0xBF, 0x0C,  // ID 191, Length 12
        0xB2, 0x01, 0x00, 0x00,  // VHT Cap Info
        0xFF, 0x00, 0xFF, 0x00,  // RX/TX MCS map
        0x00, 0x00, 0x00, 0x00,  // Reserved
    ]);

    let beacon = create_test_beacon(ie_data, 36, Band::Ghz5);
    let network = parse_beacon(&beacon);

    assert_eq!(network.ssid, Some("WiFi5-AC".to_string()));
    assert!(network.standards.contains(&"n".to_string()));
    assert!(network.standards.contains(&"ac".to_string()));
    assert_eq!(network.wifi_generation, 5);
    assert_eq!(network.features.max_qam, 256);
}

// =============================================================================
// WiFi 6 (802.11ax) Tests
// =============================================================================

#[test]
fn test_wifi6_beacon_parsing() {
    let mut ie_data = Vec::new();

    // SSID
    ie_data.extend_from_slice(&[0x00, 0x08]);
    ie_data.extend_from_slice(b"WiFi6-AX");

    // DS Parameter Set (channel 36)
    ie_data.extend_from_slice(&[0x03, 0x01, 0x24]);

    // HT Capabilities (ID 45)
    ie_data.extend_from_slice(&[
        0x2D, 0x1A,  // ID 45, Length 26
        0x6C, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // 10 bytes
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,  // 11 bytes (was 12)
        0x00, 0x00, 0x00, 0x00, 0x00,  // 5 bytes (was 4)
    ]);

    // VHT Capabilities (ID 191)
    ie_data.extend_from_slice(&[
        0xBF, 0x0C,  // ID 191, Length 12
        0xB2, 0x01, 0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ]);

    // HE Capabilities (WiFi 6) - Extended IE
    // Format: ID(255) + Length + ExtID(35) + HE Data
    // HE MAC Capabilities: 6 bytes
    // HE PHY Capabilities: 11 bytes
    // HE MCS NSS Support: 4+ bytes
    // Total HE data: 21+ bytes
    let he_data: Vec<u8> = vec![
        // HE MAC Capabilities Info (6 bytes)
        0x0C, 0x00, 0x00, 0x00, 0x00, 0x00,
        // HE PHY Capabilities Info (11 bytes)
        0x0C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        // HE TX RX MCS NSS Support (4 bytes minimum)
        0xFF, 0x00, 0xFF, 0x00,
    ];
    ie_data.push(0xFF);  // Extended IE ID
    ie_data.push(1 + he_data.len() as u8);  // Length = ExtID(1) + data
    ie_data.push(0x23);  // Extension ID 35 (HE Capabilities)
    ie_data.extend_from_slice(&he_data);

    let beacon = create_test_beacon(ie_data, 36, Band::Ghz5);
    let network = parse_beacon(&beacon);

    assert_eq!(network.ssid, Some("WiFi6-AX".to_string()));
    assert!(network.standards.contains(&"ax".to_string()), "Expected 'ax' in standards, got {:?}", network.standards);
    assert_eq!(network.wifi_generation, 6);
    assert_eq!(network.features.max_qam, 1024);
    assert!(network.features.ofdma);
}

// =============================================================================
// WiFi 7 (802.11be) Tests
// =============================================================================

#[test]
fn test_wifi7_beacon_parsing() {
    let mut ie_data = Vec::new();

    // SSID
    ie_data.extend_from_slice(&[0x00, 0x08]);
    ie_data.extend_from_slice(b"WiFi7-BE");

    // DS Parameter Set (channel 36)
    ie_data.extend_from_slice(&[0x03, 0x01, 0x24]);

    // HT Capabilities
    ie_data.extend_from_slice(&[
        0x2D, 0x1A, 0x6C, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ]);

    // VHT Capabilities
    ie_data.extend_from_slice(&[
        0xBF, 0x0C, 0xB2, 0x01, 0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ]);

    // HE Capabilities (WiFi 6) - Extended IE
    let he_data: Vec<u8> = vec![
        0x0C, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00,
    ];
    ie_data.push(0xFF);
    ie_data.push(1 + he_data.len() as u8);
    ie_data.push(0x23);
    ie_data.extend_from_slice(&he_data);

    // EHT Capabilities (WiFi 7) - Extended IE
    // Format: ID(255) + Length + ExtID(108) + Data
    let eht_data: Vec<u8> = vec![
        0x00, 0x00, 0x00, 0x00,  // MAC Capabilities (4 bytes)
        0x03, 0x01, 0x00, 0x00,  // PHY Capabilities (320MHz + 4096-QAM)
        0xFF, 0xFF, 0xFF, 0xFF,  // MCS map
    ];
    ie_data.push(0xFF);
    ie_data.push(1 + eht_data.len() as u8);
    ie_data.push(0x6C);  // Extension ID 108 (EHT Capabilities)
    ie_data.extend_from_slice(&eht_data);

    let beacon = create_test_beacon(ie_data, 36, Band::Ghz5);
    let network = parse_beacon(&beacon);

    assert_eq!(network.ssid, Some("WiFi7-BE".to_string()));
    assert!(network.standards.contains(&"be".to_string()), "Expected 'be' in standards, got {:?}", network.standards);
    assert_eq!(network.wifi_generation, 7);
    assert_eq!(network.features.max_qam, 4096);
    assert!(network.features.ofdma);
}

// =============================================================================
// Security Tests
// =============================================================================

#[test]
fn test_wpa2_psk_beacon() {
    let mut ie_data = Vec::new();

    // SSID (8 bytes)
    ie_data.extend_from_slice(&[0x00, 0x08]);
    ie_data.extend_from_slice(b"WPA2-PSK");

    // DS Parameter Set
    ie_data.extend_from_slice(&[0x03, 0x01, 0x06]);

    // RSN IE (WPA2-PSK)
    // Length = 20 bytes of data after ID and length
    ie_data.extend_from_slice(&[
        0x30, 0x14,  // ID 48, Length 20
        0x01, 0x00,  // Version 1
        0x00, 0x0F, 0xAC, 0x04,  // Group cipher: CCMP (OUI 00-0F-AC, type 4)
        0x01, 0x00,  // 1 pairwise cipher
        0x00, 0x0F, 0xAC, 0x04,  // CCMP
        0x01, 0x00,  // 1 auth suite
        0x00, 0x0F, 0xAC, 0x02,  // PSK (type 2)
        0x00, 0x00,  // RSN capabilities
    ]);

    let beacon = create_test_beacon(ie_data, 6, Band::Ghz2_4);
    let network = parse_beacon(&beacon);

    assert_eq!(network.ssid, Some("WPA2-PSK".to_string()));
    assert_eq!(network.security, "wpa2");
    assert_eq!(network.security_details.auth_method, "psk");
    assert!(!network.security_details.is_enterprise);
}

#[test]
fn test_wpa3_sae_beacon() {
    let mut ie_data = Vec::new();

    // SSID (8 bytes)
    ie_data.extend_from_slice(&[0x00, 0x08]);
    ie_data.extend_from_slice(b"WPA3-SAE");

    // DS Parameter Set
    ie_data.extend_from_slice(&[0x03, 0x01, 0x24]);

    // RSN IE (WPA3-SAE)
    ie_data.extend_from_slice(&[
        0x30, 0x14,  // ID 48, Length 20
        0x01, 0x00,  // Version 1
        0x00, 0x0F, 0xAC, 0x04,  // Group cipher: CCMP
        0x01, 0x00,  // 1 pairwise cipher
        0x00, 0x0F, 0xAC, 0x04,  // CCMP
        0x01, 0x00,  // 1 auth suite
        0x00, 0x0F, 0xAC, 0x04,  // SAE (type 4 = WPA3 in our parser)
        0x80, 0x00,  // RSN capabilities with PMF capable
    ]);

    let beacon = create_test_beacon(ie_data, 36, Band::Ghz5);
    let network = parse_beacon(&beacon);

    assert_eq!(network.ssid, Some("WPA3-SAE".to_string()));
    assert_eq!(network.security, "wpa3");
    assert!(network.security_details.sae);
}

// =============================================================================
// Channel Width Tests
// =============================================================================

#[test]
fn test_80mhz_channel_width() {
    let mut ie_data = Vec::new();

    // SSID
    ie_data.extend_from_slice(&[0x00, 0x06]);
    ie_data.extend_from_slice(b"80MHz!");

    // DS Parameter Set (channel 36)
    ie_data.extend_from_slice(&[0x03, 0x01, 0x24]);

    // VHT Operation (80MHz)
    ie_data.extend_from_slice(&[
        0xC0, 0x03,  // ID 192, Length 3
        0x01,        // Channel width: 80MHz
        0x2A,        // Center frequency segment 0 = 42
        0x00,        // Center frequency segment 1 = 0
    ]);

    let beacon = create_test_beacon(ie_data, 36, Band::Ghz5);
    let network = parse_beacon(&beacon);

    assert_eq!(network.channel_width, 80);
    assert!(network.features.channel_info.is_some());
    let ch_info = network.features.channel_info.unwrap();
    assert_eq!(ch_info.center_freq_0, Some(42));
}

#[test]
fn test_160mhz_channel_width() {
    let mut ie_data = Vec::new();

    // SSID
    ie_data.extend_from_slice(&[0x00, 0x07]);
    ie_data.extend_from_slice(b"160MHz!");

    // DS Parameter Set (channel 100)
    ie_data.extend_from_slice(&[0x03, 0x01, 0x64]);

    // VHT Operation (160MHz)
    ie_data.extend_from_slice(&[
        0xC0, 0x03,
        0x02,        // Channel width: 160MHz
        0x72,        // Center frequency segment 0 = 114
        0x00,
    ]);

    let beacon = create_test_beacon(ie_data, 100, Band::Ghz5);
    let network = parse_beacon(&beacon);

    assert_eq!(network.channel_width, 160);
}
