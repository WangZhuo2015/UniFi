<script lang="ts">
  import { onMount } from 'svelte';
  import { networks, scan, isScanning, error, byBand, selectedNetwork, selectedBssid } from '$lib/stores';
  import { cn, signalColor } from '$lib/utils';
  import NetworkCard from '$lib/components/NetworkCard.svelte';
  import IEDetailsPanel from '$lib/components/IEDetailsPanel.svelte';
  import Button from '$lib/components/ui/button.svelte';

  let filterText = $state('');
  let activeBand = $state<'all' | '2.4' | '5' | '6'>('all');
  let showIEDetails = $state(false);

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

  onMount(() => {
    scan();
  });
</script>

<div class="h-screen flex flex-col bg-gray-50 dark:bg-gray-900 text-gray-900 dark:text-gray-100">
  <!-- Header -->
  <header class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-4 py-3 shrink-0">
    <div class="flex items-center justify-between">
      <h1 class="text-xl font-bold text-gray-900 dark:text-white">WiFi Tool</h1>
      <Button onclick={() => scan()} disabled={$isScanning}>
        {$isScanning ? '扫描中...' : '扫描'}
      </Button>
    </div>
  </header>

  <!-- Main Content -->
  <div class="flex-1 flex overflow-hidden">
    {#if $error}
      <div class="absolute top-16 left-1/2 -translate-x-1/2 z-50 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 px-4 py-3 rounded shadow-lg">
        {$error}
      </div>
    {/if}

    <!-- Left Panel: Network List -->
    <div class="w-1/2 border-r border-gray-200 dark:border-gray-700 flex flex-col bg-white dark:bg-gray-800">
      <!-- Filters -->
      <div class="p-3 border-b border-gray-200 dark:border-gray-700 shrink-0">
        <div class="flex gap-2">
          <input
            type="search"
            placeholder="搜索网络..."
            bind:value={filterText}
            class="flex-1 px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 placeholder-gray-400"
          />
          <div class="flex rounded-lg overflow-hidden border border-gray-300 dark:border-gray-600">
            {#each ['all', '2.4', '5', '6'] as band}
              <button
                class="px-3 py-2 text-xs text-gray-700 dark:text-gray-200 {activeBand === band
                  ? 'bg-blue-500 text-white dark:text-white'
                  : 'bg-white dark:bg-gray-700 hover:bg-gray-50 dark:hover:bg-gray-600'}"
                onclick={() => activeBand = band as typeof activeBand}
              >
                {band === 'all' ? '全部' : '{band}G'}
              </button>
            {/each}
          </div>
        </div>
        <div class="mt-2 text-xs text-gray-500 dark:text-gray-400">
          共 {$networks.length} 个网络
        </div>
      </div>

      <!-- Network List -->
      <div class="flex-1 overflow-y-auto p-2 space-y-1">
        {#if $isScanning && $networks.length === 0}
          <div class="text-center text-gray-500 dark:text-gray-400 py-8">
            扫描中...
          </div>
        {:else if filtered.length === 0}
          <div class="text-center text-gray-500 dark:text-gray-400 py-8">
            未发现网络
          </div>
        {:else}
          {#each filtered as network (network.bssid)}
            <NetworkCard {network} />
          {/each}
        {/if}
      </div>
    </div>

    <!-- Right Panel: Network Details -->
    <div class="w-1/2 flex flex-col bg-gray-50 dark:bg-gray-900 overflow-y-auto">
      {#if $selectedNetwork}
        <div class="p-4">
          <!-- Title -->
          <div class="mb-4 pb-3 border-b border-gray-200 dark:border-gray-700">
            <h2 class="text-xl font-bold text-gray-900 dark:text-white">
              {$selectedNetwork.ssid ?? '[隐藏网络]'}
            </h2>
            <div class="mt-1 text-sm text-gray-500 dark:text-gray-400">
              {$selectedNetwork.bssid.toUpperCase()} · {$selectedNetwork.vendor || 'Unknown'}
            </div>
          </div>

          <!-- Signal Section -->
          <div class="mb-6">
            <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">信号质量</h3>
            <div class="grid grid-cols-3 gap-3">
              <div class="bg-white dark:bg-gray-800 p-3 rounded-lg border border-gray-200 dark:border-gray-700 text-center">
                <div class={cn('text-2xl font-bold', signalColor($selectedNetwork.signal))}>
                  {$selectedNetwork.signal}
                </div>
                <div class="text-xs text-gray-500 dark:text-gray-400">dBm</div>
              </div>
              <div class="bg-white dark:bg-gray-800 p-3 rounded-lg border border-gray-200 dark:border-gray-700 text-center">
                <div class="text-2xl font-bold text-gray-900 dark:text-white">
                  {$selectedNetwork.snr}
                </div>
                <div class="text-xs text-gray-500 dark:text-gray-400">SNR (dB)</div>
              </div>
              <div class="bg-white dark:bg-gray-800 p-3 rounded-lg border border-gray-200 dark:border-gray-700 text-center">
                <div class="text-2xl font-bold text-gray-900 dark:text-white">
                  {$selectedNetwork.noise}
                </div>
                <div class="text-xs text-gray-500 dark:text-gray-400">Noise (dBm)</div>
              </div>
            </div>
          </div>

          <!-- BSS Load -->
          {#if $selectedNetwork.bssLoad}
            <div class="mb-6">
              <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">BSS 负载 (802.11k)</h3>
              <div class="grid grid-cols-3 gap-3">
                <div class="bg-white dark:bg-gray-800 p-3 rounded-lg border border-gray-200 dark:border-gray-700 text-center">
                  <div class={cn('text-2xl font-bold', ($selectedNetwork.bssLoad.channelUtilization / 255) > 0.6 ? 'text-red-500' : 'text-green-500')}>
                    {(($selectedNetwork.bssLoad.channelUtilization / 255) * 100).toFixed(0)}%
                  </div>
                  <div class="text-xs text-gray-500 dark:text-gray-400">信道利用率</div>
                </div>
                <div class="bg-white dark:bg-gray-800 p-3 rounded-lg border border-gray-200 dark:border-gray-700 text-center">
                  <div class="text-2xl font-bold text-blue-500">
                    {$selectedNetwork.bssLoad.stationCount}
                  </div>
                  <div class="text-xs text-gray-500 dark:text-gray-400">连接设备</div>
                </div>
                <div class="bg-white dark:bg-gray-800 p-3 rounded-lg border border-gray-200 dark:border-gray-700 text-center">
                  <div class="text-2xl font-bold text-gray-900 dark:text-white">
                    {$selectedNetwork.bssLoad.availableCapacity}
                  </div>
                  <div class="text-xs text-gray-500 dark:text-gray-400">可用容量</div>
                </div>
              </div>
            </div>
          {/if}

          <!-- Network Info -->
          <div class="mb-6">
            <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">网络信息</h3>
            <div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 divide-y divide-gray-100 dark:divide-gray-700">
              <div class="flex justify-between px-3 py-2 text-sm">
                <span class="text-gray-500 dark:text-gray-400">频段</span>
                <span class="font-medium">{$selectedNetwork.band}GHz · CH {$selectedNetwork.channel}</span>
              </div>
              <div class="flex justify-between px-3 py-2 text-sm">
                <span class="text-gray-500 dark:text-gray-400">信道宽度</span>
                <span class="font-medium">{$selectedNetwork.channelWidth} MHz</span>
              </div>
              <div class="flex justify-between px-3 py-2 text-sm">
                <span class="text-gray-500 dark:text-gray-400">频率</span>
                <span class="font-medium">{$selectedNetwork.frequency} MHz</span>
              </div>
              <div class="flex justify-between px-3 py-2 text-sm">
                <span class="text-gray-500 dark:text-gray-400">信标间隔</span>
                <span class="font-medium">{$selectedNetwork.beaconInterval} ms</span>
              </div>
              <div class="flex justify-between px-3 py-2 text-sm">
                <span class="text-gray-500 dark:text-gray-400">国家</span>
                <span class="font-medium">{$selectedNetwork.countryCode || 'N/A'}</span>
              </div>
              <div class="flex justify-between px-3 py-2 text-sm">
                <span class="text-gray-500 dark:text-gray-400">WPS</span>
                <span class="font-medium">{$selectedNetwork.wpsEnabled ? '开启' : '关闭'}</span>
              </div>
            </div>
          </div>

          <!-- WiFi Standards -->
          <div class="mb-6">
            <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">WiFi 标准</h3>
            <div class="flex flex-wrap gap-2">
              {#each $selectedNetwork.standards as std}
                <span class="px-3 py-1 rounded-full text-sm font-medium bg-indigo-100 dark:bg-indigo-900 text-indigo-700 dark:text-indigo-300">
                  {std === 'be' ? 'WiFi 7 (be)' : std === 'ax' ? 'WiFi 6 (ax)' : std === 'ac' ? 'WiFi 5 (ac)' : std === 'n' ? 'WiFi 4 (n)' : std.toUpperCase()}
                </span>
              {/each}
            </div>
            <div class="mt-3 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 divide-y divide-gray-100 dark:divide-gray-700">
              <div class="flex justify-between px-3 py-2 text-sm">
                <span class="text-gray-500 dark:text-gray-400">空间流</span>
                <span class="font-medium">{$selectedNetwork.features?.spatialStreams ?? 1}</span>
              </div>
              <div class="flex justify-between px-3 py-2 text-sm">
                <span class="text-gray-500 dark:text-gray-400">最大速率</span>
                <span class="font-medium">{$selectedNetwork.features?.maxDataRate ?? 0} Mbps</span>
              </div>
            </div>
          </div>

          <!-- Performance Features -->
          {#if $selectedNetwork.features}
            <div class="mb-6">
              <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">性能特性</h3>
              <div class="grid grid-cols-2 gap-2">
                <div class="flex items-center justify-between px-3 py-2 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                  <span class="text-sm">MU-MIMO</span>
                  <span class={cn('font-bold', $selectedNetwork.features.muMimo ? 'text-green-500' : 'text-gray-400')}>
                    {$selectedNetwork.features.muMimo ? '✓' : '✗'}
                  </span>
                </div>
                <div class="flex items-center justify-between px-3 py-2 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                  <span class="text-sm">OFDMA</span>
                  <span class={cn('font-bold', $selectedNetwork.features.ofdma ? 'text-green-500' : 'text-gray-400')}>
                    {$selectedNetwork.features.ofdma ? '✓' : '✗'}
                  </span>
                </div>
                <div class="flex items-center justify-between px-3 py-2 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                  <span class="text-sm">BSS Coloring</span>
                  <span class={cn('font-bold', $selectedNetwork.features.bssColoring ? 'text-green-500' : 'text-gray-400')}>
                    {$selectedNetwork.features.bssColoring ? '✓' : '✗'}
                  </span>
                </div>
                <div class="flex items-center justify-between px-3 py-2 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                  <span class="text-sm">TXBF</span>
                  <span class={cn('font-bold', $selectedNetwork.features.txBeamforming ? 'text-green-500' : 'text-gray-400')}>
                    {$selectedNetwork.features.txBeamforming ? '✓' : '✗'}
                  </span>
                </div>
              </div>
            </div>
          {/if}

          <!-- Protocol Extensions -->
          {#if $selectedNetwork.protocols}
            <div class="mb-6">
              <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">协议扩展</h3>
              <div class="grid grid-cols-2 gap-2">
                <div class="flex items-center justify-between px-3 py-2 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                  <span class="text-sm">802.11k (RRM)</span>
                  <span class={cn('font-bold', $selectedNetwork.protocols.rrm ? 'text-green-500' : 'text-gray-400')}>
                    {$selectedNetwork.protocols.rrm ? '✓' : '✗'}
                  </span>
                </div>
                <div class="flex items-center justify-between px-3 py-2 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                  <span class="text-sm">802.11r (FT)</span>
                  <span class={cn('font-bold', $selectedNetwork.protocols.ft ? 'text-green-500' : 'text-gray-400')}>
                    {$selectedNetwork.protocols.ft ? '✓' : '✗'}
                  </span>
                </div>
                <div class="flex items-center justify-between px-3 py-2 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                  <span class="text-sm">802.11v (BSS)</span>
                  <span class={cn('font-bold', $selectedNetwork.protocols.bssTransition ? 'text-green-500' : 'text-gray-400')}>
                    {$selectedNetwork.protocols.bssTransition ? '✓' : '✗'}
                  </span>
                </div>
                <div class="flex items-center justify-between px-3 py-2 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                  <span class="text-sm">802.11w (PMF)</span>
                  <span class={cn('font-bold', $selectedNetwork.protocols.pmf ? 'text-green-500' : 'text-gray-400')}>
                    {$selectedNetwork.protocols.pmf ? '✓' : '✗'}
                  </span>
                </div>
              </div>
            </div>
          {/if}

          <!-- Security -->
          {#if $selectedNetwork.securityDetails}
            <div class="mb-6">
              <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">安全信息</h3>
              <div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 divide-y divide-gray-100 dark:divide-gray-700">
                <div class="flex justify-between px-3 py-2 text-sm">
                  <span class="text-gray-500 dark:text-gray-400">安全类型</span>
                  <span class="font-medium text-green-600 dark:text-green-400">{$selectedNetwork.security.toUpperCase()}</span>
                </div>
                <div class="flex justify-between px-3 py-2 text-sm">
                  <span class="text-gray-500 dark:text-gray-400">认证方式</span>
                  <span class="font-medium">{$selectedNetwork.securityDetails.authMethod}</span>
                </div>
                <div class="flex justify-between px-3 py-2 text-sm">
                  <span class="text-gray-500 dark:text-gray-400">加密</span>
                  <span class="font-medium">{$selectedNetwork.securityDetails.cipher}</span>
                </div>
                <div class="flex justify-between px-3 py-2 text-sm">
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
          <div class="mt-4">
            <Button
              onclick={() => showIEDetails = true}
              variant="outline"
              class="w-full"
            >
              <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
              查看 Beacon Frame (IE 解析)
            </Button>
          </div>
        </div>
      {:else}
        <div class="flex-1 flex items-center justify-center text-gray-400 dark:text-gray-500">
          <div class="text-center">
            <div class="text-4xl mb-2">📡</div>
            <div>点击左侧网络查看详情</div>
          </div>
        </div>
      {/if}
    </div>
  </div>

  <!-- IE Details Modal -->
  {#if showIEDetails && $selectedNetwork}
    <IEDetailsPanel
      network={$selectedNetwork}
      onClose={() => showIEDetails = false}
    />
  {/if}
</div>
