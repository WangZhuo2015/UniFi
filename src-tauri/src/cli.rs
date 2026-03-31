//! UniFi CLI
//!
//! Command-line interface for WiFi scanning.

use clap::{Parser, Subcommand};
use std::time::Instant;

use crate::types::*;
use crate::scanner::{get_scanner_with_mode, parse_scanner_mode};
use crate::parser::{parse_beacon, parse_all_ies};

#[derive(Parser)]
#[command(name = "unifi")]
#[command(about = "Professional WiFi analysis tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan for WiFi networks
    Scan {
        /// Output format: table, json, csv
        #[arg(short, long, default_value = "table")]
        format: String,

        /// Filter by band: 2.4, 5, 6, all
        #[arg(short, long, default_value = "all")]
        band: String,

        /// Show IE details
        #[arg(long)]
        ie: bool,

        /// Scanner to use: default, corewlan, airport, libpcap
        #[arg(short, long, default_value = "default")]
        scanner: String,
    },

    /// Show current connection
    Current {
        /// Scanner to use: default, corewlan, airport, libpcap
        #[arg(short, long, default_value = "default")]
        scanner: String,
    },

    /// Show detailed info for a network
    Info {
        /// BSSID of the network
        bssid: String,

        /// Scanner to use: default, corewlan, airport, libpcap
        #[arg(short, long, default_value = "default")]
        scanner: String,
    },

    /// Parse IE data from hex string
    ParseIe {
        /// Hex-encoded IE data
        data: String,
    },

    /// List available scanners
    Scanners {
        /// Show detailed diagnostic info
        #[arg(short, long)]
        verbose: bool,
    },
}

pub fn run() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan { format, band, ie, scanner } => {
            cmd_scan(&format, &band, ie, &scanner);
        }
        Commands::Current { scanner } => {
            cmd_current(&scanner);
        }
        Commands::Info { bssid, scanner } => {
            cmd_info(&bssid, &scanner);
        }
        Commands::ParseIe { data } => {
            cmd_parse_ie(&data);
        }
        Commands::Scanners { verbose } => {
            cmd_scanners(verbose);
        }
    }
}

fn cmd_scan(format: &str, band: &str, show_ie: bool, scanner_name: &str) {
    let mode = parse_scanner_mode(scanner_name);
    let scanner = get_scanner_with_mode(mode);
    eprintln!("Using scanner: {}", scanner.name());

    if scanner.requires_privilege() {
        eprintln!("Note: This scanner requires root/admin privileges");
    }

    let start = Instant::now();
    let beacons = match scanner.scan() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let duration = start.elapsed();

    // Filter by band
    let filtered: Vec<_> = beacons.iter()
        .filter(|b| {
            if band == "all" { true }
            else { b.band.as_str() == band }
        })
        .collect();

    match format {
        "json" => print_json(&filtered, show_ie),
        "csv" => print_csv(&filtered),
        _ => print_table(&filtered, show_ie),
    }

    eprintln!("\nScanned {} networks in {:?}", filtered.len(), duration);
}

fn print_table(beacons: &[&RawBeacon], show_ie: bool) {
    println!("{:<32} {:<18} {:<4} {:<6} {:<9} {:<6}",
        "SSID", "BSSID", "Ch", "Band", "Signal", "Standard");
    println!("{}", "-".repeat(80));

    for b in beacons {
        let ssid = b.ssid_string().unwrap_or("[Hidden]".into());
        let net = parse_beacon(b);

        // Show highest standard (last in the list)
        let default_std = "?".to_string();
        let highest_std = net.standards.last().unwrap_or(&default_std);

        println!("{:<32} {:<18} {:<4} {:<6} {:>4} dBm  {:<6}",
            truncate(&ssid, 32),
            b.bssid_string(),
            b.channel,
            b.band,
            b.signal_dbm,
            highest_std,
        );

        if show_ie && !b.ie_data.is_empty() {
            let ie = parse_all_ies(&b.ie_data);
            println!("  Standard: {}", ie.detection_summary.detected_standard);
            println!("  IE Count: {}", ie.elements.len());
        }
    }
}

fn print_json(beacons: &[&RawBeacon], _show_ie: bool) {
    let networks: Vec<_> = beacons.iter().map(|b| parse_beacon(b)).collect();
    println!("{}", serde_json::to_string_pretty(&networks).unwrap());
}

fn print_csv(beacons: &[&RawBeacon]) {
    println!("SSID,BSSID,Channel,Band,Signal,Standard");
    for b in beacons {
        let ssid = b.ssid_string().unwrap_or_default();
        let net = parse_beacon(b);
        println!("{},{},{},{},{},{}",
            ssid,
            b.bssid_string(),
            b.channel,
            b.band,
            b.signal_dbm,
            net.standards.first().unwrap_or(&"?".into()),
        );
    }
}

fn cmd_current(scanner_name: &str) {
    let mode = parse_scanner_mode(scanner_name);
    let scanner = get_scanner_with_mode(mode);

    eprintln!("Using scanner: {}", scanner.name());

    match scanner.current() {
        Ok(Some(b)) => {
            let net = parse_beacon(&b);
            println!("Connected to: {}", net.ssid.unwrap_or_default());
            println!("BSSID:        {}", b.bssid_string());
            println!("Channel:      {} ({} GHz)", b.channel, b.band);
            println!("Signal:       {} dBm", b.signal_dbm);
            println!("Standard:     {:?}", net.standards);
            println!("Security:     {}", net.security);
            println!("AP Streams:   {}", net.features.spatial_streams);
            println!("Current Width: {} MHz", net.channel_width);
            println!("AP Max Width: {} MHz", net.features.max_supported_width);
            println!("AP Current Peak: {} Mbps", net.max_data_rate);
            println!("AP Max Peak:     {} Mbps", net.ap_peak_data_rate);
            if let Some(client_peak) = net.client_peak_data_rate {
                println!("Local Peak:      {} Mbps", client_peak);
            }
            if let Some(client_streams) = net.client_spatial_streams {
                println!("Local Streams:   {}", client_streams);
            }
            if let Some(local_adapter) = &net.local_adapter {
                println!("Local Max Width: {} MHz", local_adapter.max_supported_width);
                println!("Local Standards: {:?}", local_adapter.supported_standards);
                println!("Local Adapter:   {}", local_adapter.driver_name);
            }
            if let Some(link_rates) = &net.link_rates {
                if let Some(rx_rate) = link_rates.rx_rate_mbps {
                    println!("Link RX:      {:.1} Mbps", rx_rate);
                }
                if let Some(tx_rate) = link_rates.tx_rate_mbps {
                    println!("Link TX:      {:.1} Mbps", tx_rate);
                }
            }
        }
        Ok(None) => {
            println!("Not connected to any network");
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_info(bssid: &str, scanner_name: &str) {
    let mode = parse_scanner_mode(scanner_name);
    let scanner = get_scanner_with_mode(mode);

    eprintln!("Using scanner: {}", scanner.name());

    let beacons = match scanner.scan() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    for b in beacons {
        if b.bssid_string().to_uppercase() == bssid.to_uppercase() {
            let net = parse_beacon(&b);

            println!("=== {} ===", net.ssid.as_deref().unwrap_or("[Hidden]"));
            println!();
            println!("BSSID:        {}", b.bssid_string());
            println!("Channel:      {} ({} GHz)", b.channel, b.band);
            println!("Frequency:    {} MHz", b.frequency());
            println!("Signal:       {} dBm", b.signal_dbm);
            println!("SNR:          {} dB", b.snr());
            println!();
            println!("=== WiFi Standard ===");
            println!("Standards:    {:?}", net.standards);
            println!("Channel Width: {} MHz", net.channel_width);
            println!("Max Width:    {} MHz", net.features.max_supported_width);
            println!("AP Streams:   {}", net.features.spatial_streams);
            println!("AP Current Peak: {} Mbps", net.max_data_rate);
            println!("AP Max Peak:     {} Mbps", net.ap_peak_data_rate);
            if let Some(client_peak) = net.client_peak_data_rate {
                println!("Local Peak:      {} Mbps", client_peak);
            }
            if let Some(client_streams) = net.client_spatial_streams {
                println!("Local Streams:   {}", client_streams);
            }
            if let Some(local_adapter) = &net.local_adapter {
                println!("Local Max Width: {} MHz", local_adapter.max_supported_width);
                println!("Local Standards: {:?}", local_adapter.supported_standards);
                println!("Local Adapter:   {}", local_adapter.driver_name);
            }
            println!();
            println!("=== MIMO & Beamforming ===");
            println!("Spatial Streams: {}", net.features.spatial_streams);
            println!("SU-MIMO:       {}", if net.features.su_mimo { "✓" } else { "✗" });
            println!("MU-MIMO (DL):  {}", if net.features.mu_mimo { "✓" } else { "✗" });
            println!("MU-MIMO (UL):  {}", if net.features.ul_mu_mimo { "✓" } else { "✗" });
            println!("SU Beamformer: {}", if net.features.su_beamformer { "✓" } else { "✗" });
            println!("SU Beamformee: {}", if net.features.su_beamformee { "✓" } else { "✗" });
            println!("MU Beamformer: {}", if net.features.mu_beamformer { "✓" } else { "✗" });
            println!();
            println!("=== Other Features ===");
            println!("OFDMA:        {}", if net.features.ofdma { "✓" } else { "✗" });
            println!("MLO:          {}", if net.features.mlo { "✓" } else { "✗" });
            println!("Max QAM:      {}", net.features.max_qam);
            println!("BSS Coloring: {}", if net.features.bss_coloring { "✓" } else { "✗" });
            println!("Guard Interval: {} ns", net.features.guard_interval);
            println!();
            println!("=== Security ===");
            println!("Type:         {}", net.security);
            println!("Auth Method:  {}", net.security_details.auth_method);
            println!("Cipher:       {}", net.security_details.cipher);
            println!();
            println!("=== Protocols ===");
            println!("802.11k (RRM): {}", net.protocols.rrm);
            println!("802.11r (FT):  {}", net.protocols.ft);
            println!("802.11v (BSS): {}", net.protocols.bss_transition);
            println!("802.11w (PMF): {}", net.protocols.pmf);

            if !b.ie_data.is_empty() {
                println!();
                println!("=== IE Details ===");
                let ie = parse_all_ies(&b.ie_data);
                println!("Detection: {}", ie.detection_summary.detected_standard);
                println!("IE Count:  {}", ie.elements.len());
            }

            return;
        }
    }

    eprintln!("Network not found: {}", bssid);
    std::process::exit(1);
}

fn cmd_parse_ie(data: &str) {
    let bytes = match hex::decode(data) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Invalid hex data: {}", e);
            std::process::exit(1);
        }
    };

    let ie = parse_all_ies(&bytes);

    println!("=== IE Details ===");
    println!("Total Length: {} bytes", ie.total_length);
    println!("Detection: {}", ie.detection_summary.detected_standard);
    println!();

    println!("{:<4} {:<6} {:<24} {:<6}", "ID", "Hex", "Name", "Len");
    println!("{}", "-".repeat(50));

    for e in &ie.elements {
        println!("{:<4} {:<6} {:<24} {:<6}",
            e.element_id,
            e.element_id_hex,
            e.name,
            e.length,
        );
    }
}

fn cmd_scanners(verbose: bool) {
    println!("Available scanners:");
    println!();

    // Check each scanner with detailed info
    #[cfg(target_os = "macos")]
    {
        use crate::scanner::ScannerMode;

        // CoreWLAN
        let corewlan = crate::scanner::get_scanner_with_mode(ScannerMode::CoreWLAN);
        let cw_avail = corewlan.is_available();
        println!("  {} CoreWLAN{}", if cw_avail { "✓" } else { "✗" }, if cw_avail { "" } else { " (unavailable)" });
        if verbose {
            println!("      - App Store compatible");
            println!("      - No IE data (no WiFi standard detection)");
            println!("      - BSSID requires Location permission");
        }

        // Airport
        let airport = crate::scanner::get_scanner_with_mode(ScannerMode::Airport);
        let ap_avail = airport.is_available();
        println!("  {} Airport{}", if ap_avail { "✓" } else { "✗" }, if ap_avail { "" } else { " (unavailable)" });
        if verbose {
            println!("      - Full IE data (WiFi 4/5/6/7 detection)");
            println!("      - May not work on macOS 26+");
        }

        // Libpcap
        let libpcap = crate::scanner::get_scanner_with_mode(ScannerMode::Libpcap);
        let lp_avail = libpcap.is_available();
        let lp_reason = if !lp_avail {
            // Get the reason
            let is_root = unsafe { libc::getuid() == 0 };
            if !is_root {
                " (requires root/sudo)"
            } else {
                " (no WiFi interface found)"
            }
        } else {
            ""
        };
        println!("  {} Libpcap{}", if lp_avail { "✓" } else { "✗" }, lp_reason);
        if verbose {
            println!("      - Captures raw 802.11 beacon frames");
            println!("      - Full IE data (WiFi 4/5/6/7 detection)");
            println!("      - Works on macOS 26+");
            println!("      - Cannot be used on App Store");
            if !lp_avail {
                println!("      - Run with: sudo unifi-cli scan --scanner libpcap");
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        println!("  ✓ nl80211");
        println!("  ✗ Libpcap (requires root/sudo)");
    }

    #[cfg(target_os = "windows")]
    {
        println!("  ✓ WlanAPI");
    }

    println!();
    println!("Usage: unifi scan --scanner <name>");
    println!();

    if !verbose {
        println!("Use --verbose for more details");
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max-3])
    } else {
        s.to_string()
    }
}
