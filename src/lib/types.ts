// UniFi - Professional Type Definitions

/** Band type */
export type Band = '2.4' | '5' | '6';

/** Security type */
export type Security = 'open' | 'wep' | 'wpa' | 'wpa2' | 'wpa3' | 'owe' | 'wpa2-ent' | 'wpa3-ent' | 'other';

/** WiFi standard */
export type WiFiStandard = 'a' | 'b' | 'g' | 'n' | 'ac' | 'ax' | 'be';

/** Channel bandwidth */
export type ChannelWidth = 20 | 40 | 80 | 160 | 320;

/** Parsed IE element */
export interface ParsedIE {
  elementId: number;
  elementIdHex: string;
  name: string;
  length: number;
  dataHex: string;
  summary: string;
  vendorName?: string;
  displayFields: ParsedField[];
  parsed: Record<string, string | number | boolean>;
}

export interface ParsedField {
  label: string;
  value: string;
  highlighted: boolean;
}

/** WiFi detection summary */
export interface DetectionSummary {
  hasEhtCapabilities: boolean;
  hasEhtOperation: boolean;
  hasHeCapabilities: boolean;
  hasHeOperation: boolean;
  hasVhtCapabilities: boolean;
  hasVhtOperation: boolean;
  hasHtCapabilities: boolean;
  hasHtOperation: boolean;
  detectedStandard: string;
}

/** Complete IE details */
export interface IEDetails {
  rawHex: string;
  totalLength: number;
  elements: ParsedIE[];
  detectionSummary: DetectionSummary;
}

/** 802.11 Protocol Extensions */
export interface ProtocolExtensions {
  /** 802.11k - Radio Resource Measurement (RRM) */
  rrm: boolean;
  /** 802.11v - Wireless Network Management / BSS Transition */
  bssTransition: boolean;
  /** 802.11r - Fast BSS Transition (Fast Roaming) */
  ft: boolean;
  /** 802.11w - Protected Management Frames */
  pmf: boolean;
  /** 802.11e - QoS / WMM */
  wmm: boolean;
}

/** Performance Features */
export interface PerformanceFeatures {
  /** Multi-User MIMO */
  muMimo: boolean;
  /** OFDMA (WiFi 6+) */
  ofdma: boolean;
  /** BSS Coloring (WiFi 6+) */
  bssColoring: boolean;
  /** Target Wake Time (WiFi 6+) */
  twt: boolean;
  /** Spatial streams count */
  spatialStreams: number;
  /** Maximum supported channel width in MHz */
  maxSupportedWidth: number;
  /** Max data rate in Mbps */
  maxDataRate: number;
  /** TX Beamforming */
  txBeamforming: boolean;
  /** A-MPDU length exponent */
  ampduLength: number;
  /** Multi-Link Operation (WiFi 7 MLO) */
  mlo: boolean;
  /** Max QAM modulation: 256, 1024, or 4096 */
  maxQam: number;
  /** Guard Interval in nanoseconds: 400, 800, 1600, 3200 */
  guardInterval: number;
  /** MCS Index (for current connection) */
  mcsIndex?: number;
}

/** BSS Load Information (802.11k) */
export interface BssLoad {
  /** Channel utilization (0-255, scaled to percentage) */
  channelUtilization: number;
  /** Number of connected stations */
  stationCount: number;
  /** Available admission capacity */
  availableCapacity: number;
}

export interface LinkRates {
  rxRateMbps?: number;
  txRateMbps?: number;
}

export interface LocalAdapterCapabilities {
  driverName: string;
  supportedStandards: WiFiStandard[];
  txSpatialStreams: number;
  rxSpatialStreams: number;
  maxSupportedWidth: number;
}

/** Security Details */
export interface SecurityDetails {
  /** Main security type */
  type: Security;
  /** Authentication method */
  authMethod: 'psk' | 'sae' | 'eap' | 'owe' | 'open' | 'unknown';
  /** Encryption cipher */
  cipher: 'none' | 'tkip' | 'ccmp' | 'gcmp' | 'unknown';
  /** Key management */
  keyMgmt: string[];
  /** Is enterprise network */
  isEnterprise: boolean;
  /** WPA3 transition mode */
  isWpa3Transition: boolean;
  /** PMF required */
  pmfRequired: boolean;
  /** PMF capable */
  pmfCapable: boolean;
}

/** A WiFi network - comprehensive professional data */
export interface Network {
  // === Basic Info ===
  ssid: string | null;
  bssid: string;
  signal: number;          // dBm
  noise: number;           // dBm
  snr: number;             // Signal-to-Noise Ratio
  channel: number;
  frequency: number;       // MHz
  band: Band;
  connected: boolean;

  // === WiFi Standard & Performance ===
  standards: WiFiStandard[];
  wifiGeneration: number;
  channelWidth: ChannelWidth;
  centerChannel?: number;
  secondaryChannel?: number;
  features: PerformanceFeatures;
  minDataRate: number;
  maxDataRate: number;
  apPeakDataRate: number;

  // === Security ===
  security: Security;
  securityDetails: SecurityDetails;

  // === 802.11 Protocol Extensions ===
  protocols: ProtocolExtensions;

  // === BSS Load (802.11k) ===
  bssLoad?: BssLoad;

  // === Network Identification ===
  /** Is this a hidden network (SSID not broadcast) */
  isHidden: boolean;
  /** Network group ID - for identifying same SSID networks */
  networkGroupId?: string;
  /** Vendor from OUI */
  vendor: string;
  /** Country code */
  countryCode?: string;

  // === Additional Info ===
  /** Supported rates in Mbps */
  supportedRates: number[];
  /** WPS enabled */
  wpsEnabled: boolean;
  /** AP mode */
  apMode: number;
  /** Capability flags */
  capabilities: number;

  // === Timing ===
  beaconInterval: number;  // ms
  firstSeen: number;       // timestamp
  lastSeen: number;        // timestamp
  seenAgeSecs: number;
  apUptimeSecs?: number;
  linkRates?: LinkRates;
  localAdapter?: LocalAdapterCapabilities;
  clientPeakDataRate?: number;
  clientSpatialStreams?: number;
}

/** Network Group - same SSID networks grouped together */
export interface NetworkGroup {
  ssid: string;
  networks: Network[];
  totalAps: number;
  bands: Band[];
  bestSignal: number;
  supportsFastRoaming: boolean;  // 802.11r
  supportsBssTransition: boolean; // 802.11v
}

/** Channel score for recommendations */
export interface ChannelScore {
  channel: number;
  score: number;
  networks: string[];
  interference: number;    // interference level
  utilization: number;     // channel utilization
  recommendation: 'best' | 'good' | 'fair' | 'avoid';
}

/** Signal history point */
export interface SignalPoint {
  time: number;
  signal: number;
}

/** Scan statistics */
export interface ScanStats {
  totalNetworks: number;
  hiddenNetworks: number;
  networkGroups: number;
  byBand: Record<Band, number>;
  bySecurity: Record<Security, number>;
  byStandard: Partial<Record<WiFiStandard, number>>;
  scanDurationMs: number;
}

/** Scanner information */
export interface ScannerInfo {
  name: string;
  available: boolean;
  requiresRoot: boolean;
}
