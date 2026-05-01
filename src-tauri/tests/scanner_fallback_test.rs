//! Tests for scanner fallback mechanism on macOS 26+

#[cfg(target_os = "macos")]
mod tests {
    use unifi_lib::{get_scanner, get_scanner_with_mode, list_scanners, ScannerMode};

    #[test]
    fn test_corewlan_always_available() {
        let scanner = get_scanner_with_mode(ScannerMode::CoreWLAN);
        assert!(scanner.is_available(), "CoreWLAN should always be available on macOS");
        assert_eq!(scanner.name(), "macOS CoreWLAN");
    }

    #[test]
    fn test_scanner_list_corewlan_available() {
        let scanners = list_scanners();
        let corewlan = scanners.iter().find(|(name, _, _)| *name == "CoreWLAN");
        assert!(corewlan.is_some(), "CoreWLAN should be in scanner list");
        let (_, available, _) = corewlan.unwrap();
        assert!(*available, "CoreWLAN should be marked as available");
    }

    #[test]
    fn test_default_scanner_works() {
        // Default scanner should always work, even on macOS 26
        let scanner = get_scanner();
        assert!(scanner.is_available(), "Default scanner must be available");
    }

    #[test]
    fn test_airport_scanner_name() {
        let scanner = get_scanner_with_mode(ScannerMode::Airport);
        assert_eq!(scanner.name(), "macOS Airport (Legacy)");
        // is_available depends on macOS version
    }

    #[test]
    fn test_libpcap_requires_privilege() {
        let scanner = get_scanner_with_mode(ScannerMode::Libpcap);
        assert!(scanner.requires_privilege(), "Libpcap should require root");
        // is_available depends on root privilege
    }

    #[test]
    fn test_scanner_list_includes_all() {
        let scanners = list_scanners();
        assert!(scanners.iter().any(|(n, _, _)| *n == "CoreWLAN"));
        assert!(scanners.iter().any(|(n, _, _)| *n == "Airport"));
        assert!(scanners.iter().any(|(n, _, _)| *n == "Libpcap"));
    }
}

#[cfg(not(target_os = "macos"))]
mod tests {
    use unifi_lib::{get_scanner, list_scanners};

    #[test]
    fn test_default_scanner_works() {
        let scanner = get_scanner();
        // On other platforms, just ensure it doesn't crash
        let _ = scanner.name();
    }

    #[test]
    fn test_scanner_list_not_empty() {
        let scanners = list_scanners();
        assert!(!scanners.is_empty(), "Scanner list should not be empty");
    }
}