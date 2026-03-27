//! Ping monitoring for roaming tests

use std::process::Command;
use std::time::Instant;
use serde::{Deserialize, Serialize};

/// Single ping result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResult {
    /// Sequence number
    pub seq: u32,
    /// Timestamp (ms from test start)
    pub timestamp_ms: u64,
    /// Round trip time in microseconds
    pub rtt_us: Option<u64>,
    /// Packet lost
    pub lost: bool,
    /// Time to live
    pub ttl: Option<u8>,
    /// Source IP (for multi-interface)
    pub source_ip: Option<String>,
}

/// Ping statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PingStats {
    /// Total packets sent
    pub packets_sent: u32,
    /// Total packets received
    pub packets_received: u32,
    /// Packet loss percentage
    pub packet_loss_percent: f32,
    /// Min RTT in ms
    pub min_rtt_ms: Option<f64>,
    /// Max RTT in ms
    pub max_rtt_ms: Option<f64>,
    /// Average RTT in ms
    pub avg_rtt_ms: Option<f64>,
    /// Standard deviation of RTT
    pub std_dev_ms: Option<f64>,
    /// Jitter (average RTT variation)
    pub jitter_ms: Option<f64>,
}

impl PingStats {
    pub fn update(&mut self, result: &PingResult) {
        self.packets_sent += 1;
        
        if !result.lost {
            self.packets_received += 1;
            
            if let Some(rtt_us) = result.rtt_us {
                let rtt_ms = rtt_us as f64 / 1000.0;
                
                // Update min
                self.min_rtt_ms = Some(self.min_rtt_ms
                    .map_or(rtt_ms, |m| m.min(rtt_ms)));
                
                // Update max
                self.max_rtt_ms = Some(self.max_rtt_ms
                    .map_or(rtt_ms, |m| m.max(rtt_ms)));
                
                // Update average
                let count = self.packets_received as f64;
                self.avg_rtt_ms = Some(self.avg_rtt_ms
                    .map_or(rtt_ms, |avg| avg + (rtt_ms - avg) / count));
            }
        }
        
        // Calculate packet loss
        if self.packets_sent > 0 {
            self.packet_loss_percent = 
                (self.packets_sent - self.packets_received) as f32 / 
                self.packets_sent as f32 * 100.0;
        }
    }
    
    pub fn calculate_final_stats(&mut self, results: &[PingResult]) {
        if results.is_empty() {
            return;
        }
        
        let rtts: Vec<f64> = results.iter()
            .filter(|r| !r.lost && r.rtt_us.is_some())
            .map(|r| r.rtt_us.unwrap() as f64 / 1000.0)
            .collect();
        
        if rtts.is_empty() {
            return;
        }
        
        // Calculate standard deviation
        let avg = self.avg_rtt_ms.unwrap_or(0.0);
        let variance: f64 = rtts.iter()
            .map(|&rtt| (rtt - avg).powi(2))
            .sum::<f64>() / rtts.len() as f64;
        self.std_dev_ms = Some(variance.sqrt());
        
        // Calculate jitter (average of consecutive RTT differences)
        if rtts.len() > 1 {
            let jitter: f64 = rtts.windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .sum::<f64>() / (rtts.len() - 1) as f64;
            self.jitter_ms = Some(jitter);
        }
    }
}

/// Ping probe configuration
#[derive(Debug, Clone)]
pub struct PingConfig {
    /// Target host to ping
    pub target: String,
    /// Interval between pings in milliseconds
    pub interval_ms: u64,
    /// Ping timeout in milliseconds
    pub timeout_ms: u64,
    /// Packet size in bytes
    pub packet_size: u16,
}

impl Default for PingConfig {
    fn default() -> Self {
        Self {
            target: "8.8.8.8".to_string(),
            interval_ms: 100,
            timeout_ms: 1000,
            packet_size: 64,
        }
    }
}

/// Execute a single ping
#[cfg(target_os = "macos")]
pub fn ping_once(config: &PingConfig, seq: u32, start_time: Instant) -> PingResult {
    let timestamp_ms = start_time.elapsed().as_millis() as u64;
    
    let output = Command::new("ping")
        .arg("-c")
        .arg("1")
        .arg("-W")
        .arg((config.timeout_ms as f64 / 1000.0).to_string())
        .arg("-s")
        .arg((config.packet_size - 8).to_string()) // -s is payload size, ICMP header is 8 bytes
        .arg(&config.target)
        .output();
    
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            // Parse ping output
            // Successful: "64 bytes from 8.8.8.8: icmp_seq=0 ttl=117 time=15.123 ms"
            // Failed: empty or "Request timeout"
            
            if output.status.success() {
                // Parse RTT
                let rtt_us = parse_rtt_from_ping(&stdout);
                let ttl = parse_ttl_from_ping(&stdout);
                
                PingResult {
                    seq,
                    timestamp_ms,
                    rtt_us,
                    lost: rtt_us.is_none(),
                    ttl,
                    source_ip: None,
                }
            } else {
                PingResult {
                    seq,
                    timestamp_ms,
                    rtt_us: None,
                    lost: true,
                    ttl: None,
                    source_ip: None,
                }
            }
        }
        Err(_) => PingResult {
            seq,
            timestamp_ms,
            rtt_us: None,
            lost: true,
            ttl: None,
            source_ip: None,
        }
    }
}

/// Execute a single ping on Linux
#[cfg(target_os = "linux")]
pub fn ping_once(config: &PingConfig, seq: u32, start_time: Instant) -> PingResult {
    let timestamp_ms = start_time.elapsed().as_millis() as u64;
    
    let output = Command::new("ping")
        .arg("-c")
        .arg("1")
        .arg("-W")
        .arg(config.timeout_ms.to_string())
        .arg("-s")
        .arg((config.packet_size - 8).to_string())
        .arg(&config.target)
        .output();
    
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            if output.status.success() {
                let rtt_us = parse_rtt_from_ping(&stdout);
                let ttl = parse_ttl_from_ping(&stdout);
                
                PingResult {
                    seq,
                    timestamp_ms,
                    rtt_us,
                    lost: rtt_us.is_none(),
                    ttl,
                    source_ip: None,
                }
            } else {
                PingResult {
                    seq,
                    timestamp_ms,
                    rtt_us: None,
                    lost: true,
                    ttl: None,
                    source_ip: None,
                }
            }
        }
        Err(_) => PingResult {
            seq,
            timestamp_ms,
            rtt_us: None,
            lost: true,
            ttl: None,
            source_ip: None,
        }
    }
}

/// Execute a single ping on Windows
#[cfg(target_os = "windows")]
pub fn ping_once(config: &PingConfig, seq: u32, start_time: Instant) -> PingResult {
    let timestamp_ms = start_time.elapsed().as_millis() as u64;
    
    let output = Command::new("ping")
        .arg("-n")
        .arg("1")
        .arg("-w")
        .arg(config.timeout_ms.to_string())
        .arg("-l")
        .arg(config.packet_size.to_string())
        .arg(&config.target)
        .output();
    
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            if output.status.success() {
                let rtt_us = parse_rtt_from_ping_windows(&stdout);
                let ttl = parse_ttl_from_ping_windows(&stdout);
                
                PingResult {
                    seq,
                    timestamp_ms,
                    rtt_us,
                    lost: rtt_us.is_none(),
                    ttl,
                    source_ip: None,
                }
            } else {
                PingResult {
                    seq,
                    timestamp_ms,
                    rtt_us: None,
                    lost: true,
                    ttl: None,
                    source_ip: None,
                }
            }
        }
        Err(_) => PingResult {
            seq,
            timestamp_ms,
            rtt_us: None,
            lost: true,
            ttl: None,
            source_ip: None,
        }
    }
}

/// Parse RTT from ping output (macOS/Linux format)
fn parse_rtt_from_ping(output: &str) -> Option<u64> {
    // Look for "time=X.XXX ms" or "time=X ms"
    for line in output.lines() {
        if line.contains("time=") {
            if let Some(time_part) = line.split("time=").nth(1) {
                let time_str = time_part.split_whitespace().next()?;
                let time_ms: f64 = time_str.trim_end_matches(" ms").parse().ok()?;
                return Some((time_ms * 1000.0) as u64); // Convert to microseconds
            }
        }
    }
    None
}

/// Parse TTL from ping output
fn parse_ttl_from_ping(output: &str) -> Option<u8> {
    for line in output.lines() {
        if line.contains("ttl=") {
            if let Some(ttl_part) = line.split("ttl=").nth(1) {
                let ttl_str = ttl_part.split_whitespace().next()?;
                return ttl_str.parse().ok();
            }
        }
    }
    None
}

/// Parse RTT from Windows ping output
#[cfg(target_os = "windows")]
fn parse_rtt_from_ping_windows(output: &str) -> Option<u64> {
    // Windows format: "Reply from 8.8.8.8: bytes=64 time=15ms TTL=117"
    for line in output.lines() {
        if line.contains("time=") {
            if let Some(time_part) = line.split("time=").nth(1) {
                let time_str = time_part.split_whitespace().next()?;
                let time_str = time_str.trim_end_matches("ms").trim_end_matches('<');
                let time_ms: f64 = time_str.parse().ok()?;
                return Some((time_ms * 1000.0) as u64);
            }
        }
    }
    None
}

/// Parse TTL from Windows ping output
#[cfg(target_os = "windows")]
fn parse_ttl_from_ping_windows(output: &str) -> Option<u8> {
    for line in output.lines() {
        if line.contains("TTL=") {
            if let Some(ttl_part) = line.split("TTL=").nth(1) {
                let ttl_str = ttl_part.split_whitespace().next()?;
                return ttl_str.parse().ok();
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rtt() {
        let output = "64 bytes from 8.8.8.8: icmp_seq=0 ttl=117 time=15.123 ms";
        let rtt = parse_rtt_from_ping(output);
        assert_eq!(rtt, Some(15123));
        
        let ttl = parse_ttl_from_ping(output);
        assert_eq!(ttl, Some(117));
    }

    #[test]
    fn test_ping_stats() {
        let mut stats = PingStats::default();
        
        stats.update(&PingResult {
            seq: 1,
            timestamp_ms: 0,
            rtt_us: Some(10000),
            lost: false,
            ttl: Some(64),
            source_ip: None,
        });
        
        stats.update(&PingResult {
            seq: 2,
            timestamp_ms: 100,
            rtt_us: Some(15000),
            lost: false,
            ttl: Some(64),
            source_ip: None,
        });
        
        stats.update(&PingResult {
            seq: 3,
            timestamp_ms: 200,
            rtt_us: None,
            lost: true,
            ttl: None,
            source_ip: None,
        });
        
        assert_eq!(stats.packets_sent, 3);
        assert_eq!(stats.packets_received, 2);
        assert!((stats.packet_loss_percent - 33.333).abs() < 0.01);
        assert_eq!(stats.min_rtt_ms, Some(10.0));
        assert_eq!(stats.max_rtt_ms, Some(15.0));
    }
}
