<script lang="ts">
  import { onMount } from 'svelte';
  import {
    networks, scan, isScanning, error, byBand, selectedNetwork, selectedBssid,
    currentNetwork, fetchCurrentNetwork, isMonitoring, startMonitor, stopMonitor,
    networkGroups, scanStats
  } from '$lib/stores';
  import { cn, signalColor, signalQuality } from '$lib/utils';
  import NetworkCard from '$lib/components/NetworkCard.svelte';
  import IEDetailsPanel from '$lib/components/IEDetailsPanel.svelte';
  import ChannelView from '$lib/components/ChannelView.svelte';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import Button from '$lib/components/ui/button.svelte';

  let filterText = $state('');
  let activeBand = $state<'all' | '2.4' | '5' | '6'>('all');
  let activeTab = $state<'networks' | 'channels' | 'groups'>('networks');
  let showIEDetails = $state(false);
  let showExportMenu = $state(false);

  const filtered = $derived.by(() => {
    let list = $networks;

    if (activeBand !== 'all') {
      list = $byBand[activeBand];
    }

    if (filterText) {
      const search = filterText.toLowerCase();
      list = list.filter(n => n.ssid?.toLowerCase().includes(search) ?? false);
    }

    return list.sort((a, b) => b.signal - a.signal);
  });

  // Export functions
  function exportJSON() {
    const data = JSON.stringify($networks, null, 2);
    downloadFile(data, 'unifi-scan.json', 'application/json');
    showExportMenu = false;
  }

  function exportCSV() {
    const headers = ['SSID', 'BSSID', 'Channel', 'Band', 'Signal (dBm)', 'Standard', 'Security', 'Vendor'];
    const rows = $networks.map(n => [
      n.ssid ?? '[Hidden]',
      n.bssid,
      n.channel,
      n.band,
      n.signal,
      n.standards.join('/'),
      n.security,
      n.vendor
    ]);

    const csv = [headers, ...rows].map(r => r.join(',')).join('\n');
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

  // Toggle monitoring
  function toggleMonitoring() {
    if ($isMonitoring) {
      stopMonitor();
    } else {
      startMonitor();
    }
  }

  onMount(() => {
    scan();
    fetchCurrentNetwork();
  });
</script>

<div class="h-screen flex flex-col bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 overflow-hidden rounded-lg shadow-2xl border border-gray-200/50 dark:border-gray-700/50">
  <!-- Custom Title Bar -->
  <TitleBar />

  <!-- Header -->
  <header class="bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-xl border-b border-gray-200/50 dark:border-gray-700/50 px-4 py-2.5 shrink-0">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-3">
        <div class="flex rounded-lg overflow-hidden bg-gray-100 dark:bg-gray-700 p-0.5 text-xs font-medium">
          <button
            class="px-3 py-1.5 rounded-md transition-all duration-200 {activeTab === 'networks' ? 'bg-white dark:bg-gray-600 text-gray-900 dark:text-white shadow-sm' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'}"
            onclick={() => activeTab = 'networks'}
          >网络列表</button>
          <button
            class="px-3 py-1.5 rounded-md transition-all duration-200 {activeTab === 'channels' ? 'bg-white dark:bg-gray-600 text-gray-900 dark:text-white shadow-sm' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'}"
            onclick={() => activeTab = 'channels'}
          >信道分析</button>
          <button
            class="px-3 py-1.5 rounded-md transition-all duration-200 {activeTab === 'groups' ? 'bg-white dark:bg-gray-600 text-gray-900 dark:text-white shadow-sm' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'}"
            onclick={() => activeTab = 'groups'}
          >网络分组</button>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <!-- Export -->
        <div class="relative">
          <Button variant="ghost" size="sm" onclick={() => showExportMenu = !showExportMenu}>
            <svg class="w-4 h-4 mr-1.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
            </svg>
            导出
          </Button>
          {#if showExportMenu}
            <div class="absolute right-0 top-full mt-1 bg-white dark:bg-gray-800 border border-gray-200/80 dark:border-gray-700/80 rounded-lg shadow-xl z-50 py-1 min-w-36 backdrop-blur-xl animate-scale-in">
              <button class="w-full text-left px-3 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700/50 transition-colors rounded-md mx-1" onclick={exportJSON}>导出 JSON</button>
              <button class="w-full text-left px-3 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700/50 transition-colors rounded-md mx-1" onclick={exportCSV}>导出 CSV</button>
            </div>
          {/if}
        </div>
        <!-- Monitoring toggle -->
        <Button variant="ghost" size="sm" onclick={toggleMonitoring}>
          <div class="relative">
            <svg class="w-4 h-4 mr-1.5 {$isMonitoring ? 'text-green-500' : 'text-gray-400'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
            </svg>
            {#if $isMonitoring}
              <span class="absolute -top-0.5 -right-0.5 w-2 h-2 bg-green-500 rounded-full animate-pulse"></span>
            {/if}
          </div>
          {$isMonitoring ? '监控中' : '开始监控'}
        </Button>
        <Button size="sm" onclick={() => scan()} disabled={$isScanning} class="native-button">
          {#if $isScanning}
            <svg class="w-4 h-4 mr-1.5 animate-spin" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
          {:else}
            <svg class="w-4 h-4 mr-1.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
          {/if}
          {$isScanning ? '扫描中...' : '扫描'}
        </Button>
      </div>
    </div>

    <!-- Current Connection Banner -->
    {#if $currentNetwork}
      <div class="mt-2 flex items-center gap-3 px-3 py-2 bg-green-50/80 dark:bg-green-900/20 rounded-lg border border-green-200/50 dark:border-green-800/50 animate-slide-up">
        <div class="flex items-center gap-2">
          <div class="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
          <span class="font-medium text-green-800 dark:text-green-200 text-sm">已连接: {$currentNetwork.ssid ?? '[Hidden]'}</span>
        </div>
        <div class="text-xs text-green-700 dark:text-green-300">
          CH {$currentNetwork.channel} · {$currentNetwork.band}GHz · {$currentNetwork.signal} dBm
          {#if $currentNetwork.standards.length > 0}
            · {$currentNetwork.standards[$currentNetwork.standards.length - 1].toUpperCase()}
          {/if}
        </div>
      </div>
    {/if}
  </header>

  <!-- Error Banner -->
  {#if $error}
    <div class="bg-red-50/80 dark:bg-red-900/20 border-b border-red-200/50 dark:border-red-800/50 text-red-700 dark:text-red-300 px-4 py-2 text-sm animate-slide-up">
      <div class="flex items-center gap-2">
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        {$error}
      </div>
    </div>
  {/if}

  <!-- Main Content -->
  <div class="flex-1 flex overflow-hidden">
    {#if activeTab === 'networks'}
      <!-- Network List View -->
      <div class="w-[340px] border-r border-gray-200/50 dark:border-gray-700/50 flex flex-col bg-gray-50/50 dark:bg-gray-800/50 backdrop-blur-sm shrink-0">
        <!-- Filters -->
        <div class="p-3 border-b border-gray-200/50 dark:border-gray-700/50 shrink-0 bg-white/50 dark:bg-gray-800/50 backdrop-blur-sm">
          <div class="flex gap-2">
            <div class="relative flex-1">
              <svg class="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
              </svg>
              <input
                type="search"
                placeholder="搜索网络..."
                bind:value={filterText}
                class="w-full pl-8 pr-3 py-2 text-xs bg-white dark:bg-gray-700/50 border border-gray-200 dark:border-gray-600/50 rounded-lg text-gray-900 dark:text-gray-100 placeholder-gray-400 focus:ring-2 focus:ring-blue-500/30 focus:border-blue-500/50 transition-all"
              />
            </div>
            <div class="flex rounded-lg overflow-hidden bg-gray-100 dark:bg-gray-700 p-0.5">
              {#each ['all', '2.4', '5', '6'] as band}
                <button
                  class="px-2.5 py-1.5 text-xs font-medium rounded-md transition-all duration-200 {activeBand === band
                    ? 'bg-white dark:bg-gray-600 text-gray-900 dark:text-white shadow-sm'
                    : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'}"
                  onclick={() => activeBand = band as typeof activeBand}
                >
                  {band === 'all' ? '全部' : `${band}G`}
                </button>
              {/each}
            </div>
          </div>
          <div class="mt-2 text-xs text-gray-500 dark:text-gray-400 flex justify-between items-center">
            <span class="font-medium">{$networks.length} 个网络</span>
            {#if $scanStats}
              <span class="text-gray-400">{$scanStats.scanDurationMs}ms</span>
            {/if}
          </div>
        </div>

        <!-- Network List -->
        <div class="flex-1 overflow-y-auto p-2 space-y-1.5">
          {#if $isScanning && $networks.length === 0}
            <div class="text-center text-gray-400 dark:text-gray-500 py-12 animate-pulse">
              <svg class="w-8 h-8 mx-auto mb-3 text-gray-300 dark:text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
              </svg>
              <div class="text-sm">扫描中...</div>
            </div>
          {:else if filtered.length === 0}
            <div class="text-center text-gray-400 dark:text-gray-500 py-12">
              <svg class="w-8 h-8 mx-auto mb-3 text-gray-300 dark:text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M18.364 5.636a9 9 0 010 12.728m0 0l-2.829-2.829m2.829 2.829L21 21M15.536 8.464a5 5 0 010 7.072m0 0l-2.829-2.829m-4.243 2.829a4.978 4.978 0 01-1.414-2.83m-1.414 5.658a9 9 0 01-2.167-9.238m7.824 2.167a1 1 0 111.414 1.414m-1.414-1.414L3 3m8.293 8.293l1.414 1.414" />
              </svg>
              <div class="text-sm">未发现网络</div>
            </div>
          {:else}
            {#each filtered as network (network.bssid)}
              <NetworkCard {network} />
            {/each}
          {/if}
        </div>
      </div>

      <!-- Right Panel: Network Details -->
      <div class="flex-1 flex flex-col bg-white dark:bg-gray-900 overflow-y-auto">
        {#if $selectedNetwork}
          <div class="p-5 max-w-2xl animate-fade-in">
            <!-- Title -->
            <div class="mb-5 pb-4 border-b border-gray-200/50 dark:border-gray-700/50">
              <h2 class="text-lg font-bold text-gray-900 dark:text-white">
                {$selectedNetwork.ssid ?? '[隐藏网络]'}
              </h2>
              <div class="mt-1.5 text-sm text-gray-500 dark:text-gray-400 flex items-center gap-2">
                <span class="font-mono text-xs bg-gray-100 dark:bg-gray-800 px-2 py-0.5 rounded">{$selectedNetwork.bssid.toUpperCase()}</span>
                <span class="text-gray-300 dark:text-gray-600">·</span>
                <span>{$selectedNetwork.vendor || 'Unknown Vendor'}</span>
              </div>
            </div>

            <!-- Signal Section -->
            <div class="mb-6">
              <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-3">信号质量</h3>
              <div class="grid grid-cols-3 gap-3">
                <div class="bg-gray-50 dark:bg-gray-800/50 p-4 rounded-xl border border-gray-200/50 dark:border-gray-700/50 text-center">
                  <div class={cn('text-2xl font-bold tabular-nums', signalColor($selectedNetwork.signal))}>
                    {$selectedNetwork.signal}
                  </div>
                  <div class="text-xs text-gray-400 dark:text-gray-500 mt-1">dBm · {signalQuality($selectedNetwork.signal)}</div>
                </div>
                <div class="bg-gray-50 dark:bg-gray-800/50 p-4 rounded-xl border border-gray-200/50 dark:border-gray-700/50 text-center">
                  <div class="text-2xl font-bold text-gray-900 dark:text-white tabular-nums">
                    {$selectedNetwork.snr}
                  </div>
                  <div class="text-xs text-gray-400 dark:text-gray-500 mt-1">SNR (dB)</div>
                </div>
                <div class="bg-gray-50 dark:bg-gray-800/50 p-4 rounded-xl border border-gray-200/50 dark:border-gray-700/50 text-center">
                  <div class="text-2xl font-bold text-gray-900 dark:text-white tabular-nums">
                    {$selectedNetwork.noise}
                  </div>
                  <div class="text-xs text-gray-400 dark:text-gray-500 mt-1">Noise (dBm)</div>
                </div>
              </div>
            </div>

            <!-- BSS Load -->
            {#if $selectedNetwork.bssLoad}
              <div class="mb-6">
                <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-3">BSS 负载 (802.11k)</h3>
                <div class="grid grid-cols-3 gap-3">
                  <div class="bg-gray-50 dark:bg-gray-800/50 p-4 rounded-xl border border-gray-200/50 dark:border-gray-700/50 text-center">
                    <div class={cn('text-2xl font-bold tabular-nums', ($selectedNetwork.bssLoad.channelUtilization / 255) > 0.6 ? 'text-red-500' : 'text-green-500')}>
                      {(($selectedNetwork.bssLoad.channelUtilization / 255) * 100).toFixed(0)}%
                    </div>
                    <div class="text-xs text-gray-400 dark:text-gray-500 mt-1">信道利用率</div>
                  </div>
                  <div class="bg-gray-50 dark:bg-gray-800/50 p-4 rounded-xl border border-gray-200/50 dark:border-gray-700/50 text-center">
                    <div class="text-2xl font-bold text-blue-500 tabular-nums">
                      {$selectedNetwork.bssLoad.stationCount}
                    </div>
                    <div class="text-xs text-gray-400 dark:text-gray-500 mt-1">连接设备</div>
                  </div>
                  <div class="bg-gray-50 dark:bg-gray-800/50 p-4 rounded-xl border border-gray-200/50 dark:border-gray-700/50 text-center">
                    <div class="text-2xl font-bold text-gray-900 dark:text-white tabular-nums">
                      {$selectedNetwork.bssLoad.availableCapacity}
                    </div>
                    <div class="text-xs text-gray-400 dark:text-gray-500 mt-1">可用容量</div>
                  </div>
                </div>
              </div>
            {/if}

            <!-- Network Info -->
            <div class="mb-6">
              <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-3">网络信息</h3>
              <div class="bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200/50 dark:border-gray-700/50 divide-y divide-gray-200/50 dark:divide-gray-700/50">
                <div class="flex justify-between px-4 py-2.5 text-sm">
                  <span class="text-gray-500 dark:text-gray-400">频段</span>
                  <span class="font-medium">{$selectedNetwork.band}GHz · CH {$selectedNetwork.channel}</span>
                </div>
                <div class="flex justify-between px-4 py-2.5 text-sm">
                  <span class="text-gray-500 dark:text-gray-400">信道宽度</span>
                  <span class="font-medium">{$selectedNetwork.channelWidth} MHz</span>
                </div>
                <div class="flex justify-between px-4 py-2.5 text-sm">
                  <span class="text-gray-500 dark:text-gray-400">频率</span>
                  <span class="font-medium tabular-nums">{$selectedNetwork.frequency} MHz</span>
                </div>
                <div class="flex justify-between px-4 py-2.5 text-sm">
                  <span class="text-gray-500 dark:text-gray-400">信标间隔</span>
                  <span class="font-medium tabular-nums">{$selectedNetwork.beaconInterval} ms</span>
                </div>
                <div class="flex justify-between px-4 py-2.5 text-sm">
                  <span class="text-gray-500 dark:text-gray-400">国家</span>
                  <span class="font-medium">{$selectedNetwork.countryCode || 'N/A'}</span>
                </div>
                <div class="flex justify-between px-4 py-2.5 text-sm">
                  <span class="text-gray-500 dark:text-gray-400">WPS</span>
                  <span class={cn('font-medium', $selectedNetwork.wpsEnabled ? 'text-orange-500' : '')}>{$selectedNetwork.wpsEnabled ? '开启' : '关闭'}</span>
                </div>
              </div>
            </div>

            <!-- WiFi Standards -->
            <div class="mb-6">
              <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-3">WiFi 标准</h3>
              <div class="flex flex-wrap gap-2 mb-3">
                {#each $selectedNetwork.standards as std}
                  <span class="px-3 py-1.5 rounded-lg text-xs font-semibold bg-gradient-to-r from-indigo-500 to-purple-600 text-white shadow-sm">
                    {std === 'be' ? 'WiFi 7' : std === 'ax' ? 'WiFi 6' : std === 'ac' ? 'WiFi 5' : std === 'n' ? 'WiFi 4' : std.toUpperCase()}
                  </span>
                {/each}
              </div>
              <div class="bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200/50 dark:border-gray-700/50 divide-y divide-gray-200/50 dark:divide-gray-700/50">
                <div class="flex justify-between px-4 py-2.5 text-sm">
                  <span class="text-gray-500 dark:text-gray-400">空间流</span>
                  <span class="font-medium">{$selectedNetwork.features?.spatialStreams ?? 1} × {$selectedNetwork.features?.spatialStreams ?? 1}</span>
                </div>
                <div class="flex justify-between px-4 py-2.5 text-sm">
                  <span class="text-gray-500 dark:text-gray-400">最大速率</span>
                  <span class="font-medium tabular-nums">{$selectedNetwork.features?.maxDataRate ?? 0} Mbps</span>
                </div>
                <div class="flex justify-between px-4 py-2.5 text-sm">
                  <span class="text-gray-500 dark:text-gray-400">保护间隔</span>
                  <span class="font-medium">
                    {#if $selectedNetwork.features?.guardInterval}
                      {$selectedNetwork.features.guardInterval < 1000
                        ? `0.${$selectedNetwork.features.guardInterval / 100} µs`
                        : `${$selectedNetwork.features.guardInterval / 1000} µs`}
                    {:else}
                      0.8 µs
                    {/if}
                  </span>
                </div>
                {#if $selectedNetwork.features?.mcsIndex}
                  <div class="flex justify-between px-4 py-2.5 text-sm">
                    <span class="text-gray-500 dark:text-gray-400">MCS Index</span>
                    <span class="font-medium tabular-nums">{$selectedNetwork.features.mcsIndex}</span>
                  </div>
                {/if}
              </div>
            </div>

            <!-- Performance Features -->
            {#if $selectedNetwork.features}
              <div class="mb-6">
                <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-3">性能特性</h3>
                <div class="grid grid-cols-2 gap-2">
                  <div class="flex items-center justify-between px-4 py-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200/50 dark:border-gray-700/50">
                    <span class="text-sm">MU-MIMO</span>
                    <span class={cn('w-5 h-5 rounded-full flex items-center justify-center text-xs font-bold', $selectedNetwork.features.muMimo ? 'bg-green-500 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-400')}>
                      {$selectedNetwork.features.muMimo ? '✓' : '✗'}
                    </span>
                  </div>
                  <div class="flex items-center justify-between px-4 py-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200/50 dark:border-gray-700/50">
                    <span class="text-sm">OFDMA</span>
                    <span class={cn('w-5 h-5 rounded-full flex items-center justify-center text-xs font-bold', $selectedNetwork.features.ofdma ? 'bg-green-500 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-400')}>
                      {$selectedNetwork.features.ofdma ? '✓' : '✗'}
                    </span>
                  </div>
                  <div class="flex items-center justify-between px-4 py-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200/50 dark:border-gray-700/50">
                    <span class="text-sm">BSS Coloring</span>
                    <span class={cn('w-5 h-5 rounded-full flex items-center justify-center text-xs font-bold', $selectedNetwork.features.bssColoring ? 'bg-green-500 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-400')}>
                      {$selectedNetwork.features.bssColoring ? '✓' : '✗'}
                    </span>
                  </div>
                  <div class="flex items-center justify-between px-4 py-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200/50 dark:border-gray-700/50">
                    <span class="text-sm">TXBF</span>
                    <span class={cn('w-5 h-5 rounded-full flex items-center justify-center text-xs font-bold', $selectedNetwork.features.txBeamforming ? 'bg-green-500 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-400')}>
                      {$selectedNetwork.features.txBeamforming ? '✓' : '✗'}
                    </span>
                  </div>
                </div>
              </div>
            {/if}

            <!-- Protocol Extensions -->
            {#if $selectedNetwork.protocols}
              <div class="mb-6">
                <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-3">协议扩展</h3>
                <div class="grid grid-cols-2 gap-2">
                  <div class="flex items-center justify-between px-4 py-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200/50 dark:border-gray-700/50">
                    <span class="text-sm">802.11k (RRM)</span>
                    <span class={cn('w-5 h-5 rounded-full flex items-center justify-center text-xs font-bold', $selectedNetwork.protocols.rrm ? 'bg-green-500 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-400')}>
                      {$selectedNetwork.protocols.rrm ? '✓' : '✗'}
                    </span>
                  </div>
                  <div class="flex items-center justify-between px-4 py-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200/50 dark:border-gray-700/50">
                    <span class="text-sm">802.11r (FT)</span>
                    <span class={cn('w-5 h-5 rounded-full flex items-center justify-center text-xs font-bold', $selectedNetwork.protocols.ft ? 'bg-green-500 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-400')}>
                      {$selectedNetwork.protocols.ft ? '✓' : '✗'}
                    </span>
                  </div>
                  <div class="flex items-center justify-between px-4 py-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200/50 dark:border-gray-700/50">
                    <span class="text-sm">802.11v (BSS)</span>
                    <span class={cn('w-5 h-5 rounded-full flex items-center justify-center text-xs font-bold', $selectedNetwork.protocols.bssTransition ? 'bg-green-500 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-400')}>
                      {$selectedNetwork.protocols.bssTransition ? '✓' : '✗'}
                    </span>
                  </div>
                  <div class="flex items-center justify-between px-4 py-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200/50 dark:border-gray-700/50">
                    <span class="text-sm">802.11w (PMF)</span>
                    <span class={cn('w-5 h-5 rounded-full flex items-center justify-center text-xs font-bold', $selectedNetwork.protocols.pmf ? 'bg-green-500 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-400')}>
                      {$selectedNetwork.protocols.pmf ? '✓' : '✗'}
                    </span>
                  </div>
                </div>
              </div>
            {/if}

            <!-- Security -->
            {#if $selectedNetwork.securityDetails}
              <div class="mb-6">
                <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-3">安全信息</h3>
                <div class="bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200/50 dark:border-gray-700/50 divide-y divide-gray-200/50 dark:divide-gray-700/50">
                  <div class="flex justify-between px-4 py-2.5 text-sm">
                    <span class="text-gray-500 dark:text-gray-400">安全类型</span>
                    <span class="font-medium text-green-600 dark:text-green-400">{$selectedNetwork.security.toUpperCase()}</span>
                  </div>
                  <div class="flex justify-between px-4 py-2.5 text-sm">
                    <span class="text-gray-500 dark:text-gray-400">认证方式</span>
                    <span class="font-medium">{$selectedNetwork.securityDetails.authMethod}</span>
                  </div>
                  <div class="flex justify-between px-4 py-2.5 text-sm">
                    <span class="text-gray-500 dark:text-gray-400">加密</span>
                    <span class="font-medium">{$selectedNetwork.securityDetails.cipher}</span>
                  </div>
                  <div class="flex justify-between px-4 py-2.5 text-sm">
                    <span class="text-gray-500 dark:text-gray-400">PMF</span>
                    <span class="font-medium">
                      {$selectedNetwork.securityDetails.pmfCapable ? '支持' : '不支持'}
                      {#if $selectedNetwork.securityDetails.pmfRequired}
                        <span class="text-blue-500">(必需)</span>
                      {/if}
                    </span>
                  </div>
                </div>
              </div>
            {/if}

            <!-- IE Details Button -->
            <div class="mt-6">
              <Button
                onclick={() => showIEDetails = true}
                variant="outline"
                class="w-full native-button"
              >
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                </svg>
                查看 Beacon Frame (IE 解析)
              </Button>
            </div>
          </div>
        {:else}
          <div class="flex-1 flex items-center justify-center text-gray-400 dark:text-gray-500 bg-gray-50/50 dark:bg-gray-800/30">
            <div class="text-center animate-fade-in">
              <div class="w-16 h-16 mx-auto mb-4 rounded-2xl bg-gray-100 dark:bg-gray-800 flex items-center justify-center">
                <svg class="w-8 h-8 text-gray-300 dark:text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
                </svg>
              </div>
              <div class="text-sm font-medium">点击左侧网络查看详情</div>
              <div class="text-xs text-gray-300 dark:text-gray-600 mt-1">选择一个网络以查看完整信息</div>
            </div>
          </div>
        {/if}
      </div>

    {:else if activeTab === 'channels'}
      <!-- Channel Analysis View -->
      <div class="flex-1 overflow-y-auto bg-gray-50/30 dark:bg-gray-800/30">
        <ChannelView />
      </div>

    {:else if activeTab === 'groups'}
      <!-- Network Groups View -->
      <div class="flex-1 overflow-y-auto p-5 bg-gray-50/30 dark:bg-gray-800/30">
        {#if $networkGroups.length === 0}
          <div class="text-center text-gray-400 dark:text-gray-500 py-12 animate-fade-in">
            <div class="w-16 h-16 mx-auto mb-4 rounded-2xl bg-gray-100 dark:bg-gray-800 flex items-center justify-center">
              <svg class="w-8 h-8 text-gray-300 dark:text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
              </svg>
            </div>
            <div class="text-sm font-medium">无网络分组</div>
            <div class="text-xs text-gray-300 dark:text-gray-600 mt-1">扫描后将自动分组相同SSID的网络</div>
          </div>
        {:else}
          <div class="space-y-3 max-w-3xl">
            {#each $networkGroups as group (group.ssid)}
              <div class="bg-white dark:bg-gray-800/80 rounded-xl border border-gray-200/50 dark:border-gray-700/50 p-4 shadow-sm hover:shadow-md transition-shadow">
                <div class="flex items-center justify-between mb-3">
                  <div>
                    <h3 class="font-bold text-gray-900 dark:text-white">{group.ssid}</h3>
                    <div class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                      {group.totalAps} 个 AP · {group.bands.join(', ')}GHz · 信号: {group.bestSignal} dBm
                    </div>
                  </div>
                  <div class="flex gap-1.5">
                    {#if group.supportsFastRoaming}
                      <span class="px-2 py-1 text-xs bg-green-100 dark:bg-green-900/50 text-green-700 dark:text-green-300 rounded-md font-medium">802.11r</span>
                    {/if}
                    {#if group.supportsBssTransition}
                      <span class="px-2 py-1 text-xs bg-blue-100 dark:bg-blue-900/50 text-blue-700 dark:text-blue-300 rounded-md font-medium">802.11v</span>
                    {/if}
                  </div>
                </div>
                <div class="space-y-1.5">
                  {#each group.networks as net (net.bssid)}
                    <div class="flex items-center justify-between text-xs p-2.5 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                      <div class="flex items-center gap-2">
                        <span class={cn('w-2 h-2 rounded-full', net.signal >= -50 ? 'bg-green-500' : net.signal >= -70 ? 'bg-yellow-500' : 'bg-red-500')}></span>
                        <span class="font-mono">{net.bssid.toUpperCase()}</span>
                      </div>
                      <div class="flex items-center gap-3 text-gray-500 dark:text-gray-400">
                        <span>CH {net.channel}</span>
                        <span>{net.band}GHz</span>
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
    {/if}
  </div>

  <!-- IE Details Modal -->
  {#if showIEDetails && $selectedNetwork}
    <IEDetailsPanel
      network={$selectedNetwork}
      onClose={() => showIEDetails = false}
    />
  {/if}
</div>
