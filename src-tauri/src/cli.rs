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
            // Detailed spatial stream info
            if let Some(ss_info) = &net.features.spatial_stream_info {
                if let Some(tx) = ss_info.tx_streams {
                    println!("TX Streams:   {}", tx);
                }
                if let Some(rx) = ss_info.rx_streams {
                    println!("RX Streams:   {}", rx);
                }
            }
            println!();
            println!("=== Channel Details ===");
            if let Some(ch_info) = &net.features.channel_info {
                println!("Primary:      CH {}", ch_info.primary);
                println!("Bandwidth:    {} MHz", ch_info.bandwidth.as_mhz());
                if let Some(sec) = ch_info.secondary {
                    println!("Secondary:    CH {}", sec);
                }
                if let Some(offset) = &ch_info.secondary_offset {
                    println!("Sec Offset:   {:?}", offset);
                }
                if let Some(cf0) = ch_info.center_freq_0 {
                    println!("Center Freq 0: {} MHz", cf0 * 5 + 5000);
                }
                if let Some(cf1) = ch_info.center_freq_1 {
                    println!("Center Freq 1: {} MHz", cf1 * 5 + 5000);
                }
            } else {
                println!("Bandwidth:    {} MHz", net.channel_width);
            }
            println!();
            println!("=== OFDMA & TWT ===");
            println!("OFDMA:        {}", if net.features.ofdma { "✓" } else { "✗" });
            if let Some(ofdma) = &net.features.ofdma_info {
                println!("DL OFDMA:     {}", if ofdma.dl_ofdma { "✓" } else { "✗" });
                println!("UL OFDMA:     {}", if ofdma.ul_ofdma { "✓" } else { "✗" });
                if !ofdma.ru_sizes.is_empty() {
                    let ru_names: Vec<String> = ofdma.ru_sizes.iter().map(|ru| match ru {
                        RuSize::R26 => "26",
                        RuSize::R52 => "52",
                        RuSize::R106 => "106",
                        RuSize::R242 => "242",
                        RuSize::R484 => "484",
                        RuSize::R996 => "996",
                        RuSize::R996x2 => "996x2",
                    }).map(|s| format!("{}-tone", s)).collect();
                    println!("RU Sizes:     {}", ru_names.join(", "));
                }
            }
            if let Some(twt) = &net.features.twt_info {
                println!("TWT:          ✓");
                println!("Broadcast TWT: {}", if twt.broadcast_twt { "✓" } else { "✗" });
                println!("Individual TWT: {}", if twt.individual_twt { "✓" } else { "✗" });
                println!("TWT Requester: {}", if twt.twt_requester { "✓" } else { "✗" });
                println!("TWT Responder: {}", if twt.twt_responder { "✓" } else { "✗" });
            }
            println!();
            println!("=== WiFi 7 Features ===");
            println!("MLO:          {}", if net.features.mlo { "✓" } else { "✗" });
            if let Some(w7) = &net.features.wifi7_features {
                if let Some(mlo) = &w7.mlo {
                    println!("MLO Links:    {}", mlo.num_links);
                }
                println!("Punctured Preamble: {}", if w7.punctured_preamble { "✓" } else { "✗" });
                println!("Multi-RU:     {}", if w7.multi_ru { "✓" } else { "✗" });
            }
            println!();
            println!("=== MCS & Modulation ===");
            println!("Max QAM:      {}", net.features.max_qam);
            if let Some(mcs) = &net.features.mcs_info {
                if let Some(max_mcs) = mcs.max_mcs {
                    println!("Max MCS:      {}", max_mcs);
                }
                if let Some(modulation) = &mcs.max_modulation {
                    println!("Modulation:   {:?}", modulation);
                }
            }
            println!("BSS Coloring: {}", if net.features.bss_coloring { "✓" } else { "✗" });
            println!("Guard Interval: {} ns", net.features.guard_interval);
            println!();
            println!("=== Security ===");
            println!("Type:         {}", net.security);
            println!("Auth Method:  {}", net.security_details.auth_method);
            println!("Cipher:       {}", net.security_details.cipher);
            if let Some(group) = &net.security_details.group_cipher {
                println!("Group Cipher: {}", group);
            }
            if !net.security_details.pairwise_ciphers.is_empty() {
                println!("Pairwise:     {}", net.security_details.pairwise_ciphers.join(", "));
            }
            println!("SAE (WPA3):   {}", if net.security_details.sae { "✓" } else { "✗" });
            println!("OWE:          {}", if net.security_details.owe { "✓" } else { "✗" });
            println!("PMF Capable:  {}", if net.security_details.pmf_capable { "✓" } else { "✗" });
            println!("PMF Required: {}", if net.security_details.pmf_required { "✓" } else { "✗" });
            println!("WPA3 Transition: {}", if net.security_details.is_wpa3_transition { "✓" } else { "✗" });
            println!();
            println!("=== Roaming Protocols ===");
            println!("802.11k (RRM): {}", if net.protocols.rrm { "✓" } else { "✗" });
            println!("  Neighbor Report: {}", if net.protocols.neighbor_report { "✓" } else { "✗" });
            println!("  Beacon Report:   {}", if net.protocols.beacon_report { "✓" } else { "✗" });
            println!("802.11r (FT):  {}", if net.protocols.ft { "✓" } else { "✗" });
            println!("  FT over DS:      {}", if net.protocols.ft_over_ds { "✓" } else { "✗" });
            println!("  FT Resource Req: {}", if net.protocols.ft_resource_request { "✓" } else { "✗" });
            println!("802.11v (BSS): {}", if net.protocols.bss_transition { "✓" } else { "✗" });
            println!("  WNM Sleep:       {}", if net.protocols.wnm_sleep { "✓" } else { "✗" });
            println!("802.11w (PMF): {}", if net.protocols.pmf { "✓" } else { "✗" });
            println!("WMM:           {}", if net.protocols.wmm { "✓" } else { "✗" });
            println!("  U-APSD:          {}", if net.protocols.wmm_uapsd { "✓" } else { "✗" });

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
