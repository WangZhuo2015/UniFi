// WiFi Tool - Utility Functions

import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';
import type { Network, ProtocolExtensions, PerformanceFeatures } from './types';

/** Merge Tailwind classes */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** Get signal quality from dBm */
export function signalQuality(signal: number): 'excellent' | 'good' | 'fair' | 'weak' | 'poor' {
  if (signal >= -50) return 'excellent';
  if (signal >= -60) return 'good';
  if (signal >= -70) return 'fair';
  if (signal >= -80) return 'weak';
  return 'poor';
}

/** Get signal color class */
export function signalColor(signal: number): string {
  const quality = signalQuality(signal);
  const colors: Record<string, string> = {
    excellent: 'text-green-500',
    good: 'text-lime-500',
    fair: 'text-yellow-500',
    weak: 'text-orange-500',
    poor: 'text-red-500',
  };
  return colors[quality];
}

/** Get signal bar count (0-4) */
export function signalBars(signal: number): number {
  if (signal >= -50) return 4;
  if (signal >= -60) return 3;
  if (signal >= -70) return 2;
  if (signal >= -80) return 1;
  return 0;
}

/** Get security icon */
export function securityIcon(security: string): string {
  if (security === 'open') return '🔓';
  if (security === 'wpa3') return '🔐';
  if (security === 'owe') return '🛡️';
  return '🔒';
}

/** Get security badge color */
export function securityColor(security: string): string {
  if (security === 'open') return 'bg-yellow-100 dark:bg-yellow-900 text-yellow-800 dark:text-yellow-200';
  if (security === 'wpa3') return 'bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200';
  if (security === 'wpa2' || security === 'wpa2-ent') return 'bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200';
  return 'bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-200';
}

/** Check if channel is DFS */
export function isDfsChannel(channel: number): boolean {
  return (channel >= 52 && channel <= 64) || (channel >= 100 && channel <= 144);
}

/** Format channel width */
export function formatChannelWidth(width: number): string {
  return `${width}MHz`;
}

/** Format WiFi standards */
export function formatStandards(standards: string[]): string {
  const display = standards.map(s => {
    switch (s) {
      case 'a': return '802.11a';
      case 'b': return '802.11b';
      case 'g': return '802.11g';
      case 'n': return 'WiFi 4';
      case 'ac': return 'WiFi 5';
      case 'ax': return 'WiFi 6';
      case 'be': return 'WiFi 7';
      default: return s;
    }
  });
  return [...new Set(display)].join(', ');
}

/** Get protocol extensions badge */
export function formatProtocolExtensions(protocols: ProtocolExtensions): string[] {
  const badges: string[] = [];
  if (protocols.rrm) badges.push('k');
  if (protocols.ft) badges.push('r');
  if (protocols.bssTransition) badges.push('v');
  if (protocols.pmf) badges.push('w');
  return badges;
}

/** Check if network supports fast roaming */
export function supportsFastRoaming(protocols: ProtocolExtensions): boolean {
  return protocols.ft;
}

/** Check if network has advanced features */
export function hasAdvancedFeatures(features: PerformanceFeatures): boolean {
  return features.muMimo || features.ofdma;
}

/** Format data rate */
export function formatDataRate(mbps: number): string {
  if (mbps >= 1000) {
    return `${(mbps / 1000).toFixed(1)} Gbps`;
  }
  return `${mbps} Mbps`;
}

/** Get WiFi generation from standards */
export function getWifiGeneration(standards: string[]): string {
  if (standards.includes('be')) return 'WiFi 7';
  if (standards.includes('ax')) return 'WiFi 6';
  if (standards.includes('ac')) return 'WiFi 5';
  if (standards.includes('n')) return 'WiFi 4';
  return 'Legacy';
}

/** Check if network is hidden */
export function isHiddenNetwork(network: Network): boolean {
  return network.isHidden || !network.ssid;
}

/** Get network display name */
export function getNetworkDisplayName(network: Network): string {
  if (network.ssid) return network.ssid;
  return '[隐藏网络]';
}

/** Compare networks for grouping (same SSID check) */
export function isSameNetworkGroup(a: Network, b: Network): boolean {
  if (!a.ssid || !b.ssid) return false;
  return a.ssid === b.ssid && a.security === b.security;
}

/** Get band color */
export function bandColor(band: string): string {
  switch (band) {
    case '2.4': return 'bg-purple-100 dark:bg-purple-900 text-purple-800 dark:text-purple-200';
    case '5': return 'bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200';
    case '6': return 'bg-cyan-100 dark:bg-cyan-900 text-cyan-800 dark:text-cyan-200';
    default: return 'bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-200';
  }
}