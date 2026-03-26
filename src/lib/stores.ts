// UniFi - State Management
// Svelte stores, simple and direct

import { writable, derived } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Network, NetworkGroup, ScanStats, SignalPoint } from './types';

// ============ Raw Data ============

/** All discovered networks */
export const networks = writable<Network[]>([]);

/** Network groups (same SSID, different APs) */
export const networkGroups = writable<NetworkGroup[]>([]);

/** Scan statistics */
export const scanStats = writable<ScanStats | null>(null);

/** Currently connected network */
export const currentNetwork = writable<Network | null>(null);

/** Selected network for details */
export const selectedBssid = writable<string | null>(null);

/** Network being monitored for signal */
export const monitoredBssid = writable<string | null>(null);

/** Signal history per network */
export const signalHistory = writable<Map<string, SignalPoint[]>>(new Map());

/** Monitoring state */
export const isMonitoring = writable(false);

/** Scanning state */
export const isScanning = writable(false);

/** Error message */
export const error = writable<string | null>(null);

// ============ Derived Data ============

/** Networks grouped by band */
export const byBand = derived(networks, ($networks) => ({
  '2.4': $networks.filter(n => n.band === '2.4'),
  '5': $networks.filter(n => n.band === '5'),
  '6': $networks.filter(n => n.band === '6'),
}));

/** Hidden networks */
export const hiddenNetworks = derived(networks, ($networks) =>
  $networks.filter(n => n.isHidden)
);

/** Best signal network */
export const bestSignal = derived(networks, ($networks) =>
  $networks.reduce<Network | null>(
    (best, n) => n.signal > (best?.signal ?? -200) ? n : best,
    null
  )
);

/** Selected network */
export const selectedNetwork = derived(
  [networks, selectedBssid],
  ([$networks, $bssid]) => $networks.find(n => n.bssid === $bssid) ?? null
);

/** Signal history for monitored network */
export const monitoredHistory = derived(
  [signalHistory, monitoredBssid],
  ([$history, $bssid]) => $bssid ? ($history.get($bssid) ?? []) : []
);

/** Networks with WiFi 6 (ax) support */
export const wifi6Networks = derived(networks, ($networks) =>
  $networks.filter(n => n.standards.includes('ax'))
);

/** Networks with roaming support (802.11r) */
export const roamingNetworks = derived(networks, ($networks) =>
  $networks.filter(n => n.protocols.ft)
);

/** Network count by SSID (for detecting roaming groups) */
export const ssidCount = derived(networks, ($networks) => {
  const counts = new Map<string, number>();
  for (const n of $networks) {
    const ssid = n.ssid ?? '[Hidden]';
    counts.set(ssid, (counts.get(ssid) ?? 0) + 1);
  }
  return counts;
});

// ============ Actions ============

/** Scan for networks */
export async function scan() {
  isScanning.set(true);
  error.set(null);

  try {
    const result = await invoke<Network[]>('scan_networks');
    networks.set(result);

    // Also get network groups
    const groups = await invoke<NetworkGroup[]>('get_network_groups');
    networkGroups.set(groups);

    // Get scan stats
    const stats = await invoke<ScanStats>('get_scan_stats');
    scanStats.set(stats);
  } catch (e) {
    error.set(String(e));
    console.error('Scan error:', e);
  } finally {
    isScanning.set(false);
  }
}

/** Get current connection */
export async function fetchCurrentNetwork() {
  try {
    const result = await invoke<Network | null>('current_network');
    currentNetwork.set(result);
  } catch (e) {
    console.error('Current network error:', e);
  }
}

/** Start background monitoring */
export async function startMonitor() {
  try {
    await invoke('start_monitor');
    isMonitoring.set(true);
  } catch (e) {
    error.set(String(e));
  }
}

/** Stop monitoring */
export async function stopMonitor() {
  try {
    await invoke('stop_monitor');
    isMonitoring.set(false);
  } catch (e) {
    error.set(String(e));
  }
}

/** Look up vendor by BSSID */
export async function lookupVendor(bssid: string): Promise<string | null> {
  try {
    return await invoke<string | null>('lookup_vendor', { bssid });
  } catch {
    return null;
  }
}

/** Update signal history */
function addSignalPoint(bssid: string, signal: number) {
  signalHistory.update(history => {
    const points = history.get(bssid) ?? [];
    const newPoints = [...points, { time: Date.now(), signal }];
    if (newPoints.length > 120) newPoints.shift();
    history.set(bssid, newPoints);
    return new Map(history);
  });
}

// ============ Event Listeners ============

listen<Network[]>('networks-updated', (event) => {
  networks.set(event.payload);
});

listen<{ bssid: string; signal: number }>('signal-update', (event) => {
  addSignalPoint(event.payload.bssid, event.payload.signal);
});