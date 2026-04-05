<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import {
    networks,
    scan,
    isScanning,
    error,
    byBand,
    selectedBssid,
    selectedNetwork,
    currentNetwork,
    fetchCurrentNetwork,
    isMonitoring,
    startMonitor,
    stopMonitor,
    networkGroups,
    scanStats,
    availableScanners,
    currentScanner,
    fetchAvailableScanners
  } from '$lib/stores';
  import { locale, t, translateSignalQuality } from '$lib/i18n';
  import { cn, signalColor, signalQuality } from '$lib/utils';
  import IEDetailsPanel from '$lib/components/IEDetailsPanel.svelte';
  import ChannelView from '$lib/components/ChannelView.svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import RoamingTest from '$lib/components/RoamingTest.svelte';
  import VendorLogo from '$lib/components/VendorLogo.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import Dropdown from '$lib/components/ui/dropdown.svelte';
  import type { Network } from '$lib/types';

  let filterText = $state('');
  let activeBand = $state<'all' | '2.4' | '5' | '6'>('all');
  let activeTab = $state<'networks' | 'channels' | 'groups' | 'roaming'>('networks');
  let showIEDetails = $state(false);
  let showDetailsPane = $state(true);
  let sortKey = $state<
    'signal' | 'ssid' | 'bssid' | 'beacon' | 'minRate' | 'maxRate' | 'band' | 'channel' | 'width' | 'standard' | 'country' | 'generation' | 'uptime' | 'security' | 'seen'
  >('signal');
  let sortDirection = $state<'asc' | 'desc'>('desc');
  let scannerDropdownOpen = $state(false);
  let exportDropdownOpen = $state(false);

  // Make dropdowns mutually exclusive
  $effect(() => {
    if (scannerDropdownOpen && exportDropdownOpen) {
      exportDropdownOpen = false;
    }
  });

  // Check if all networks are hidden - indicates missing location permission
  const allNetworksHidden = $derived.by(() => {
    if ($networks.length === 0) return false;
    return $networks.every(n =>
      (n.ssid === null || n.ssid === undefined) &&
      n.bssid === '00:00:00:00:00:00'
    );
  });

  const filtered = $derived.by(() => {
    let list = $networks;

    if (activeBand !== 'all') {
      list = $byBand[activeBand];
    }

    if (filterText) {
      const search = filterText.toLowerCase();
      list = list.filter((n) =>
        (n.ssid?.toLowerCase().includes(search) ?? false) ||
        n.bssid.toLowerCase().includes(search) ||
        n.vendor.toLowerCase().includes(search)
      );
    }

    return [...list]
      .map((network, index) => ({ network, index }))
      .sort((left, right) => {
        const comparison = compareNetworks(left.network, right.network, sortKey);
        if (comparison !== 0) {
          return sortDirection === 'asc' ? comparison : -comparison;
        }

        return left.index - right.index;
      })
      .map(({ network }) => network);
  });

  const groupSummary = $derived.by(() => {
    const groups = $networkGroups;
    const totalAps = groups.reduce((sum, group) => sum + group.totalAps, 0);
    const roamingReady = groups.filter((group) => group.supportsFastRoaming || group.supportsBssTransition).length;
    const strongest = groups.reduce((best, group) => Math.max(best, group.bestSignal), -100);

    return {
      totalGroups: groups.length,
      totalAps,
      roamingReady,
      strongest: groups.length === 0 ? undefined : strongest
    };
  });

  function exportJSON() {
    const data = JSON.stringify($networks, null, 2);
    downloadFile(data, 'unifi-scan.json', 'application/json');
    showExportMenu = false;
  }

  function exportCSV() {
    const headers = ['SSID', 'BSSID', 'Channel', 'Band', 'Signal (dBm)', 'Standard', 'Security', 'Vendor'];
    const rows = $networks.map((n) => [
      n.ssid ?? t($locale, 'hiddenNetwork'),
      n.bssid,
      n.channel,
      n.band,
      n.signal,
      n.standards.join('/'),
      n.security,
      n.vendor
    ]);

    const csv = [headers, ...rows].map((row) => row.map((value) => `"${String(value).replaceAll('"', '""')}"`).join(',')).join('\n');
    downloadFile(csv, 'unifi-scan.csv', 'text/csv');
    showExportMenu = false;
  }

  function downloadFile(content: string, filename: string, type: string) {
    const blob = new Blob([content], { type });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  function toggleMonitoring() {
    if ($isMonitoring) {
      stopMonitor();
    } else {
      startMonitor();
    }
  }

  function detailValue(value: string | number | null | undefined, fallback = '-') {
    return value === null || value === undefined || value === '' ? fallback : value;
  }

  function selectNetwork(bssid: string) {
    selectedBssid.set(bssid);
  }

  function formatRate(value: number) {
    return `${value.toFixed(1)} Mbps`;
  }

  function formatOptionalRate(value: number | undefined) {
    return value === undefined ? t($locale, 'notAvailable') : formatRate(value);
  }

  function formatDuration(totalSeconds: number | undefined) {
    if (totalSeconds === undefined) {
      return t($locale, 'notAvailable');
    }

    const days = Math.floor(totalSeconds / 86400);
    const hours = Math.floor((totalSeconds % 86400) / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;

    if (days > 0) {
      return `${days}d ${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
    }

    return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
  }

  function formatSeen(ageSeconds: number) {
    return ageSeconds === 0 ? t($locale, 'nowLabel') : `${ageSeconds}s`;
  }

  function bandLabel(band: Network['band']) {
    return `${band} GHz`;
  }

  function groupProtocols(group: (typeof $networkGroups)[number]) {
    return [
      group.supportsFastRoaming ? '802.11r' : null,
      group.supportsBssTransition ? '802.11v' : null
    ].filter(Boolean) as string[];
  }

  function toggleSort(nextKey: typeof sortKey) {
    if (sortKey === nextKey) {
      sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
      return;
    }

    sortKey = nextKey;
    sortDirection = nextKey === 'ssid' || nextKey === 'bssid' || nextKey === 'security' || nextKey === 'standard' || nextKey === 'country'
      ? 'asc'
      : 'desc';
  }

  function sortIndicator(key: typeof sortKey) {
    if (sortKey !== key) {
      return '↕';
    }

    return sortDirection === 'asc' ? '↑' : '↓';
  }

  function compareText(left: string, right: string) {
    return left.localeCompare(right, undefined, { sensitivity: 'base', numeric: true });
  }

  function compareNumbers(left: number | undefined, right: number | undefined) {
    return (left ?? Number.NEGATIVE_INFINITY) - (right ?? Number.NEGATIVE_INFINITY);
  }

  function compareNetworks(left: Network, right: Network, key: typeof sortKey) {
    switch (key) {
      case 'ssid':
        return compareText(left.ssid ?? '', right.ssid ?? '');
      case 'bssid':
        return compareText(left.bssid, right.bssid);
      case 'beacon':
        return compareNumbers(left.beaconInterval, right.beaconInterval);
      case 'minRate':
        return compareNumbers(left.minDataRate, right.minDataRate);
      case 'maxRate':
        return compareNumbers(left.maxDataRate, right.maxDataRate);
      case 'band':
        return compareNumbers(Number(left.band), Number(right.band));
      case 'channel':
        return compareNumbers(left.channel, right.channel);
      case 'width':
        return compareNumbers(left.channelWidth, right.channelWidth);
      case 'standard':
        return compareText(displayStandard(left), displayStandard(right));
      case 'country':
        return compareText(left.countryCode ?? '', right.countryCode ?? '');
      case 'generation':
        return compareNumbers(left.wifiGeneration, right.wifiGeneration);
      case 'uptime':
        return compareNumbers(left.apUptimeSecs, right.apUptimeSecs);
      case 'security':
        return compareText(left.security, right.security);
      case 'seen':
        return compareNumbers(left.seenAgeSecs, right.seenAgeSecs);
      case 'signal':
      default:
        return compareNumbers(left.signal, right.signal);
    }
  }

  function displayStandard(network: Network | null) {
    if (!network || network.standards.length === 0) {
      return '-';
    }

    return network.standards.join('/');
  }

  onMount(() => {
    const timer = window.setTimeout(() => {
      void fetchAvailableScanners();
      void fetchCurrentNetwork();
      void scan();
    }, 150);

    return () => window.clearTimeout(timer);
  });

  function selectScanner(name: string) {
    currentScanner.set(name);
    void scan();
  }

  async function requestLocationPermission() {
    try {
      await invoke('request_location_permission');
    } catch (e) {
      console.error('Failed to request permission:', e);
    }
    await openLocationSettings();
    setTimeout(() => void scan(), 2000);
  }

  async function openLocationSettings() {
    try {
      await invoke('open_url', { url: 'x-apple.systempreferences:com.apple.preference.security?Privacy_LocationServices' });
    } catch {
      try {
        await invoke('open_url', { url: 'x-apple.systempreferences:com.apple.preference.security' });
      } catch { /* ignore */ }
    }
  }
</script>

<div class="flex h-screen flex-col overflow-hidden rounded-lg border border-gray-200/50 bg-white text-gray-900 shadow-2xl dark:border-gray-700/50 dark:bg-gray-900 dark:text-gray-100">
  <TitleBar />

  <header class="shrink-0 border-b border-gray-200/50 bg-gray-50/80 px-4 py-2.5 backdrop-blur-xl dark:border-gray-700/50 dark:bg-gray-800/80">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div class="min-w-0 flex-1">
        <div class="inline-flex max-w-full flex-wrap rounded-lg bg-gray-100 p-0.5 text-xs font-medium dark:bg-gray-700">
          <button data-testid="tab-networks" class="px-3 py-1.5 rounded-md transition-all duration-200 {activeTab === 'networks' ? 'bg-white text-gray-900 shadow-sm dark:bg-gray-600 dark:text-white' : 'text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-200'}" onclick={() => activeTab = 'networks'}>{t($locale, 'networksTab')}</button>
          <button data-testid="tab-channels" class="px-3 py-1.5 rounded-md transition-all duration-200 {activeTab === 'channels' ? 'bg-white text-gray-900 shadow-sm dark:bg-gray-600 dark:text-white' : 'text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-200'}" onclick={() => activeTab = 'channels'}>{t($locale, 'channelsTab')}</button>
          <button data-testid="tab-groups" class="px-3 py-1.5 rounded-md transition-all duration-200 {activeTab === 'groups' ? 'bg-white text-gray-900 shadow-sm dark:bg-gray-600 dark:text-white' : 'text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-200'}" onclick={() => activeTab = 'groups'}>{t($locale, 'groupsTab')}</button>
          <button data-testid="tab-roaming" class="px-3 py-1.5 rounded-md transition-all duration-200 {activeTab === 'roaming' ? 'bg-white text-gray-900 shadow-sm dark:bg-gray-600 dark:text-white' : 'text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-200'}" onclick={() => activeTab = 'roaming'}>{t($locale, 'roamingTab')}</button>
        </div>
      </div>

      <div class="flex shrink-0 flex-wrap items-center justify-end gap-2">
        <Dropdown label="{$currentScanner} ▾" bind:open={scannerDropdownOpen}>
          <button class="mx-1 block w-full rounded-md px-3 py-2 text-left text-sm transition-colors hover:bg-gray-100 dark:hover:bg-gray-700/50 {$currentScanner === 'Default' ? 'font-semibold text-blue-600 dark:text-blue-400' : ''}" onclick={() => { selectScanner('Default'); scannerDropdownOpen = false; }}>{t($locale, 'defaultScanner')}</button>
          {#each $availableScanners as scanner (scanner.name)}
            {#if scanner.available}
              <button class="mx-1 block w-full rounded-md px-3 py-2 text-left text-sm transition-colors hover:bg-gray-100 dark:hover:bg-gray-700/50 {$currentScanner === scanner.name ? 'font-semibold text-blue-600 dark:text-blue-400' : ''}" onclick={() => { selectScanner(scanner.name); scannerDropdownOpen = false; }}>{scanner.name}{#if scanner.requiresRoot}<span class="ml-1 text-xs text-amber-500">(root)</span>{/if}</button>
            {/if}
          {/each}
        </Dropdown>

        <Dropdown label="{t($locale, 'export')} ▾" bind:open={exportDropdownOpen} disabled={$networks.length === 0}>
          <button class="mx-1 block w-full rounded-md px-3 py-2 text-left text-sm transition-colors hover:bg-gray-100 dark:hover:bg-gray-700/50" onclick={() => { exportJSON(); exportDropdownOpen = false; }}>{t($locale, 'exportJson')}</button>
          <button class="mx-1 block w-full rounded-md px-3 py-2 text-left text-sm transition-colors hover:bg-gray-100 dark:hover:bg-gray-700/50" onclick={() => { exportCSV(); exportDropdownOpen = false; }}>{t($locale, 'exportCsv')}</button>
        </Dropdown>

        <Button variant="ghost" size="sm" onclick={toggleMonitoring}>
          {$isMonitoring ? t($locale, 'monitoring') : t($locale, 'startMonitoring')}
        </Button>

        <Button size="sm" disabled={$isScanning} class="native-button" onclick={() => scan()}>
          {$isScanning ? t($locale, 'scanning') : t($locale, 'scan')}
        </Button>
      </div>
    </div>

    {#if $currentNetwork}
      <div class="mt-2 flex flex-wrap items-center gap-3 rounded-lg border border-green-200/50 bg-green-50/80 px-3 py-2 text-sm text-green-800 dark:border-green-800/50 dark:bg-green-900/20 dark:text-green-200">
        <span class="inline-flex h-2 w-2 rounded-full bg-green-500"></span>
        <span class="font-medium">{t($locale, 'connectedTo')} {$currentNetwork.ssid ?? t($locale, 'hiddenNetwork')}</span>
        <span class="text-xs text-green-700 dark:text-green-300">CH {$currentNetwork.channel} · {$currentNetwork.band} GHz · {$currentNetwork.signal} dBm</span>
        {#if $currentNetwork.linkRates?.rxRateMbps !== undefined || $currentNetwork.linkRates?.txRateMbps !== undefined}
          <span class="text-xs text-green-700 dark:text-green-300">
            Rx {formatOptionalRate($currentNetwork.linkRates?.rxRateMbps)} · Tx {formatOptionalRate($currentNetwork.linkRates?.txRateMbps)}
          </span>
        {/if}
      </div>
    {/if}
  </header>

  {#if $error}
    <div class="border-b border-red-200/50 bg-red-50/80 px-4 py-2 text-sm text-red-700 dark:border-red-800/50 dark:bg-red-900/20 dark:text-red-300">{$error}</div>
  {/if}

  {#if allNetworksHidden}
    <div class="border-b border-amber-200/50 bg-amber-50/80 px-4 py-3 text-sm text-amber-800 dark:border-amber-800/50 dark:bg-amber-900/20 dark:text-amber-200">
      <div class="flex items-center gap-3">
        <svg class="h-5 w-5 shrink-0 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <div class="flex-1">
          <p class="font-medium">{t($locale, 'locationPermissionRequired')}</p>
          <p class="text-xs text-amber-700 dark:text-amber-300">{t($locale, 'locationPermissionReason')}</p>
        </div>
        <button type="button" class="shrink-0 rounded-md bg-amber-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-amber-700" onclick={requestLocationPermission}>{t($locale, 'openLocationSettings')}</button>
      </div>
    </div>
  {/if}

  <div class="min-h-0 flex flex-1 flex-col overflow-hidden xl:flex-row">
    {#if activeTab === 'networks'}
      <div class={cn(
        'flex min-h-0 min-w-0 flex-1 flex-col bg-gray-50/50 dark:bg-gray-800/50',
        showDetailsPane && $selectedNetwork
          ? 'xl:border-r xl:border-gray-200/50 xl:dark:border-gray-700/50'
          : 'w-full'
      )}>
        <div class="shrink-0 border-b border-gray-200/50 bg-white/50 p-3 backdrop-blur-sm dark:border-gray-700/50 dark:bg-gray-800/50">
          <div class="flex flex-wrap gap-2">
            <input type="search" bind:value={filterText} placeholder={t($locale, 'searchNetworks')} class="min-w-[180px] flex-1 rounded-lg border border-gray-200 bg-white px-3 py-2 text-xs text-gray-900 placeholder-gray-400 focus:border-blue-500/50 focus:ring-2 focus:ring-blue-500/30 dark:border-gray-600/50 dark:bg-gray-700/50 dark:text-gray-100" />
            <div class="inline-flex shrink-0 flex-wrap rounded-lg bg-gray-100 p-0.5 dark:bg-gray-700">
              {#each [
                { value: 'all', label: t($locale, 'allBands') },
                { value: '2.4', label: t($locale, 'band24') },
                { value: '5', label: t($locale, 'band5') },
                { value: '6', label: t($locale, 'band6') }
              ] as item}
                <button class="px-2.5 py-1.5 text-xs font-medium rounded-md transition-all duration-200 {activeBand === item.value ? 'bg-white text-gray-900 shadow-sm dark:bg-gray-600 dark:text-white' : 'text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-200'}" onclick={() => activeBand = item.value as typeof activeBand}>{item.label}</button>
              {/each}
            </div>
          </div>
          <div class="mt-2 flex flex-wrap items-center justify-between gap-2 text-xs text-gray-500 dark:text-gray-400">
            <div class="flex flex-wrap items-center gap-3">
              <span class="font-medium">{t($locale, 'networksCount', { count: filtered.length })}</span>
              <span>{t($locale, 'horizontalScrollHint')}</span>
            </div>
            <div class="flex flex-wrap items-center gap-3">
              {#if $scanStats}
                <span>{t($locale, 'scanDuration')}: {$scanStats.scanDurationMs} ms</span>
              {/if}
              {#if $selectedNetwork}
                <button
                  type="button"
                  class="rounded-md border border-gray-200/80 bg-white px-2.5 py-1 font-medium text-gray-600 transition-colors hover:border-gray-300 hover:text-gray-900 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-300 dark:hover:text-white"
                  onclick={() => showDetailsPane = !showDetailsPane}
                >
                  {showDetailsPane ? t($locale, 'hideDetailsPane') : t($locale, 'showDetailsPane')}
                </button>
              {/if}
            </div>
          </div>
        </div>

        <div class="min-h-0 flex-1 overflow-hidden">
          {#if $isScanning && $networks.length === 0}
            <div class="py-12 text-center text-sm text-gray-400 dark:text-gray-500">{t($locale, 'scanningInProgress')}</div>
          {:else if filtered.length === 0}
            <div class="py-12 text-center text-sm text-gray-400 dark:text-gray-500">{t($locale, 'noNetworksFound')}</div>
          {:else}
            <div class="h-full overflow-y-auto overflow-x-auto">
              <div class="min-w-[92rem] pb-3">
                <table class="w-full table-auto border-separate border-spacing-0 text-sm">
                  <thead class="sticky top-0 z-10 bg-gray-100/95 backdrop-blur dark:bg-gray-800/95">
                  <tr class="text-xs font-semibold uppercase tracking-wide text-gray-600 dark:text-gray-300">
                    <th class="border-b border-gray-200/60 px-3 py-2 text-left dark:border-gray-700/60">
                      <button type="button" class="inline-flex items-center gap-1" onclick={() => toggleSort('bssid')}>{t($locale, 'bssidColumn')}<span class="text-[10px] opacity-70">{sortIndicator('bssid')}</span></button>
                    </th>
                    <th class="border-b border-gray-200/60 px-3 py-2 text-left dark:border-gray-700/60 min-w-[12rem]">
                      <button type="button" class="inline-flex items-center gap-1" onclick={() => toggleSort('ssid')}>{t($locale, 'networkNameColumn')}<span class="text-[10px] opacity-70">{sortIndicator('ssid')}</span></button>
                    </th>
                    <th class="border-b border-gray-200/60 px-3 py-2 text-left dark:border-gray-700/60 min-w-[8rem]">
                      <button type="button" class="inline-flex items-center gap-1" onclick={() => toggleSort('signal')}>{t($locale, 'rssiColumn')}<span class="text-[10px] opacity-70">{sortIndicator('signal')}</span></button>
                    </th>
                    <th class="border-b border-gray-200/60 px-3 py-2 text-left dark:border-gray-700/60">
                      <button type="button" class="inline-flex items-center gap-1" onclick={() => toggleSort('beacon')}>{t($locale, 'beaconColumn')}<span class="text-[10px] opacity-70">{sortIndicator('beacon')}</span></button>
                    </th>
                    <th class="border-b border-gray-200/60 px-3 py-2 text-left dark:border-gray-700/60">
                      <button type="button" class="inline-flex items-center gap-1" onclick={() => toggleSort('minRate')}>{t($locale, 'minRateColumn')}<span class="text-[10px] opacity-70">{sortIndicator('minRate')}</span></button>
                    </th>
                    <th class="border-b border-gray-200/60 px-3 py-2 text-left dark:border-gray-700/60">
                      <button type="button" class="inline-flex items-center gap-1" onclick={() => toggleSort('maxRate')}>{t($locale, 'apCurrentPeakRateColumn')}<span class="text-[10px] opacity-70">{sortIndicator('maxRate')}</span></button>
                    </th>
                    <th class="border-b border-gray-200/60 px-3 py-2 text-left dark:border-gray-700/60">
                      <button type="button" class="inline-flex items-center gap-1" onclick={() => toggleSort('band')}>{t($locale, 'band')}<span class="text-[10px] opacity-70">{sortIndicator('band')}</span></button>
                    </th>
                    <th class="border-b border-gray-200/60 px-3 py-2 text-left dark:border-gray-700/60">
                      <button type="button" class="inline-flex items-center gap-1" onclick={() => toggleSort('channel')}>{t($locale, 'channelColumn')}<span class="text-[10px] opacity-70">{sortIndicator('channel')}</span></button>
                    </th>
                    <th class="border-b border-gray-200/60 px-3 py-2 text-left dark:border-gray-700/60">
                      <button type="button" class="inline-flex items-center gap-1" onclick={() => toggleSort('width')}>{t($locale, 'widthColumn')}<span class="text-[10px] opacity-70">{sortIndicator('width')}</span></button>
                    </th>
                    <th class="border-b border-gray-200/60 px-3 py-2 text-left dark:border-gray-700/60 min-w-[7rem]">
                      <button type="button" class="inline-flex items-center gap-1" onclick={() => toggleSort('standard')}>{t($locale, 'standardColumn')}<span class="text-[10px] opacity-70">{sortIndicator('standard')}</span></button>
                    </th>
                    <th class="border-b border-gray-200/60 px-3 py-2 text-left dark:border-gray-700/60">
                      <button type="button" class="inline-flex items-center gap-1" onclick={() => toggleSort('country')}>{t($locale, 'countryCodeColumn')}<span class="text-[10px] opacity-70">{sortIndicator('country')}</span></button>
                    </th>
                    <th class="border-b border-gray-200/60 px-3 py-2 text-left dark:border-gray-700/60">
                      <button type="button" class="inline-flex items-center gap-1" onclick={() => toggleSort('generation')}>{t($locale, 'generationColumn')}<span class="text-[10px] opacity-70">{sortIndicator('generation')}</span></button>
                    </th>
                    <th class="border-b border-gray-200/60 px-3 py-2 text-left dark:border-gray-700/60 min-w-[8rem]">
                      <button type="button" class="inline-flex items-center gap-1" onclick={() => toggleSort('uptime')}>{t($locale, 'apUptimeColumn')}<span class="text-[10px] opacity-70">{sortIndicator('uptime')}</span></button>
                    </th>
                    <th class="border-b border-gray-200/60 px-3 py-2 text-left dark:border-gray-700/60 min-w-[8rem]">
                      <button type="button" class="inline-flex items-center gap-1" onclick={() => toggleSort('security')}>{t($locale, 'securityColumn')}<span class="text-[10px] opacity-70">{sortIndicator('security')}</span></button>
                    </th>
                    <th class="border-b border-gray-200/60 px-3 py-2 text-left dark:border-gray-700/60">
                      <button type="button" class="inline-flex items-center gap-1" onclick={() => toggleSort('seen')}>{t($locale, 'seenColumn')}<span class="text-[10px] opacity-70">{sortIndicator('seen')}</span></button>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {#each filtered as network, i (`${network.bssid}-${network.channel}-${i}`)}
                    <tr
                      data-testid="network-row"
                      class={cn(
                        'cursor-pointer transition-colors',
                        $selectedNetwork?.bssid === network.bssid
                          ? 'bg-blue-600/90 text-white'
                          : 'text-gray-900 hover:bg-gray-100/70 dark:text-gray-100 dark:hover:bg-gray-800/70'
                      )}
                      onclick={() => selectNetwork(network.bssid)}
                    >
                      <td class="border-b border-gray-200/40 px-3 py-2 font-mono text-xs whitespace-nowrap dark:border-gray-700/40">{network.bssid.toUpperCase()}</td>
                      <td class="border-b border-gray-200/40 px-3 py-2 min-w-[12rem] dark:border-gray-700/40">
                        <div class="max-w-[20rem] truncate">{network.ssid ?? t($locale, 'hiddenNetwork')}</div>
                      </td>
                      <td class="border-b border-gray-200/40 px-3 py-2 dark:border-gray-700/40">
                        <div class="flex min-w-[7.5rem] items-center gap-2">
                          <span class="tabular-nums whitespace-nowrap">{network.signal} dBm</span>
                          <span class="h-2 w-14 overflow-hidden rounded-full border border-amber-500/60 bg-gray-900/40">
                            <span class="block h-full bg-amber-400" style={`width:${Math.max(8, Math.min(100, ((network.signal + 100) / 70) * 100))}%`}></span>
                          </span>
                        </div>
                      </td>
                      <td class="border-b border-gray-200/40 px-3 py-2 whitespace-nowrap dark:border-gray-700/40">{network.beaconInterval.toFixed(1)} ms</td>
                      <td class="border-b border-gray-200/40 px-3 py-2 whitespace-nowrap dark:border-gray-700/40">{formatRate(network.minDataRate)}</td>
                      <td class="border-b border-gray-200/40 px-3 py-2 whitespace-nowrap dark:border-gray-700/40">{formatRate(network.maxDataRate)}</td>
                      <td class="border-b border-gray-200/40 px-3 py-2 whitespace-nowrap dark:border-gray-700/40">{network.band} GHz</td>
                      <td class="border-b border-gray-200/40 px-3 py-2 whitespace-nowrap dark:border-gray-700/40">{network.channel}</td>
                      <td class="border-b border-gray-200/40 px-3 py-2 whitespace-nowrap dark:border-gray-700/40">{network.channelWidth} MHz</td>
                      <td class="border-b border-gray-200/40 px-3 py-2 whitespace-nowrap dark:border-gray-700/40">{displayStandard(network)}</td>
                      <td class="border-b border-gray-200/40 px-3 py-2 whitespace-nowrap dark:border-gray-700/40">{network.countryCode ?? t($locale, 'notAvailable')}</td>
                      <td class="border-b border-gray-200/40 px-3 py-2 whitespace-nowrap dark:border-gray-700/40">{network.wifiGeneration}</td>
                      <td class="border-b border-gray-200/40 px-3 py-2 whitespace-nowrap dark:border-gray-700/40">{formatDuration(network.apUptimeSecs)}</td>
                      <td class="border-b border-gray-200/40 px-3 py-2 whitespace-nowrap dark:border-gray-700/40">{network.security === 'open' ? t($locale, 'openSecurity') : network.security.toUpperCase()}</td>
                      <td class="border-b border-gray-200/40 px-3 py-2 whitespace-nowrap dark:border-gray-700/40">{formatSeen(network.seenAgeSecs)}</td>
                    </tr>
                  {/each}
                </tbody>
                </table>
              </div>
            </div>
          {/if}
        </div>
      </div>

      {#if $selectedNetwork && showDetailsPane}
      <div class={cn(
        'min-h-0 min-w-0 overflow-y-auto bg-white dark:bg-gray-900',
        'border-t border-gray-200/50 dark:border-gray-700/50 xl:w-[28rem] xl:shrink-0 xl:border-t-0'
      )}>
        {#if $selectedNetwork}
          <div class="mx-auto min-w-0 max-w-5xl p-5 xl:max-w-none">
            <div class="mb-5 border-b border-gray-200/50 pb-4 dark:border-gray-700/50">
              <div class="flex flex-wrap items-start justify-between gap-4">
                <div>
                  <h2 class="text-lg font-bold text-gray-900 dark:text-white">{$selectedNetwork.ssid ?? t($locale, 'hiddenNetwork')}</h2>
                  <div class="mt-1.5 flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
                    <span class="rounded bg-gray-100 px-2 py-0.5 font-mono text-xs dark:bg-gray-800">{$selectedNetwork.bssid.toUpperCase()}</span>
                    <span>{$selectedNetwork.vendor || t($locale, 'unknownVendor')}</span>
                  </div>
                </div>
                <div class="flex max-w-full shrink-0 items-center gap-3 rounded-2xl border border-gray-200/60 bg-gray-50 px-3 py-2 shadow-sm dark:border-gray-700/60 dark:bg-gray-800/60">
                  <VendorLogo vendor={$selectedNetwork.vendor} />
                  <div class="min-w-0 text-right">
                    <div class="text-xs uppercase tracking-wide text-gray-400 dark:text-gray-500">Vendor</div>
                    <div class="truncate text-sm font-semibold text-gray-900 dark:text-white">{$selectedNetwork.vendor || t($locale, 'unknownVendor')}</div>
                  </div>
                </div>
              </div>
            </div>

            <div class="mb-6 grid grid-cols-1 gap-3 md:grid-cols-3">
              <div class="rounded-xl border border-gray-200/50 bg-gray-50 p-4 text-center dark:border-gray-700/50 dark:bg-gray-800/50">
                <div class={cn('text-2xl font-bold tabular-nums', signalColor($selectedNetwork.signal))}>{$selectedNetwork.signal}</div>
                <div class="mt-1 text-xs text-gray-400 dark:text-gray-500">dBm · {translateSignalQuality($locale, signalQuality($selectedNetwork.signal))}</div>
              </div>
              <div class="rounded-xl border border-gray-200/50 bg-gray-50 p-4 text-center dark:border-gray-700/50 dark:bg-gray-800/50">
                <div class="text-2xl font-bold tabular-nums">{$selectedNetwork.snr}</div>
                <div class="mt-1 text-xs text-gray-400 dark:text-gray-500">SNR (dB)</div>
              </div>
              <div class="rounded-xl border border-gray-200/50 bg-gray-50 p-4 text-center dark:border-gray-700/50 dark:bg-gray-800/50">
                <div class="text-2xl font-bold tabular-nums">{$selectedNetwork.noise}</div>
                <div class="mt-1 text-xs text-gray-400 dark:text-gray-500">Noise (dBm)</div>
              </div>
            </div>

            {#if $selectedNetwork.bssLoad}
              <div class="mb-6">
                <h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'bssLoad')}</h3>
                <div class="grid grid-cols-1 gap-3 md:grid-cols-3">
                  <div class="rounded-xl border border-gray-200/50 bg-gray-50 p-4 text-center dark:border-gray-700/50 dark:bg-gray-800/50">
                    <div class="text-2xl font-bold">{(($selectedNetwork.bssLoad.channelUtilization / 255) * 100).toFixed(0)}%</div>
                    <div class="mt-1 text-xs text-gray-400 dark:text-gray-500">{t($locale, 'channelUtilization')}</div>
                  </div>
                  <div class="rounded-xl border border-gray-200/50 bg-gray-50 p-4 text-center dark:border-gray-700/50 dark:bg-gray-800/50">
                    <div class="text-2xl font-bold">{$selectedNetwork.bssLoad.stationCount}</div>
                    <div class="mt-1 text-xs text-gray-400 dark:text-gray-500">{t($locale, 'connectedDevices')}</div>
                  </div>
                  <div class="rounded-xl border border-gray-200/50 bg-gray-50 p-4 text-center dark:border-gray-700/50 dark:bg-gray-800/50">
                    <div class="text-2xl font-bold">{$selectedNetwork.bssLoad.availableCapacity}</div>
                    <div class="mt-1 text-xs text-gray-400 dark:text-gray-500">{t($locale, 'availableCapacity')}</div>
                  </div>
                </div>
              </div>
            {/if}

            <div class="mb-6 grid grid-cols-1 gap-6 xl:grid-cols-2">
              <section>
                <h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'networkInfo')}</h3>
                <div class="overflow-hidden rounded-xl border border-gray-200/50 bg-gray-50 dark:border-gray-700/50 dark:bg-gray-800/50">
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'band')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$selectedNetwork.band} GHz · CH {$selectedNetwork.channel}</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'currentWidth')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$selectedNetwork.channelWidth} MHz</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'maxSupportedWidth')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$selectedNetwork.features?.maxSupportedWidth ? `${$selectedNetwork.features.maxSupportedWidth} MHz` : '-'}</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'frequency')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$selectedNetwork.frequency} MHz</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'beaconInterval')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$selectedNetwork.beaconInterval} ms</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'country')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{detailValue($selectedNetwork.countryCode)}</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'wps')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$selectedNetwork.wpsEnabled ? t($locale, 'enabled') : t($locale, 'disabled')}</span></div>
                </div>
              </section>

              <section>
                <h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'wifiStandards')}</h3>
                <div class="mb-3 flex flex-wrap gap-2">
                  {#each $selectedNetwork.standards as std}
                    <span class="rounded-lg bg-gradient-to-r from-indigo-500 to-blue-600 px-3 py-1.5 text-xs font-semibold text-white shadow-sm">
                      {std === 'be' ? 'WiFi 7' : std === 'ax' ? 'WiFi 6' : std === 'ac' ? 'WiFi 5' : std === 'n' ? 'WiFi 4' : std.toUpperCase()}
                    </span>
                  {/each}
                </div>
                <div class="overflow-hidden rounded-xl border border-gray-200/50 bg-gray-50 dark:border-gray-700/50 dark:bg-gray-800/50">
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'apSpatialStreams')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$selectedNetwork.features?.spatialStreams ?? 1}</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'apCurrentPeakRate')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{formatRate($selectedNetwork.maxDataRate)}</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'apPeakRate')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{formatRate($selectedNetwork.apPeakDataRate)}</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'guardInterval')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$selectedNetwork.features?.guardInterval ? `${$selectedNetwork.features.guardInterval} ns` : '-'}</span></div>
                </div>
              </section>
            </div>

            {#if $currentNetwork && $currentNetwork.bssid === $selectedNetwork.bssid && ($currentNetwork.linkRates?.rxRateMbps !== undefined || $currentNetwork.linkRates?.txRateMbps !== undefined)}
              <div class="mb-6">
                <h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'currentLinkRates')}</h3>
                <div class="overflow-hidden rounded-xl border border-gray-200/50 bg-gray-50 dark:border-gray-700/50 dark:bg-gray-800/50">
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'receiveRate')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{formatOptionalRate($currentNetwork.linkRates?.rxRateMbps)}</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'transmitRate')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{formatOptionalRate($currentNetwork.linkRates?.txRateMbps)}</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'localPeakRate')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{formatOptionalRate($currentNetwork.clientPeakDataRate)}</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'clientSpatialStreams')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$currentNetwork.clientSpatialStreams ?? t($locale, 'notAvailable')}</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'localMaxWidth')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$currentNetwork.localAdapter?.maxSupportedWidth ? `${$currentNetwork.localAdapter.maxSupportedWidth} MHz` : t($locale, 'notAvailable')}</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'localStandards')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$currentNetwork.localAdapter?.supportedStandards?.length ? $currentNetwork.localAdapter.supportedStandards.join('/') : t($locale, 'notAvailable')}</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'localAdapter')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$currentNetwork.localAdapter?.driverName ?? t($locale, 'notAvailable')}</span></div>
                </div>
              </div>
            {/if}

            <div class="mb-6 grid grid-cols-1 gap-6 xl:grid-cols-2">
              <section>
                <h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'performanceFeatures')}</h3>
                <div class="grid grid-cols-1 gap-2">
                  {#each [
                    ['MU-MIMO', $selectedNetwork.features.muMimo],
                    ['OFDMA', $selectedNetwork.features.ofdma],
                    ['BSS Coloring', $selectedNetwork.features.bssColoring],
                    ['TXBF', $selectedNetwork.features.txBeamforming]
                  ] as [label, enabled]}
                    <div class="flex flex-col items-start gap-2 rounded-xl border border-gray-200/50 bg-gray-50 px-4 py-3 dark:border-gray-700/50 dark:bg-gray-800/50">
                      <span class="text-sm leading-6 text-gray-900 dark:text-gray-100">{label}</span>
                      <span class={cn('shrink-0 rounded-full px-2 py-0.5 text-xs font-semibold', enabled ? 'bg-green-500 text-white' : 'bg-gray-200 text-gray-500 dark:bg-gray-700 dark:text-gray-300')}>
                        {enabled ? t($locale, 'supported') : t($locale, 'unsupported')}
                      </span>
                    </div>
                  {/each}
                </div>
              </section>

              <section>
                <h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'protocolExtensions')}</h3>
                <div class="grid grid-cols-1 gap-2">
                  {#each [
                    ['802.11k (RRM)', $selectedNetwork.protocols.rrm],
                    ['802.11r (FT)', $selectedNetwork.protocols.ft],
                    ['802.11v (BSS)', $selectedNetwork.protocols.bssTransition],
                    ['802.11w (PMF)', $selectedNetwork.protocols.pmf]
                  ] as [label, enabled]}
                    <div class="flex flex-col items-start gap-2 rounded-xl border border-gray-200/50 bg-gray-50 px-4 py-3 dark:border-gray-700/50 dark:bg-gray-800/50">
                      <span class="text-sm leading-6 text-gray-900 dark:text-gray-100">{label}</span>
                      <span class={cn('shrink-0 rounded-full px-2 py-0.5 text-xs font-semibold', enabled ? 'bg-green-500 text-white' : 'bg-gray-200 text-gray-500 dark:bg-gray-700 dark:text-gray-300')}>
                        {enabled ? t($locale, 'supported') : t($locale, 'unsupported')}
                      </span>
                    </div>
                  {/each}
                </div>
              </section>
            </div>

            <section class="mb-6">
              <h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'securityInfo')}</h3>
              <div class="overflow-hidden rounded-xl border border-gray-200/50 bg-gray-50 dark:border-gray-700/50 dark:bg-gray-800/50">
                <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'securityType')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$selectedNetwork.security === 'open' ? t($locale, 'openSecurity') : $selectedNetwork.security.toUpperCase()}</span></div>
                <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'authMethod')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$selectedNetwork.securityDetails.authMethod}</span></div>
                <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'encryption')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$selectedNetwork.securityDetails.cipher}</span></div>
                <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">PMF</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$selectedNetwork.securityDetails.pmfCapable ? t($locale, 'supported') : t($locale, 'unsupported')}{#if $selectedNetwork.securityDetails.pmfRequired}<span class="ml-1 text-blue-500">({t($locale, 'required')})</span>{/if}</span></div>
              </div>
            </section>

            <Button variant="outline" class="w-full native-button" onclick={() => showIEDetails = true}>{t($locale, 'viewBeaconFrame')}</Button>
          </div>
        {/if}
      </div>
      {/if}
    {:else if activeTab === 'channels'}
      <div class="flex-1 overflow-y-auto bg-gray-50/30 dark:bg-gray-800/30"><ChannelView /></div>
    {:else if activeTab === 'groups'}
      <div class="flex-1 overflow-y-auto bg-gray-50/30 p-5 dark:bg-gray-800/30" data-testid="groups-view">
        {#if $networkGroups.length === 0}
          <div class="py-12 text-center text-sm text-gray-400 dark:text-gray-500" data-testid="groups-empty">
            <div class="font-medium">{t($locale, 'noNetworkGroups')}</div>
            <div class="mt-1 text-xs text-gray-300 dark:text-gray-600">{t($locale, 'noNetworkGroupsHint')}</div>
          </div>
        {:else}
          <div class="space-y-5">
            <section class="grid gap-3 xl:grid-cols-4" data-testid="groups-summary">
              <div class="rounded-2xl border border-gray-200/60 bg-white/90 p-4 shadow-sm dark:border-gray-700/60 dark:bg-gray-800/80">
                <div class="text-xs font-semibold uppercase tracking-[0.24em] text-gray-400 dark:text-gray-500">{t($locale, 'groupsTab')}</div>
                <div class="mt-2 text-2xl font-bold text-gray-900 dark:text-white">{groupSummary.totalGroups}</div>
                <div class="mt-1 text-xs text-gray-500 dark:text-gray-400">{t($locale, 'groupSummaryGroupsHint')}</div>
              </div>
              <div class="rounded-2xl border border-gray-200/60 bg-white/90 p-4 shadow-sm dark:border-gray-700/60 dark:bg-gray-800/80">
                <div class="text-xs font-semibold uppercase tracking-[0.24em] text-gray-400 dark:text-gray-500">{t($locale, 'groupSummaryTotalAps')}</div>
                <div class="mt-2 text-2xl font-bold text-gray-900 dark:text-white">{groupSummary.totalAps}</div>
                <div class="mt-1 text-xs text-gray-500 dark:text-gray-400">{t($locale, 'groupSummaryTotalApsHint')}</div>
              </div>
              <div class="rounded-2xl border border-gray-200/60 bg-white/90 p-4 shadow-sm dark:border-gray-700/60 dark:bg-gray-800/80">
                <div class="text-xs font-semibold uppercase tracking-[0.24em] text-gray-400 dark:text-gray-500">{t($locale, 'groupSummaryRoaming')}</div>
                <div class="mt-2 text-2xl font-bold text-gray-900 dark:text-white">{groupSummary.roamingReady}</div>
                <div class="mt-1 text-xs text-gray-500 dark:text-gray-400">{t($locale, 'groupSummaryRoamingHint')}</div>
              </div>
              <div class="rounded-2xl border border-gray-200/60 bg-white/90 p-4 shadow-sm dark:border-gray-700/60 dark:bg-gray-800/80">
                <div class="text-xs font-semibold uppercase tracking-[0.24em] text-gray-400 dark:text-gray-500">{t($locale, 'groupSummaryStrongest')}</div>
                <div class="mt-2 text-2xl font-bold text-gray-900 dark:text-white">{groupSummary.strongest !== undefined ? `${groupSummary.strongest} dBm` : t($locale, 'notAvailable')}</div>
                <div class="mt-1 text-xs text-gray-500 dark:text-gray-400">{t($locale, 'groupSummaryStrongestHint')}</div>
              </div>
            </section>

            <div class="grid gap-4 2xl:grid-cols-2">
            {#each $networkGroups as group (group.ssid)}
              {@const bestNetwork = [...group.networks].sort((left, right) => right.signal - left.signal)[0]}
              {@const protocolBadges = groupProtocols(group)}
              <article class="rounded-2xl border border-gray-200/60 bg-white/95 p-5 shadow-sm transition-shadow hover:shadow-md dark:border-gray-700/60 dark:bg-gray-800/85" data-testid="group-card">
                <div class="flex flex-wrap items-start justify-between gap-3">
                  <div class="min-w-0">
                    <h3 class="truncate text-lg font-bold text-gray-900 dark:text-white">{group.ssid}</h3>
                    <p class="mt-1 text-sm text-gray-500 dark:text-gray-400">{t($locale, 'apsSummary', { count: group.totalAps, bands: group.bands.join(', '), signal: group.bestSignal })}</p>
                  </div>
                  <div class="flex flex-wrap gap-1.5">
                    {#if protocolBadges.length === 0}
                      <span class="rounded-full bg-gray-100 px-2.5 py-1 text-xs font-medium text-gray-500 dark:bg-gray-700 dark:text-gray-300">{t($locale, 'groupNoRoaming')}</span>
                    {:else}
                      {#each protocolBadges as badge}
                        <span class="rounded-full bg-blue-100 px-2.5 py-1 text-xs font-medium text-blue-700 dark:bg-blue-900/50 dark:text-blue-300">{badge}</span>
                      {/each}
                    {/if}
                  </div>
                </div>

                <div class="mt-4 grid gap-3 md:grid-cols-3">
                  <div class="rounded-xl bg-gray-50 p-3 dark:bg-gray-700/40">
                    <div class="text-xs uppercase tracking-wide text-gray-400 dark:text-gray-500">{t($locale, 'groupSummaryBestAp')}</div>
                    <div class="mt-1 text-sm font-semibold text-gray-900 dark:text-white">{bestNetwork?.bssid.toUpperCase() ?? t($locale, 'notAvailable')}</div>
                  </div>
                  <div class="rounded-xl bg-gray-50 p-3 dark:bg-gray-700/40">
                    <div class="text-xs uppercase tracking-wide text-gray-400 dark:text-gray-500">{t($locale, 'groupSummaryBands')}</div>
                    <div class="mt-1 text-sm font-semibold text-gray-900 dark:text-white">{group.bands.map((band) => bandLabel(band)).join(' / ')}</div>
                  </div>
                  <div class="rounded-xl bg-gray-50 p-3 dark:bg-gray-700/40">
                    <div class="text-xs uppercase tracking-wide text-gray-400 dark:text-gray-500">{t($locale, 'groupSummaryChannels')}</div>
                    <div class="mt-1 text-sm font-semibold text-gray-900 dark:text-white">{[...new Set(group.networks.map((network) => `CH ${network.channel}`))].join(', ')}</div>
                  </div>
                </div>

                <div class="mt-4 space-y-2">
                  {#each [...group.networks].sort((left, right) => right.signal - left.signal) as net (net.bssid)}
                    <div class="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-gray-200/70 bg-gray-50 px-3 py-3 text-sm dark:border-gray-700/60 dark:bg-gray-700/30">
                      <div class="min-w-0">
                        <div class="flex items-center gap-2">
                          <span class={cn('h-2.5 w-2.5 rounded-full', net.signal >= -50 ? 'bg-green-500' : net.signal >= -70 ? 'bg-yellow-500' : 'bg-red-500')}></span>
                          <span class="font-mono text-xs text-gray-700 dark:text-gray-200">{net.bssid.toUpperCase()}</span>
                        </div>
                        <div class="mt-1 text-xs text-gray-500 dark:text-gray-400">
                          {bandLabel(net.band)} · CH {net.channel} · {net.channelWidth} MHz
                        </div>
                      </div>
                      <div class="flex items-center gap-3 text-xs text-gray-500 dark:text-gray-400">
                        <span class="rounded-full bg-white px-2 py-1 font-medium text-gray-700 dark:bg-gray-800 dark:text-gray-200">{displayStandard(net)}</span>
                        <span class="font-semibold tabular-nums text-gray-900 dark:text-white">{net.signal} dBm</span>
                      </div>
                    </div>
                  {/each}
                </div>
              </article>
            {/each}
            </div>
          </div>
        {/if}
      </div>
    {:else}
      <div class="flex-1 overflow-y-auto bg-gray-50/30 dark:bg-gray-800/30"><RoamingTest /></div>
    {/if}
  </div>

  {#if showIEDetails && $selectedNetwork}
    <IEDetailsPanel network={$selectedNetwork} onClose={() => showIEDetails = false} />
  {/if}
</div>

