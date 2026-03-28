<script lang="ts">
  import { onMount } from 'svelte';
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
    scanStats
  } from '$lib/stores';
  import { locale, t, translateSignalQuality } from '$lib/i18n';
  import { cn, signalColor, signalQuality } from '$lib/utils';
  import IEDetailsPanel from '$lib/components/IEDetailsPanel.svelte';
  import ChannelView from '$lib/components/ChannelView.svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import RoamingTest from '$lib/components/RoamingTest.svelte';
  import VendorLogo from '$lib/components/VendorLogo.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import type { Network } from '$lib/types';

  let filterText = $state('');
  let activeBand = $state<'all' | '2.4' | '5' | '6'>('all');
  let activeTab = $state<'networks' | 'channels' | 'groups' | 'roaming'>('networks');
  let showIEDetails = $state(false);
  let showExportMenu = $state(false);

  const filtered = $derived.by(() => {
    let list = $networks;

    if (activeBand !== 'all') {
      list = $byBand[activeBand];
    }

    if (filterText) {
      const search = filterText.toLowerCase();
      list = list.filter((n) => n.ssid?.toLowerCase().includes(search) ?? false);
    }

    return [...list].sort((a, b) => b.signal - a.signal);
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

  function displayStandard(network: Network | null) {
    if (!network || network.standards.length === 0) {
      return '-';
    }

    return network.standards[network.standards.length - 1];
  }

  onMount(() => {
    scan();
    fetchCurrentNetwork();
  });
</script>

<div class="flex h-screen flex-col overflow-hidden rounded-lg border border-gray-200/50 bg-white text-gray-900 shadow-2xl dark:border-gray-700/50 dark:bg-gray-900 dark:text-gray-100">
  <TitleBar />

  <header class="shrink-0 border-b border-gray-200/50 bg-gray-50/80 px-4 py-2.5 backdrop-blur-xl dark:border-gray-700/50 dark:bg-gray-800/80">
    <div class="flex items-center justify-between gap-3">
      <div class="flex items-center gap-3">
        <div class="flex overflow-hidden rounded-lg bg-gray-100 p-0.5 text-xs font-medium dark:bg-gray-700">
          <button class="px-3 py-1.5 rounded-md transition-all duration-200 {activeTab === 'networks' ? 'bg-white text-gray-900 shadow-sm dark:bg-gray-600 dark:text-white' : 'text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-200'}" onclick={() => activeTab = 'networks'}>{t($locale, 'networksTab')}</button>
          <button class="px-3 py-1.5 rounded-md transition-all duration-200 {activeTab === 'channels' ? 'bg-white text-gray-900 shadow-sm dark:bg-gray-600 dark:text-white' : 'text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-200'}" onclick={() => activeTab = 'channels'}>{t($locale, 'channelsTab')}</button>
          <button class="px-3 py-1.5 rounded-md transition-all duration-200 {activeTab === 'groups' ? 'bg-white text-gray-900 shadow-sm dark:bg-gray-600 dark:text-white' : 'text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-200'}" onclick={() => activeTab = 'groups'}>{t($locale, 'groupsTab')}</button>
          <button class="px-3 py-1.5 rounded-md transition-all duration-200 {activeTab === 'roaming' ? 'bg-white text-gray-900 shadow-sm dark:bg-gray-600 dark:text-white' : 'text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-200'}" onclick={() => activeTab = 'roaming'}>{t($locale, 'roamingTab')}</button>
        </div>
      </div>

      <div class="flex items-center gap-2">
        <div class="relative">
          <Button variant="ghost" size="sm" onclick={() => showExportMenu = !showExportMenu}>{t($locale, 'export')}</Button>
          {#if showExportMenu}
            <div class="absolute right-0 top-full z-50 mt-1 min-w-36 rounded-lg border border-gray-200/80 bg-white py-1 shadow-xl backdrop-blur-xl dark:border-gray-700/80 dark:bg-gray-800">
              <button class="mx-1 block w-full rounded-md px-3 py-2 text-left text-sm transition-colors hover:bg-gray-100 dark:hover:bg-gray-700/50" onclick={exportJSON}>{t($locale, 'exportJson')}</button>
              <button class="mx-1 block w-full rounded-md px-3 py-2 text-left text-sm transition-colors hover:bg-gray-100 dark:hover:bg-gray-700/50" onclick={exportCSV}>{t($locale, 'exportCsv')}</button>
            </div>
          {/if}
        </div>

        <Button variant="ghost" size="sm" onclick={toggleMonitoring}>
          {$isMonitoring ? t($locale, 'monitoring') : t($locale, 'startMonitoring')}
        </Button>

        <Button size="sm" disabled={$isScanning} class="native-button" onclick={() => scan()}>
          {$isScanning ? t($locale, 'scanning') : t($locale, 'scan')}
        </Button>
      </div>
    </div>

    {#if $currentNetwork}
      <div class="mt-2 flex items-center gap-3 rounded-lg border border-green-200/50 bg-green-50/80 px-3 py-2 text-sm text-green-800 dark:border-green-800/50 dark:bg-green-900/20 dark:text-green-200">
        <span class="inline-flex h-2 w-2 rounded-full bg-green-500"></span>
        <span class="font-medium">{t($locale, 'connectedTo')} {$currentNetwork.ssid ?? t($locale, 'hiddenNetwork')}</span>
        <span class="text-xs text-green-700 dark:text-green-300">CH {$currentNetwork.channel} · {$currentNetwork.band} GHz · {$currentNetwork.signal} dBm</span>
      </div>
    {/if}
  </header>

  {#if $error}
    <div class="border-b border-red-200/50 bg-red-50/80 px-4 py-2 text-sm text-red-700 dark:border-red-800/50 dark:bg-red-900/20 dark:text-red-300">{$error}</div>
  {/if}

  <div class="flex flex-1 overflow-hidden">
    {#if activeTab === 'networks'}
      <div class="flex w-[58%] min-w-0 shrink-0 flex-col border-r border-gray-200/50 bg-gray-50/50 dark:border-gray-700/50 dark:bg-gray-800/50">
        <div class="shrink-0 border-b border-gray-200/50 bg-white/50 p-3 backdrop-blur-sm dark:border-gray-700/50 dark:bg-gray-800/50">
          <div class="flex gap-2">
            <input type="search" bind:value={filterText} placeholder={t($locale, 'searchNetworks')} class="w-full rounded-lg border border-gray-200 bg-white px-3 py-2 text-xs text-gray-900 placeholder-gray-400 focus:border-blue-500/50 focus:ring-2 focus:ring-blue-500/30 dark:border-gray-600/50 dark:bg-gray-700/50 dark:text-gray-100" />
            <div class="flex overflow-hidden rounded-lg bg-gray-100 p-0.5 dark:bg-gray-700">
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
          <div class="mt-2 flex items-center justify-between text-xs text-gray-500 dark:text-gray-400">
            <span class="font-medium">{t($locale, 'networksCount', { count: filtered.length })}</span>
            {#if $scanStats}
              <span>{t($locale, 'scanDuration')}: {$scanStats.scanDurationMs} ms</span>
            {/if}
          </div>
        </div>

        <div class="flex-1 overflow-auto">
          {#if $isScanning && $networks.length === 0}
            <div class="py-12 text-center text-sm text-gray-400 dark:text-gray-500">{t($locale, 'scanningInProgress')}</div>
          {:else if filtered.length === 0}
            <div class="py-12 text-center text-sm text-gray-400 dark:text-gray-500">{t($locale, 'noNetworksFound')}</div>
          {:else}
            <div class="min-w-[1180px]">
              <div class="grid grid-cols-[160px_260px_110px_100px_110px_110px_90px_70px_90px_90px_70px_70px_150px_150px_90px] border-b border-gray-200/60 bg-gray-100/80 px-3 py-2 text-xs font-semibold uppercase tracking-wide text-gray-600 dark:border-gray-700/60 dark:bg-gray-800 dark:text-gray-300">
                <div>{t($locale, 'bssidColumn')}</div>
                <div>{t($locale, 'networkNameColumn')}</div>
                <div>{t($locale, 'rssiColumn')}</div>
                <div>{t($locale, 'beaconColumn')}</div>
                <div>{t($locale, 'minRateColumn')}</div>
                <div>{t($locale, 'maxRateColumn')}</div>
                <div>{t($locale, 'band')}</div>
                <div>{t($locale, 'channelColumn')}</div>
                <div>{t($locale, 'widthColumn')}</div>
                <div>{t($locale, 'standardColumn')}</div>
                <div>{t($locale, 'countryCodeColumn')}</div>
                <div>{t($locale, 'generationColumn')}</div>
                <div>{t($locale, 'apUptimeColumn')}</div>
                <div>{t($locale, 'securityColumn')}</div>
                <div>{t($locale, 'seenColumn')}</div>
              </div>

              {#each filtered as network (network.bssid)}
                <button
                  type="button"
                  class={cn(
                    'grid w-full grid-cols-[160px_260px_110px_100px_110px_110px_90px_70px_90px_90px_70px_70px_150px_150px_90px] border-b border-gray-200/40 px-3 py-2 text-left text-sm transition-colors dark:border-gray-700/40',
                    $selectedNetwork?.bssid === network.bssid
                      ? 'bg-blue-600/90 text-white'
                      : 'bg-transparent text-gray-900 hover:bg-gray-100/70 dark:text-gray-100 dark:hover:bg-gray-800/70'
                  )}
                  onclick={() => selectNetwork(network.bssid)}
                >
                  <div class="truncate font-mono text-xs">{network.bssid.toUpperCase()}</div>
                  <div class="truncate">{network.ssid ?? t($locale, 'hiddenNetwork')}</div>
                  <div class="flex items-center gap-2">
                    <span class="tabular-nums">{network.signal} dBm</span>
                    <span class="h-2 w-14 overflow-hidden rounded-full border border-amber-500/60 bg-gray-900/40">
                      <span
                        class="block h-full bg-amber-400"
                        style={`width:${Math.max(8, Math.min(100, ((network.signal + 100) / 70) * 100))}%`}
                      ></span>
                    </span>
                  </div>
                  <div>{network.beaconInterval.toFixed(1)} ms</div>
                  <div>{formatRate(network.minDataRate)}</div>
                  <div>{formatRate(network.maxDataRate)}</div>
                  <div>{network.band} GHz</div>
                  <div>{network.channel}</div>
                  <div>{network.channelWidth} MHz</div>
                  <div>{displayStandard(network)}</div>
                  <div>{network.countryCode ?? t($locale, 'notAvailable')}</div>
                  <div>{network.wifiGeneration}</div>
                  <div>{formatDuration(network.apUptimeSecs)}</div>
                  <div>{network.security === 'open' ? t($locale, 'openSecurity') : network.security.toUpperCase()}</div>
                  <div>{formatSeen(network.seenAgeSecs)}</div>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>

      <div class="flex-1 min-w-0 overflow-y-auto bg-white dark:bg-gray-900">
        {#if $selectedNetwork}
          <div class="mx-auto min-w-0 max-w-4xl p-5">
            <div class="mb-5 border-b border-gray-200/50 pb-4 dark:border-gray-700/50">
              <div class="flex items-start justify-between gap-4">
                <div>
                  <h2 class="text-lg font-bold text-gray-900 dark:text-white">{$selectedNetwork.ssid ?? t($locale, 'hiddenNetwork')}</h2>
                  <div class="mt-1.5 flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
                    <span class="rounded bg-gray-100 px-2 py-0.5 font-mono text-xs dark:bg-gray-800">{$selectedNetwork.bssid.toUpperCase()}</span>
                    <span>{$selectedNetwork.vendor || t($locale, 'unknownVendor')}</span>
                  </div>
                </div>
                <div class="flex items-center gap-3 rounded-2xl border border-gray-200/60 bg-gray-50 px-3 py-2 shadow-sm dark:border-gray-700/60 dark:bg-gray-800/60">
                  <VendorLogo vendor={$selectedNetwork.vendor} />
                  <div class="text-right">
                    <div class="text-xs uppercase tracking-wide text-gray-400 dark:text-gray-500">Vendor</div>
                    <div class="text-sm font-semibold text-gray-900 dark:text-white">{$selectedNetwork.vendor || t($locale, 'unknownVendor')}</div>
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

            <div class="mb-6 grid grid-cols-1 gap-6 lg:grid-cols-2">
              <section>
                <h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'networkInfo')}</h3>
                <div class="overflow-hidden rounded-xl border border-gray-200/50 bg-gray-50 dark:border-gray-700/50 dark:bg-gray-800/50">
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'band')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$selectedNetwork.band} GHz · CH {$selectedNetwork.channel}</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'channelWidth')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$selectedNetwork.channelWidth} MHz</span></div>
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
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'spatialStreams')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$selectedNetwork.features?.spatialStreams ?? 1}</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'maxRate')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{formatRate($selectedNetwork.maxDataRate)}</span></div>
                  <div class="flex items-start justify-between gap-4 px-4 py-2.5 text-sm"><span class="shrink-0 text-gray-500 dark:text-gray-400">{t($locale, 'guardInterval')}</span><span class="max-w-[60%] min-w-0 break-words text-right font-medium">{$selectedNetwork.features?.guardInterval ? `${$selectedNetwork.features.guardInterval} ns` : '-'}</span></div>
                </div>
              </section>
            </div>

            <div class="mb-6 grid grid-cols-1 gap-6 lg:grid-cols-2">
              <section>
                <h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'performanceFeatures')}</h3>
                <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
                  {#each [
                    ['MU-MIMO', $selectedNetwork.features.muMimo],
                    ['OFDMA', $selectedNetwork.features.ofdma],
                    ['BSS Coloring', $selectedNetwork.features.bssColoring],
                    ['TXBF', $selectedNetwork.features.txBeamforming]
                  ] as [label, enabled]}
                    <div class="flex items-center justify-between rounded-xl border border-gray-200/50 bg-gray-50 px-4 py-3 dark:border-gray-700/50 dark:bg-gray-800/50">
                      <span class="min-w-0 pr-2 text-sm [overflow-wrap:anywhere]">{label}</span>
                      <span class={cn('shrink-0 rounded-full px-2 py-0.5 text-xs font-semibold', enabled ? 'bg-green-500 text-white' : 'bg-gray-200 text-gray-500 dark:bg-gray-700 dark:text-gray-300')}>
                        {enabled ? t($locale, 'supported') : t($locale, 'unsupported')}
                      </span>
                    </div>
                  {/each}
                </div>
              </section>

              <section>
                <h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'protocolExtensions')}</h3>
                <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
                  {#each [
                    ['802.11k (RRM)', $selectedNetwork.protocols.rrm],
                    ['802.11r (FT)', $selectedNetwork.protocols.ft],
                    ['802.11v (BSS)', $selectedNetwork.protocols.bssTransition],
                    ['802.11w (PMF)', $selectedNetwork.protocols.pmf]
                  ] as [label, enabled]}
                    <div class="flex items-center justify-between rounded-xl border border-gray-200/50 bg-gray-50 px-4 py-3 dark:border-gray-700/50 dark:bg-gray-800/50">
                      <span class="min-w-0 pr-2 text-sm [overflow-wrap:anywhere]">{label}</span>
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
        {:else}
          <div class="flex h-full items-center justify-center bg-gray-50/50 text-gray-400 dark:bg-gray-800/30 dark:text-gray-500">
            <div class="text-center">
              <div class="text-sm font-medium">{t($locale, 'selectNetworkTitle')}</div>
              <div class="mt-1 text-xs text-gray-300 dark:text-gray-600">{t($locale, 'selectNetworkHint')}</div>
            </div>
          </div>
        {/if}
      </div>
    {:else if activeTab === 'channels'}
      <div class="flex-1 overflow-y-auto bg-gray-50/30 dark:bg-gray-800/30"><ChannelView /></div>
    {:else if activeTab === 'groups'}
      <div class="flex-1 overflow-y-auto bg-gray-50/30 p-5 dark:bg-gray-800/30">
        {#if $networkGroups.length === 0}
          <div class="py-12 text-center text-sm text-gray-400 dark:text-gray-500">
            <div class="font-medium">{t($locale, 'noNetworkGroups')}</div>
            <div class="mt-1 text-xs text-gray-300 dark:text-gray-600">{t($locale, 'noNetworkGroupsHint')}</div>
          </div>
        {:else}
          <div class="max-w-3xl space-y-3">
            {#each $networkGroups as group (group.ssid)}
              <div class="rounded-xl border border-gray-200/50 bg-white p-4 shadow-sm transition-shadow hover:shadow-md dark:border-gray-700/50 dark:bg-gray-800/80">
                <div class="mb-3 flex items-center justify-between">
                  <div>
                    <h3 class="font-bold text-gray-900 dark:text-white">{group.ssid}</h3>
                    <div class="mt-0.5 text-xs text-gray-500 dark:text-gray-400">{t($locale, 'apsSummary', { count: group.totalAps, bands: group.bands.join(', '), signal: group.bestSignal })}</div>
                  </div>
                  <div class="flex gap-1.5">
                    {#if group.supportsFastRoaming}
                      <span class="rounded-md bg-green-100 px-2 py-1 text-xs font-medium text-green-700 dark:bg-green-900/50 dark:text-green-300">802.11r</span>
                    {/if}
                    {#if group.supportsBssTransition}
                      <span class="rounded-md bg-blue-100 px-2 py-1 text-xs font-medium text-blue-700 dark:bg-blue-900/50 dark:text-blue-300">802.11v</span>
                    {/if}
                  </div>
                </div>
                <div class="space-y-1.5">
                  {#each group.networks as net (net.bssid)}
                    <div class="flex items-center justify-between rounded-lg bg-gray-50 p-2.5 text-xs dark:bg-gray-700/50">
                      <div class="flex items-center gap-2">
                        <span class={cn('h-2 w-2 rounded-full', net.signal >= -50 ? 'bg-green-500' : net.signal >= -70 ? 'bg-yellow-500' : 'bg-red-500')}></span>
                        <span class="font-mono">{net.bssid.toUpperCase()}</span>
                      </div>
                      <div class="flex items-center gap-3 text-gray-500 dark:text-gray-400">
                        <span>CH {net.channel}</span>
                        <span>{net.band} GHz</span>
                        <span class="font-medium tabular-nums">{net.signal} dBm</span>
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            {/each}
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

