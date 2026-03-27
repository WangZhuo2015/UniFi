<script lang="ts">
  import { cn, signalColor, securityIcon, securityColor, bandColor } from '$lib/utils';
  import { selectedBssid } from '$lib/stores';
  import type { Network } from '$lib/types';

  interface Props {
    network: Network;
  }

  let { network }: Props = $props();

  function select() {
    $selectedBssid = network.bssid;
  }

  // WiFi generation
  const wifiGen = $derived.by(() => {
    if (network.standards?.includes('be')) return { name: 'WiFi 7', color: 'bg-gradient-to-r from-amber-400 to-orange-500 text-white' };
    if (network.standards?.includes('ax')) return { name: 'WiFi 6', color: 'bg-gradient-to-r from-indigo-500 to-purple-600 text-white' };
    if (network.standards?.includes('ac')) return { name: 'WiFi 5', color: 'bg-gradient-to-r from-blue-400 to-cyan-500 text-white' };
    if (network.standards?.includes('n')) return { name: 'WiFi 4', color: 'bg-gray-500 text-white' };
    return null;
  });

  // QAM label
  const qamLabel = $derived.by(() => {
    const qam = network.features?.maxQam ?? 256;
    if (qam >= 4096) return { name: '4K-QAM', color: 'bg-gradient-to-r from-fuchsia-500 to-pink-500 text-white' };
    if (qam >= 1024) return { name: '1K-QAM', color: 'bg-purple-500 text-white' };
    return null;
  });

  // Signal quality percentage
  const signalQuality = $derived(Math.round(Math.min(100, Math.max(0, (network.signal + 90) * (100 / 60)))));
</script>

<button
  data-testid="network-card"
  data-bssid={network.bssid}
  data-ssid={network.ssid ?? ''}
  class={cn(
    'w-full p-3 rounded-xl transition-all duration-200 native-button',
    'bg-white dark:bg-gray-800/80 text-gray-900 dark:text-gray-100',
    'border border-gray-200/50 dark:border-gray-700/50',
    'hover:shadow-md hover:border-blue-300 dark:hover:border-blue-600',
    $selectedBssid === network.bssid
      ? 'ring-2 ring-blue-500/50 border-blue-500/50 shadow-md'
      : ''
  )}
  onclick={select}
>
  <!-- Header Row: SSID + Signal -->
  <div class="flex items-center justify-between gap-3">
    <div class="flex items-center gap-2 min-w-0">
      <span class="text-base shrink-0">{securityIcon(network.security)}</span>
      <span data-testid="ssid" class="font-semibold truncate text-sm">
        {network.ssid ?? '[隐藏网络]'}
      </span>
      {#if network.wpsEnabled}
        <span class="text-[10px] px-1.5 py-0.5 bg-orange-100 dark:bg-orange-900/50 text-orange-600 dark:text-orange-300 rounded-md font-medium shrink-0">WPS</span>
      {/if}
    </div>
    <div class="flex items-center gap-2 shrink-0">
      <div class="flex items-center gap-0.5">
        {#each Array(4) as _, i}
          <div class={cn(
            'w-1 rounded-sm transition-colors duration-200',
            i < Math.round(signalQuality / 25) ? 'bg-green-500' : 'bg-gray-200 dark:bg-gray-700',
            i === 0 ? 'h-1' : i === 1 ? 'h-1.5' : i === 2 ? 'h-2' : 'h-2.5'
          )}></div>
        {/each}
      </div>
      <span class={cn('font-mono font-bold text-sm tabular-nums', signalColor(network.signal))}>
        {network.signal}
      </span>
    </div>
  </div>

  <!-- Info Row -->
  <div class="mt-1 text-[11px] text-gray-400 dark:text-gray-500 flex items-center gap-1.5 flex-wrap">
    <span class="font-mono">{network.bssid.toUpperCase()}</span>
    <span class="text-gray-300 dark:text-gray-600">·</span>
    <span>CH {network.channel}</span>
    <span class="text-gray-300 dark:text-gray-600">·</span>
    <span>{network.channelWidth}MHz</span>
    {#if network.vendor}
      <span class="text-gray-300 dark:text-gray-600">·</span>
      <span class="text-gray-500 dark:text-gray-400">{network.vendor}</span>
    {/if}
  </div>

  <!-- Key Feature Labels -->
  <div class="mt-2 flex flex-wrap gap-1">
    <!-- Band -->
    <span class={cn('text-[10px] px-2 py-0.5 rounded-md font-semibold', bandColor(network.band))}>
      {network.band}GHz
    </span>
    <!-- WiFi Generation -->
    {#if wifiGen}
      <span class={cn('text-[10px] px-2 py-0.5 rounded-md font-semibold shadow-sm', wifiGen.color)}>
        {wifiGen.name}
      </span>
    {/if}
    <!-- Security -->
    <span class={cn('text-[10px] px-2 py-0.5 rounded-md font-semibold', securityColor(network.security))}>
      {network.security.toUpperCase()}
    </span>
    <!-- MLO -->
    {#if network.features?.mlo}
      <span class="text-[10px] px-2 py-0.5 rounded-md font-semibold bg-gradient-to-r from-rose-500 to-red-500 text-white shadow-sm">
        MLO
      </span>
    {/if}
    <!-- QAM -->
    {#if qamLabel}
      <span class={cn('text-[10px] px-2 py-0.5 rounded-md font-semibold shadow-sm', qamLabel.color)}>
        {qamLabel.name}
      </span>
    {/if}
  </div>

  <!-- Protocol Extensions -->
  {#if network.protocols?.rrm || network.protocols?.ft || network.protocols?.bssTransition || network.protocols?.pmf}
    <div class="mt-1.5 flex flex-wrap gap-1">
      {#if network.protocols?.rrm}
        <span class="text-[9px] px-1.5 py-0.5 rounded bg-violet-50 dark:bg-violet-900/30 text-violet-600 dark:text-violet-300 font-medium">
          802.11k
        </span>
      {/if}
      {#if network.protocols?.ft}
        <span class="text-[9px] px-1.5 py-0.5 rounded bg-pink-50 dark:bg-pink-900/30 text-pink-600 dark:text-pink-300 font-medium">
          802.11r
        </span>
      {/if}
      {#if network.protocols?.bssTransition}
        <span class="text-[9px] px-1.5 py-0.5 rounded bg-sky-50 dark:bg-sky-900/30 text-sky-600 dark:text-sky-300 font-medium">
          802.11v
        </span>
      {/if}
      {#if network.protocols?.pmf}
        <span class="text-[9px] px-1.5 py-0.5 rounded bg-teal-50 dark:bg-teal-900/30 text-teal-600 dark:text-teal-300 font-medium">
          802.11w
        </span>
      {/if}
    </div>
  {/if}

  <!-- BSS Load -->
  {#if network.bssLoad}
    <div class="mt-2 text-[11px] flex items-center gap-3 text-gray-500 dark:text-gray-400 bg-gray-50 dark:bg-gray-900/50 rounded-lg px-2.5 py-1.5">
      <span class="flex items-center gap-1">
        <span class="w-1.5 h-1.5 rounded-full bg-blue-500"></span>
        利用率: {((network.bssLoad.channelUtilization / 255) * 100).toFixed(0)}%
      </span>
      <span class="flex items-center gap-1">
        <span class="w-1.5 h-1.5 rounded-full bg-green-500"></span>
        设备: {network.bssLoad.stationCount}
      </span>
    </div>
  {/if}

  <!-- Performance Stats -->
  <div class="mt-2 pt-2 border-t border-gray-100 dark:border-gray-700/50 text-[10px] text-gray-400 dark:text-gray-500 flex items-center justify-between">
    <div class="flex gap-3">
      <span class="flex items-center gap-1">
        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
        </svg>
        <span class="tabular-nums">{network.features?.maxDataRate ?? 0}</span> Mbps
      </span>
      <span>{network.features?.spatialStreams ?? 1} 流</span>
      {#if network.snr > 0}
        <span>SNR: <span class="tabular-nums">{network.snr}</span></span>
      {/if}
    </div>
    <div class="flex gap-1">
      {#if network.features?.muMimo}
        <span class="px-1 py-0.5 rounded bg-gray-100 dark:bg-gray-700/50 text-gray-500 dark:text-gray-400">MU-MIMO</span>
      {/if}
      {#if network.features?.ofdma}
        <span class="px-1 py-0.5 rounded bg-gray-100 dark:bg-gray-700/50 text-gray-500 dark:text-gray-400">OFDMA</span>
      {/if}
    </div>
  </div>
</button>
