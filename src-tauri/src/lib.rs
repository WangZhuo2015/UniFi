//! UniFi - Multi-platform WiFi Analysis
//!
//! Supports GUI (Tauri) and CLI modes.

mod types;
mod scanner;
mod parser;
mod vendor;
mod process;
pub mod roaming;

pub mod cli;

pub use types::*;
pub use scanner::{get_scanner, get_scanner_with_mode, list_scanners, ScannerMode};
pub use roaming::{RoamingMonitor, RoamingTestConfig, RoamingTestResult, PingConfig};

// ============================================================================
// GUI Mode (Tauri)
// ============================================================================

#[cfg(feature = "gui")]
mod gui {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex as StdMutex;
    use tauri::Emitter;
    use scanner::{get_scanner, parse_scanner_mode};
    use parser::{parse_beacon, parse_all_ies};
    use roaming::{RoamingMonitor, RoamingTestConfig, PingConfig};

    static MONITORING: AtomicBool = AtomicBool::new(false);
    static ROAMING_MONITOR: std::sync::OnceLock<StdMutex<RoamingMonitor>> = std::sync::OnceLock::new();

    fn get_roaming_monitor() -> &'static StdMutex<RoamingMonitor> {
        ROAMING_MONITOR.get_or_init(|| StdMutex::new(RoamingMonitor::default()))
    }

    #[tauri::command]
    pub async fn scan_networks() -> Result<Vec<Network>, String> {
        tauri::async_runtime::spawn_blocking(scanner::scan_networks)
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn scan_networks_with_scanner(scanner_name: String) -> Result<Vec<Network>, String> {
        let mode = parse_scanner_mode(&scanner_name);
        tauri::async_runtime::spawn_blocking(move || scanner::scan_networks_with_mode(mode))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub async fn current_network() -> Result<Option<Network>, String> {
        tauri::async_runtime::spawn_blocking(move || {
            let scanner = get_scanner();
            scanner.current().map(|opt| opt.map(|b| parse_beacon(&b)))
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub fn get_network_groups() -> Result<Vec<NetworkGroup>, String> {
        let networks = scanner::scan_networks().map_err(|e| e.to_string())?;
        Ok(group_networks(networks))
    }

    #[tauri::command]
    pub fn get_scan_stats() -> Result<ScanStats, String> {
        let start = std::time::Instant::now();
        let networks = scanner::scan_networks().map_err(|e| e.to_string())?;
        Ok(compute_stats(networks, start.elapsed().as_millis() as u64))
    }

    #[tauri::command]
    pub fn get_ie_details(bssid: String) -> Option<IEDetails> {
        let scanner = get_scanner();
        let beacons = scanner.scan().ok()?;

        for beacon in beacons {
            if beacon.bssid_string().to_uppercase() == bssid.to_uppercase() {
                return Some(parse_all_ies(&beacon.ie_data));
            }
        }
        None
    }

    #[tauri::command]
    pub fn lookup_vendor(bssid: String) -> Option<String> {
        Some(vendor::lookup_vendor(&bssid))
    }

    #[tauri::command]
    pub fn list_available_scanners() -> Vec<ScannerInfo> {
        let scanners = scanner::list_scanners();
        scanners.into_iter().map(|(name, available, requires_root)| ScannerInfo {
            name: name.to_string(),
            available,
            requires_root,
        }).collect()
    }

    #[tauri::command]
    pub fn start_monitor(app: tauri::AppHandle) -> Result<(), String> {
        MONITORING.store(true, Ordering::SeqCst);

        let app_handle = app.clone();
        std::thread::spawn(move || {
            while MONITORING.load(Ordering::SeqCst) {
                if let Ok(networks) = scanner::scan_networks() {
                    let _ = app_handle.emit("networks-updated", networks);
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        });

        Ok(())
    }

    #[tauri::command]
    pub fn stop_monitor() -> Result<(), String> {
        MONITORING.store(false, Ordering::SeqCst);
        Ok(())
    }

    // ========================================================================
    // Roaming Test Commands
    // ========================================================================

    #[tauri::command]
    pub fn start_roaming_test(
        target: String,
        duration_secs: u64,
        interval_ms: u64,
    ) -> Result<(), String> {
        let config = RoamingTestConfig {
            ping: PingConfig {
                target,
                interval_ms,
                timeout_ms: 1000,
                packet_size: 64,
            },
            duration_secs,
            ..Default::default()
        };

        let monitor = get_roaming_monitor();
        let mut monitor = monitor.lock().map_err(|e| e.to_string())?;

        // Update with new config
        *monitor = RoamingMonitor::new(config);
        monitor.start()
    }

    #[tauri::command]
    pub fn stop_roaming_test() -> Result<RoamingTestResult, String> {
        let monitor = get_roaming_monitor();
        let monitor = monitor.lock().map_err(|e| e.to_string())?;
        Ok(monitor.stop())
    }

    #[tauri::command]
    pub fn get_roaming_test_status() -> Result<(bool, u32, u32), String> {
        let monitor = get_roaming_monitor();
        let monitor = monitor.lock().map_err(|e| e.to_string())?;
        let running = monitor.is_running();
        let (current, total) = monitor.get_progress();
        Ok((running, current, total))
    }

    #[tauri::command]
    pub fn get_roaming_test_results() -> Result<RoamingTestResult, String> {
        let monitor = get_roaming_monitor();
        let monitor = monitor.lock().map_err(|e| e.to_string())?;
        Ok(monitor.get_results())
    }

    pub fn run() {
        tauri::Builder::default()
            .plugin(tauri_plugin_opener::init())
            .invoke_handler(tauri::generate_handler![
                scan_networks,
                scan_networks_with_scanner,
                current_network,
                get_network_groups,
                get_scan_stats,
                get_ie_details,
                lookup_vendor,
                list_available_scanners,
                start_monitor,
                stop_monitor,
                start_roaming_test,
                stop_roaming_test,
                get_roaming_test_status,
                get_roaming_test_results,
            ])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }

    fn group_networks(networks: Vec<Network>) -> Vec<NetworkGroup> {
        let mut groups: std::collections::HashMap<String, NetworkGroup> = std::collections::HashMap::new();

        for net in networks {
            let key = net.ssid.clone().unwrap_or_else(|| "[Hidden]".into());

            let group = groups.entry(key.clone()).or_insert(NetworkGroup {
                ssid: key,
                networks: vec![],
                total_aps: 0,
                bands: vec![],
                best_signal: -100,
                supports_fast_roaming: false,
                supports_bss_transition: false,
            });

            group.networks.push(net.clone());
            group.total_aps += 1;

            if !group.bands.contains(&net.band) {
                group.bands.push(net.band.clone());
            }

            if net.signal > group.best_signal {
                group.best_signal = net.signal;
            }

            if net.protocols.ft { group.supports_fast_roaming = true; }
            if net.protocols.bss_transition { group.supports_bss_transition = true; }
        }

        groups.into_values().collect()
    }

    fn compute_stats(networks: Vec<Network>, duration_ms: u64) -> ScanStats {
        use std::collections::HashMap;

        let mut stats = ScanStats {
            total_networks: networks.len() as u32,
            hidden_networks: 0,
            network_groups: 0,
            by_band: HashMap::new(),
            by_security: HashMap::new(),
            by_standard: HashMap::new(),
            scan_duration_ms: duration_ms,
        };

        let mut seen_ssids: std::collections::HashSet<String> = std::collections::HashSet::new();

        for net in &networks {
            if net.is_hidden { stats.hidden_networks += 1; }
            if let Some(ref ssid) = net.ssid { seen_ssids.insert(ssid.clone()); }

            *stats.by_band.entry(net.band.clone()).or_insert(0) += 1;
            *stats.by_security.entry(net.security.clone()).or_insert(0) += 1;

            for std in &net.standards {
                *stats.by_standard.entry(std.clone()).or_insert(0) += 1;
            }
        }

        stats.network_groups = seen_ssids.len() as u32;
        stats
    }
}

/// Scanner info for GUI
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ScannerInfo {
    pub name: String,
    pub available: bool,
    pub requires_root: bool,
}

#[cfg(feature = "gui")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    gui::run();
}
