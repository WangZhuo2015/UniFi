//! UniFi CLI
//!
//! Command-line interface for WiFi scanning.

use clap::{Parser, Subcommand};
use std::time::Instant;

use crate::types::*;
use crate::scanner::{get_scanner, Scanner};
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
    },
    
    /// Show current connection
    Current,
    
    /// Show detailed info for a network
    Info {
        /// BSSID of the network
        bssid: String,
    },
    
    /// Parse IE data from hex string
    ParseIe {
        /// Hex-encoded IE data
        data: String,
    },
    
    /// List available scanners
    Scanners,
}

pub fn run() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Scan { format, band, ie } => {
            cmd_scan(&format, &band, ie);
        }
        Commands::Current => {
            cmd_current();
        }
        Commands::Info { bssid } => {
            cmd_info(&bssid);
        }
        Commands::ParseIe { data } => {
            cmd_parse_ie(&data);
        }
        Commands::Scanners => {
            cmd_scanners();
        }
    }
}

fn cmd_scan(format: &str, band: &str, show_ie: bool) {
    let scanner = get_scanner();
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
    println!("{:<32} {:<18} {:<4} {:<6} {:<8} {:<6}",
        "SSID", "BSSID", "Ch", "Band", "Signal", "Standard");
    println!("{}", "-".repeat(80));
    
    for b in beacons {
        let ssid = b.ssid_string().unwrap_or("[Hidden]".into());
        let net = parse_beacon(b);
        
        println!("{:<32} {:<18} {:<4} {:<6} {:<4}dBm  {:<6}",
            truncate(&ssid, 32),
            b.bssid_string(),
            b.channel,
            b.band,
            b.signal_dbm,
            net.standards.first().unwrap_or(&"?".into()),
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

fn cmd_current() {
    let scanner = get_scanner();
    
    match scanner.current() {
        Ok(Some(b)) => {
            let net = parse_beacon(&b);
            println!("Connected to: {}", net.ssid.unwrap_or_default());
            println!("BSSID:        {}", b.bssid_string());
            println!("Channel:      {} ({} GHz)", b.channel, b.band);
            println!("Signal:       {} dBm", b.signal_dbm);
            println!("Standard:     {:?}", net.standards);
            println!("Security:     {}", net.security);
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

fn cmd_info(bssid: &str) {
    let scanner = get_scanner();
    
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
            println!("Spatial Streams: {}", net.features.spatial_streams);
            println!("Max Rate:     {} Mbps", net.features.max_data_rate);
            println!();
            println!("=== Features ===");
            println!("MU-MIMO:      {}", net.features.mu_mimo);
            println!("OFDMA:        {}", net.features.ofdma);
            println!("MLO:          {}", net.features.mlo);
            println!("Max QAM:      {}", net.features.max_qam);
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

fn cmd_scanners() {
    let scanner = get_scanner();
    println!("Default scanner: {}", scanner.name());
    println!("Requires privilege: {}", scanner.requires_privilege());
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max-3])
    } else {
        s.to_string()
    }
}
