<script lang="ts">
  import { locale, t } from '$lib/i18n';
  import { selectedBssid } from '$lib/stores';
  import type { Network } from '$lib/types';
  import { bandColor, cn, formatStandards, securityColor, signalBars, signalColor } from '$lib/utils';

  export let network: Network;

  function selectNetwork() {
    selectedBssid.set(network.bssid);
  }
</script>

<button
  type="button"
  class="w-full text-left rounded-xl border border-gray-200/70 bg-white/90 p-3 shadow-sm transition hover:border-blue-400/70 hover:shadow-md dark:border-gray-700/60 dark:bg-gray-800/80"
  onclick={selectNetwork}
>
  <div class="flex items-start justify-between gap-3">
    <div class="min-w-0 flex-1">
      <div class="flex items-center gap-2">
        <h3 class="truncate text-sm font-semibold text-gray-900 dark:text-gray-100">
          {network.ssid ?? t($locale, 'hiddenNetwork')}
        </h3>
        {#if network.connected}
          <span class="rounded-full bg-green-100 px-2 py-0.5 text-[10px] font-medium text-green-700 dark:bg-green-900/40 dark:text-green-300">
            {t($locale, 'connectedTo')}
          </span>
        {/if}
      </div>
      <div class="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-gray-500 dark:text-gray-400">
        <span class="font-mono">{network.bssid.toUpperCase()}</span>
        <span>CH {network.channel}</span>
        <span>{network.frequency} MHz</span>
        <span>{network.vendor || t($locale, 'unknownVendor')}</span>
      </div>
    </div>
    <div class="text-right">
      <div class={cn('text-lg font-bold tabular-nums', signalColor(network.signal))}>{network.signal}</div>
      <div class="text-[11px] text-gray-500 dark:text-gray-400">dBm</div>
    </div>
  </div>

  <div class="mt-3 flex items-center justify-between gap-3">
    <div class="flex items-center gap-1.5">
      {#each Array.from({ length: 4 }) as _, index}
        <span
          class={cn(
            'block w-1.5 rounded-sm bg-gray-200 dark:bg-gray-700',
            index < signalBars(network.signal) && 'bg-blue-500'
          )}
          style={`height:${8 + index * 3}px`}
        ></span>
      {/each}
    </div>

    <div class="flex flex-wrap items-center justify-end gap-1.5 text-[11px]">
      <span class={cn('rounded-full px-2 py-0.5 font-medium', bandColor(network.band))}>{network.band} GHz</span>
      <span class={cn('rounded-full px-2 py-0.5 font-medium', securityColor(network.security))}>
        {network.security === 'open' ? t($locale, 'openSecurity') : network.security.toUpperCase()}
      </span>
      {#if network.standards.length > 0}
        <span class="rounded-full bg-gray-100 px-2 py-0.5 font-medium text-gray-700 dark:bg-gray-700 dark:text-gray-200">
          {formatStandards(network.standards)}
        </span>
      {/if}
    </div>
  </div>
</button>

