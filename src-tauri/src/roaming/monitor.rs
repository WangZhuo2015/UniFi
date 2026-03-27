//! Roaming test monitor
//! 
//! Monitors WiFi roaming performance by combining:
//! - Continuous ping to detect latency and packet loss
//! - WiFi connection state monitoring to detect AP changes

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::{PingConfig, PingResult, PingStats, ping_once};

/// Roaming event detected during test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoamingEvent {
    /// Event timestamp (ms from test start)
    pub timestamp_ms: u64,
    /// Previous BSSID
    pub from_bssid: Option<String>,
    /// New BSSID
    pub to_bssid: Option<String>,
    /// Previous SSID
    pub from_ssid: Option<String>,
    /// New SSID
    pub to_ssid: Option<String>,
    /// Previous channel
    pub from_channel: Option<u32>,
    /// New channel
    pub to_channel: Option<u32>,
    /// Roaming duration in ms (time until ping recovers)
    pub roaming_duration_ms: Option<u64>,
    /// Packets lost during roaming
    pub packets_lost: u32,
    /// Max latency during roaming in ms
    pub max_latency_ms: Option<f64>,
}

/// WiFi connection state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WifiState {
    pub ssid: Option<String>,
    pub bssid: Option<String>,
    pub channel: Option<u32>,
    pub signal: Option<i32>,
    pub band: Option<String>,
}

/// Latency spike event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencySpike {
    /// Timestamp (ms from test start)
    pub timestamp_ms: u64,
    /// Latency in ms
    pub latency_ms: f64,
    /// Previous average latency
    pub baseline_ms: f64,
    /// Spike ratio (latency / baseline)
    pub spike_ratio: f64,
}

/// Roaming test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoamingTestResult {
    /// Test duration in seconds
    pub duration_secs: u64,
    /// Total ping statistics
    pub ping_stats: PingStats,
    /// All ping results
    pub ping_results: Vec<PingResult>,
    /// Detected roaming events
    pub roaming_events: Vec<RoamingEvent>,
    /// Detected latency spikes (latency > 2x baseline)
    pub latency_spikes: Vec<LatencySpike>,
    /// Number of roaming events
    pub roaming_count: u32,
    /// Average roaming duration in ms
    pub avg_roaming_duration_ms: Option<f64>,
    /// Total packets lost during roaming
    pub total_roaming_packet_loss: u32,
    /// Connection state history
    pub connection_history: Vec<ConnectionSnapshot>,
}

/// Connection state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSnapshot {
    pub timestamp_ms: u64,
    pub state: WifiState,
}

/// Roaming test configuration
#[derive(Debug, Clone)]
pub struct RoamingTestConfig {
    /// Ping configuration
    pub ping: PingConfig,
    /// Test duration in seconds
    pub duration_secs: u64,
    /// Latency spike threshold (ratio over baseline)
    pub latency_spike_threshold: f64,
    /// Window size for baseline calculation (number of pings)
    pub baseline_window: usize,
}

impl Default for RoamingTestConfig {
    fn default() -> Self {
        Self {
            ping: PingConfig::default(),
            duration_secs: 60,
            latency_spike_threshold: 2.0,
            baseline_window: 20,
        }
    }
}

/// Roaming test monitor
pub struct RoamingMonitor {
    config: RoamingTestConfig,
    running: Arc<AtomicBool>,
    results: Arc<Mutex<RoamingTestResult>>,
    current_wifi: Arc<Mutex<WifiState>>,
}

impl RoamingMonitor {
    pub fn new(config: RoamingTestConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            results: Arc::new(Mutex::new(RoamingTestResult::default())),
            current_wifi: Arc::new(Mutex::new(WifiState::default())),
        }
    }
    
    /// Start the roaming test
    pub fn start(&self) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Err("Test already running".to_string());
        }
        
        self.running.store(true, Ordering::SeqCst);
        
        // Reset results
        *self.results.lock().unwrap() = RoamingTestResult::default();
        
        let running = self.running.clone();
        let results = self.results.clone();
        let current_wifi = self.current_wifi.clone();
        let config = self.config.clone();
        
        thread::spawn(move || {
            run_test(&config, running, results, current_wifi);
        });
        
        Ok(())
    }
    
    /// Stop the roaming test
    pub fn stop(&self) -> RoamingTestResult {
        self.running.store(false, Ordering::SeqCst);
        
        // Wait a bit for the thread to finish
        thread::sleep(Duration::from_millis(200));
        
        let mut results = self.results.lock().unwrap().clone();
        
        // Calculate final statistics
        results.ping_stats.calculate_final_stats(&results.ping_results);
        
        // Calculate average roaming duration
        if !results.roaming_events.is_empty() {
            let total: f64 = results.roaming_events.iter()
                .filter_map(|e| e.roaming_duration_ms.map(|d| d as f64))
                .sum();
            let count = results.roaming_events.iter()
                .filter(|e| e.roaming_duration_ms.is_some())
                .count() as f64;
            
            if count > 0.0 {
                results.avg_roaming_duration_ms = Some(total / count);
            }
        }
        
        results
    }
    
    /// Check if test is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
    
    /// Get current progress
    pub fn get_progress(&self) -> (u32, u32) {
        let results = self.results.lock().unwrap();
        (results.ping_results.len() as u32, 
         (self.config.duration_secs * 1000 / self.config.ping.interval_ms) as u32)
    }
    
    /// Get current results (intermediate)
    pub fn get_results(&self) -> RoamingTestResult {
        self.results.lock().unwrap().clone()
    }
    
    /// Update current WiFi state (called externally)
    pub fn update_wifi_state(&self, state: WifiState) {
        *self.current_wifi.lock().unwrap() = state;
    }
}

impl Default for RoamingMonitor {
    fn default() -> Self {
        Self::new(RoamingTestConfig::default())
    }
}

/// Run the roaming test
fn run_test(
    config: &RoamingTestConfig,
    running: Arc<AtomicBool>,
    results: Arc<Mutex<RoamingTestResult>>,
    current_wifi: Arc<Mutex<WifiState>>,
) {
    let start_time = Instant::now();
    let test_duration = Duration::from_secs(config.duration_secs);
    
    let mut seq: u32 = 0;
    let mut prev_wifi = current_wifi.lock().unwrap().clone();
    let mut latency_window: VecDeque<f64> = VecDeque::with_capacity(config.baseline_window);
    
    // Track roaming state
    let mut in_roaming = false;
    let mut roaming_start_ms: Option<u64> = None;
    let mut roaming_packets_lost: u32 = 0;
    let mut roaming_max_latency: Option<f64> = None;
    
    while running.load(Ordering::SeqCst) && start_time.elapsed() < test_duration {
        // Send ping
        let ping_result = ping_once(&config.ping, seq, start_time);
        let timestamp_ms = ping_result.timestamp_ms;
        
        // Get current WiFi state
        let wifi_state = current_wifi.lock().unwrap().clone();
        
        // Record connection snapshot periodically (every 10 pings)
        if seq % 10 == 0 {
            results.lock().unwrap().connection_history.push(ConnectionSnapshot {
                timestamp_ms,
                state: wifi_state.clone(),
            });
        }
        
        // Check for roaming (BSSID changed)
        let bssid_changed = wifi_state.bssid.is_some() && 
                          prev_wifi.bssid.is_some() && 
                          wifi_state.bssid != prev_wifi.bssid;
        
        let ssid_changed = wifi_state.ssid != prev_wifi.ssid;
        
        if bssid_changed || ssid_changed {
            // End previous roaming if any
            if in_roaming {
                // Calculate roaming duration based on recovery
                let roaming_duration = if ping_result.lost {
                    None // Still losing packets
                } else {
                    roaming_start_ms.map(|start| timestamp_ms - start)
                };
                
                // Record the roaming event
                let event = RoamingEvent {
                    timestamp_ms: roaming_start_ms.unwrap_or(timestamp_ms),
                    from_bssid: prev_wifi.bssid.clone(),
                    to_bssid: wifi_state.bssid.clone(),
                    from_ssid: prev_wifi.ssid.clone(),
                    to_ssid: wifi_state.ssid.clone(),
                    from_channel: prev_wifi.channel,
                    to_channel: wifi_state.channel,
                    roaming_duration_ms: roaming_duration,
                    packets_lost: roaming_packets_lost,
                    max_latency_ms: roaming_max_latency,
                };
                
                results.lock().unwrap().roaming_events.push(event);
                results.lock().unwrap().roaming_count += 1;
                results.lock().unwrap().total_roaming_packet_loss += roaming_packets_lost;
            }
            
            // Start new roaming
            in_roaming = true;
            roaming_start_ms = Some(timestamp_ms);
            roaming_packets_lost = 0;
            roaming_max_latency = None;
            
            prev_wifi = wifi_state.clone();
        }
        
        // Track latency during roaming
        if in_roaming {
            if ping_result.lost {
                roaming_packets_lost += 1;
            } else if let Some(rtt_us) = ping_result.rtt_us {
                let latency_ms = rtt_us as f64 / 1000.0;
                roaming_max_latency = Some(roaming_max_latency
                    .map_or(latency_ms, |m| m.max(latency_ms)));
                
                // Check if latency is back to normal (within 1.5x baseline)
                if latency_window.len() >= 5 {
                    let baseline: f64 = latency_window.iter().sum::<f64>() / latency_window.len() as f64;
                    if latency_ms <= baseline * 1.5 {
                        // Roaming complete
                        let roaming_duration = roaming_start_ms.map(|start| timestamp_ms - start);
                        
                        let event = RoamingEvent {
                            timestamp_ms: roaming_start_ms.unwrap_or(timestamp_ms),
                            from_bssid: prev_wifi.bssid.clone(),
                            to_bssid: wifi_state.bssid.clone(),
                            from_ssid: prev_wifi.ssid.clone(),
                            to_ssid: wifi_state.ssid.clone(),
                            from_channel: prev_wifi.channel,
                            to_channel: wifi_state.channel,
                            roaming_duration_ms: roaming_duration,
                            packets_lost: roaming_packets_lost,
                            max_latency_ms: roaming_max_latency,
                        };
                        
                        results.lock().unwrap().roaming_events.push(event);
                        results.lock().unwrap().roaming_count += 1;
                        results.lock().unwrap().total_roaming_packet_loss += roaming_packets_lost;
                        
                        in_roaming = false;
                        roaming_start_ms = None;
                        roaming_packets_lost = 0;
                        roaming_max_latency = None;
                    }
                }
            }
        }
        
        // Track latency for baseline
        if let Some(rtt_us) = ping_result.rtt_us {
            let latency_ms = rtt_us as f64 / 1000.0;
            
            // Add to window
            latency_window.push_back(latency_ms);
            if latency_window.len() > config.baseline_window {
                latency_window.pop_front();
            }
            
            // Check for latency spike
            if latency_window.len() >= 5 && !in_roaming {
                let baseline: f64 = latency_window.iter().sum::<f64>() / latency_window.len() as f64;
                let spike_ratio = latency_ms / baseline;
                
                if spike_ratio > config.latency_spike_threshold {
                    results.lock().unwrap().latency_spikes.push(LatencySpike {
                        timestamp_ms,
                        latency_ms,
                        baseline_ms: baseline,
                        spike_ratio,
                    });
                }
            }
        }
        
        // Record ping result
        {
            let mut results = results.lock().unwrap();
            results.ping_stats.update(&ping_result);
            results.ping_results.push(ping_result);
        }
        
        seq += 1;
        
        // Wait for next ping interval
        thread::sleep(Duration::from_millis(config.ping.interval_ms));
    }
    
    // Record final duration
    results.lock().unwrap().duration_secs = start_time.elapsed().as_secs();
}

impl Default for RoamingTestResult {
    fn default() -> Self {
        Self {
            duration_secs: 0,
            ping_stats: PingStats::default(),
            ping_results: Vec::new(),
            roaming_events: Vec::new(),
            latency_spikes: Vec::new(),
            roaming_count: 0,
            avg_roaming_duration_ms: None,
            total_roaming_packet_loss: 0,
            connection_history: Vec::new(),
        }
    }
}
