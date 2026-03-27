// Mock Tauri API for testing
export const mockNetworks = [
  {
    ssid: 'Test-Network-1',
    bssid: 'AA:BB:CC:DD:EE:01',
    signal: -45,
    noise: -90,
    snr: 45,
    channel: 6,
    frequency: 2437,
    band: '2.4',
    connected: false,
    standards: ['n', 'ac', 'ax'],
    channelWidth: 80,
    centerChannel: null,
    secondaryChannel: null,
    features: {
      muMimo: true,
      ofdma: true,
      bssColoring: true,
      twt: false,
      spatialStreams: 2,
      maxDataRate: 1200,
      txBeamforming: true,
      ampduLength: 3,
      mlo: false,
      maxQam: 1024,
      guardInterval: 800,
      mcsIndex: null
    },
    security: 'wpa2',
    securityDetails: {
      securityType: 'wpa2',
      authMethod: 'psk',
      cipher: 'ccmp',
      keyMgmt: ['psk'],
      isEnterprise: false,
      isWpa3Transition: false,
      pmfRequired: false,
      pmfCapable: true
    },
    protocols: {
      rrm: true,
      bssTransition: true,
      ft: false,
      pmf: true,
      wmm: true
    },
    bssLoad: {
      channelUtilization: 45,
      stationCount: 5,
      availableCapacity: 200
    },
    isHidden: false,
    networkGroupId: null,
    vendor: 'Test Vendor',
    countryCode: 'CN',
    supportedRates: [1, 2, 5.5, 11, 6, 9, 12, 18, 24, 36, 48, 54],
    wpsEnabled: false,
    apMode: 0,
    capabilities: 0,
    beaconInterval: 100,
    firstSeen: Date.now(),
    lastSeen: Date.now()
  },
  {
    ssid: 'Test-Network-5G',
    bssid: 'AA:BB:CC:DD:EE:02',
    signal: -55,
    noise: -90,
    snr: 35,
    channel: 36,
    frequency: 5180,
    band: '5',
    connected: true,
    standards: ['ac', 'ax', 'be'],
    channelWidth: 160,
    centerChannel: 50,
    secondaryChannel: null,
    features: {
      muMimo: true,
      ofdma: true,
      bssColoring: true,
      twt: true,
      spatialStreams: 4,
      maxDataRate: 4800,
      txBeamforming: true,
      ampduLength: 3,
      mlo: true,
      maxQam: 4096,
      guardInterval: 400,
      mcsIndex: 11
    },
    security: 'wpa3',
    securityDetails: {
      securityType: 'wpa3',
      authMethod: 'sae',
      cipher: 'ccmp',
      keyMgmt: ['sae'],
      isEnterprise: false,
      isWpa3Transition: false,
      pmfRequired: true,
      pmfCapable: true
    },
    protocols: {
      rrm: true,
      bssTransition: true,
      ft: true,
      pmf: true,
      wmm: true
    },
    bssLoad: null,
    isHidden: false,
    networkGroupId: null,
    vendor: 'WiFi Alliance',
    countryCode: 'US',
    supportedRates: [6, 9, 12, 18, 24, 36, 48, 54],
    wpsEnabled: false,
    apMode: 0,
    capabilities: 0,
    beaconInterval: 100,
    firstSeen: Date.now(),
    lastSeen: Date.now()
  },
  {
    ssid: null, // Hidden network
    bssid: 'AA:BB:CC:DD:EE:03',
    signal: -75,
    noise: -90,
    snr: 15,
    channel: 11,
    frequency: 2462,
    band: '2.4',
    connected: false,
    standards: ['n'],
    channelWidth: 40,
    centerChannel: null,
    secondaryChannel: null,
    features: {
      muMimo: false,
      ofdma: false,
      bssColoring: false,
      twt: false,
      spatialStreams: 2,
      maxDataRate: 300,
      txBeamforming: false,
      ampduLength: 2,
      mlo: false,
      maxQam: 256,
      guardInterval: 800,
      mcsIndex: null
    },
    security: 'wpa2',
    securityDetails: {
      securityType: 'wpa2',
      authMethod: 'psk',
      cipher: 'tkip',
      keyMgmt: ['psk'],
      isEnterprise: false,
      isWpa3Transition: false,
      pmfRequired: false,
      pmfCapable: false
    },
    protocols: {
      rrm: false,
      bssTransition: false,
      ft: false,
      pmf: false,
      wmm: true
    },
    bssLoad: null,
    isHidden: true,
    networkGroupId: null,
    vendor: 'Unknown',
    countryCode: null,
    supportedRates: [1, 2, 5.5, 11],
    wpsEnabled: true,
    apMode: 0,
    capabilities: 0,
    beaconInterval: 100,
    firstSeen: Date.now(),
    lastSeen: Date.now()
  }
];

export const mockNetworkGroups = [
  {
    ssid: 'Test-Network-1',
    networks: [mockNetworks[0]],
    totalAps: 1,
    bands: ['2.4'],
    bestSignal: -45,
    supportsFastRoaming: false,
    supportsBssTransition: true
  },
  {
    ssid: 'Test-Network-5G',
    networks: [mockNetworks[1]],
    totalAps: 1,
    bands: ['5'],
    bestSignal: -55,
    supportsFastRoaming: true,
    supportsBssTransition: true
  }
];

export const mockScanStats = {
  totalNetworks: 3,
  hiddenNetworks: 1,
  networkGroups: 2,
  byBand: { '2.4': 2, '5': 1 },
  bySecurity: { wpa2: 2, wpa3: 1 },
  byStandard: { n: 2, ac: 1, ax: 2, be: 1 },
  scanDurationMs: 3500
};

// Setup mock for Tauri invoke
export function setupTauriMock() {
  if (typeof window !== 'undefined') {
    (window as any).__TAURI__ = {
      core: {
        invoke: async (cmd: string, args?: any) => {
          switch (cmd) {
            case 'scan_networks':
              return mockNetworks;
            case 'current_network':
              return mockNetworks[1]; // Return connected network
            case 'get_network_groups':
              return mockNetworkGroups;
            case 'get_scan_stats':
              return mockScanStats;
            case 'get_ie_details':
              return null;
            case 'lookup_vendor':
              return 'Test Vendor';
            default:
              console.log('Unknown command:', cmd);
              return null;
          }
        }
      }
    };
  }
}
