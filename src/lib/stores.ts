// UniFi - State Management
// Svelte stores, simple and direct

import { writable, derived, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Network, NetworkGroup, ScanStats, SignalPoint, ScannerInfo } from './types';

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

/** Available scanners */
export const availableScanners = writable<ScannerInfo[]>([]);

/** Current scanner name */
export const currentScanner = writable<string>('Default');

// ============ Derived Data ============

/** Networks grouped by band */
export const byBand = derived(networks, ($networks) => ({
  '2.4': $networks.filter(n => n.band === '2.4'),
  '5': $networks.filter(n => n.band === '5'),
  '6': $networks.filter(n => n.band === '6'),
}));

function buildNetworkGroups(input: Network[]): NetworkGroup[] {
  const groups = new Map<string, NetworkGroup>();

  for (const net of input) {
    const key = net.ssid ?? '[Hidden]';
    const existing = groups.get(key) ?? {
      ssid: key,
      networks: [],
      totalAps: 0,
      bands: [],
      bestSignal: -100,
      supportsFastRoaming: false,
      supportsBssTransition: false,
    };

    existing.networks.push(net);
    existing.totalAps += 1;
    if (!existing.bands.includes(net.band)) {
      existing.bands.push(net.band);
    }
    existing.bestSignal = Math.max(existing.bestSignal, net.signal);
    existing.supportsFastRoaming ||= net.protocols.ft;
    existing.supportsBssTransition ||= net.protocols.bssTransition;

    groups.set(key, existing);
  }

  return [...groups.values()];
}

function buildScanStats(input: Network[], scanDurationMs: number): ScanStats {
  const byBand: Record<string, number> = {};
  const bySecurity: Record<string, number> = {};
  const byStandard: Record<string, number> = {};
  const visibleSsids = new Set<string>();
  let hiddenNetworks = 0;

  for (const net of input) {
    if (net.isHidden) {
      hiddenNetworks += 1;
    }
    if (net.ssid) {
      visibleSsids.add(net.ssid);
    }

    byBand[net.band] = (byBand[net.band] ?? 0) + 1;
    bySecurity[net.security] = (bySecurity[net.security] ?? 0) + 1;
    for (const standard of net.standards) {
      byStandard[standard] = (byStandard[standard] ?? 0) + 1;
    }
  }

  return {
    totalNetworks: input.length,
    hiddenNetworks,
    networkGroups: visibleSsids.size,
    byBand: byBand as ScanStats['byBand'],
    bySecurity: bySecurity as ScanStats['bySecurity'],
    byStandard,
    scanDurationMs,
  };
}

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
  // Prevent double scan
  if (get(isScanning)) {
    return;
  }

  isScanning.set(true);
  error.set(null);
  const startedAt = performance.now();

  try {
    const scannerName = get(currentScanner);
    const result = scannerName === 'Default'
      ? await invoke<Network[]>('scan_networks')
      : await invoke<Network[]>('scan_networks_with_scanner', { scannerName });

    if (Array.isArray(result)) {
      networks.set(result);
      networkGroups.set(buildNetworkGroups(result));
      scanStats.set(buildScanStats(result, Math.round(performance.now() - startedAt)));
    } else {
      networks.set([]);
      networkGroups.set([]);
    }
  } catch (e) {
    console.error('[Scan] Error:', e);
    error.set(String(e));
    networks.set([]);
    networkGroups.set([]);
  } finally {
    isScanning.set(false);
  }
}

/** Fetch available scanners */
export async function fetchAvailableScanners() {
  try {
    const result = await invoke<ScannerInfo[]>('list_available_scanners');
    availableScanners.set(result || []);
  } catch (e) {
    console.error('[Scanners] Error:', e);
  }
}

/** Get current connection */
export async function fetchCurrentNetwork() {
  try {
    const result = await invoke<Network | null>('current_network');
    currentNetwork.set(result);
  } catch (e) {
    console.error('[CurrentNetwork] Error:', e);
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
  networkGroups.set(buildNetworkGroups(event.payload));
  const previousDuration = get(scanStats)?.scanDurationMs ?? 0;
  scanStats.set(buildScanStats(event.payload, previousDuration));
});

listen<{ bssid: string; signal: number }>('signal-update', (event) => {
  addSignalPoint(event.payload.bssid, event.payload.signal);
});