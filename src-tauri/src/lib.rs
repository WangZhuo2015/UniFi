//! UniFi - Multi-platform WiFi Analysis
//!
//! Supports GUI (Tauri) and CLI modes.

mod types;
mod scanner;
mod parser;
mod vendor;

#[cfg(feature = "cli")]
pub mod cli;

pub use types::*;
pub use scanner::get_scanner;

// ============================================================================
// GUI Mode (Tauri)
// ============================================================================

#[cfg(feature = "gui")]
mod gui {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tauri::Emitter;
    use scanner::{Scanner, get_scanner};
    use parser::{parse_beacon, parse_all_ies};

    static MONITORING: AtomicBool = AtomicBool::new(false);

    #[tauri::command]
    pub fn scan_networks() -> Result<Vec<Network>, String> {
        scanner::scan_networks().map_err(|e| e.to_string())
    }

    #[tauri::command]
    pub fn current_network() -> Result<Option<Network>, String> {
        let scanner = get_scanner();
        scanner.current()
            .map_err(|e| e.to_string())
            .map(|opt| opt.map(|b| parse_beacon(&b)))
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

    pub fn run() {
        tauri::Builder::default()
            .plugin(tauri_plugin_opener::init())
            .invoke_handler(tauri::generate_handler![
                scan_networks,
                current_network,
                get_network_groups,
                get_scan_stats,
                get_ie_details,
                lookup_vendor,
                start_monitor,
                stop_monitor,
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

#[cfg(feature = "gui")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    gui::run();
}
