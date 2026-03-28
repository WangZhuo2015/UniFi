<script lang="ts">
  import { onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import Button from './ui/button.svelte';
  import { locale, t } from '$lib/i18n';

  interface PingResult {
    seq: number;
    timestamp_ms: number;
    rtt_us: number | null;
    lost: boolean;
    ttl: number | null;
    source_ip: string | null;
  }

  interface RoamingEvent {
    timestamp_ms: number;
    from_bssid: string | null;
    to_bssid: string | null;
    from_ssid: string | null;
    to_ssid: string | null;
    from_channel: number | null;
    to_channel: number | null;
    roaming_duration_ms: number | null;
    packets_lost: number;
    max_latency_ms: number | null;
  }

  interface LatencySpike {
    timestamp_ms: number;
    latency_ms: number;
    baseline_ms: number;
    spike_ratio: number;
  }

  interface PingStats {
    packets_sent: number;
    packets_received: number;
    packet_loss_percent: number;
    min_rtt_ms: number | null;
    max_rtt_ms: number | null;
    avg_rtt_ms: number | null;
    std_dev_ms: number | null;
    jitter_ms: number | null;
  }

  interface RoamingTestResult {
    duration_secs: number;
    ping_stats: PingStats;
    ping_results: PingResult[];
    roaming_events: RoamingEvent[];
    latency_spikes: LatencySpike[];
    roaming_count: number;
    avg_roaming_duration_ms: number | null;
    total_roaming_packet_loss: number;
  }

  let targetHost = $state('8.8.8.8');
  let durationSecs = $state(60);
  let intervalMs = $state(100);
  let isRunning = $state(false);
  let results = $state<RoamingTestResult | null>(null);
  let currentProgress = $state({ current: 0, total: 0 });
  let error = $state<string | null>(null);
  let latencyData = $state<{ time: number; latency: number; lost: boolean }[]>([]);

  let chartWidth = 600;
  let chartHeight = 200;
  let pollInterval: ReturnType<typeof setInterval> | null = null;

  const targetInputId = 'roaming-target-host';
  const durationInputId = 'roaming-duration-secs';
  const intervalInputId = 'roaming-interval-ms';

  async function startTest() {
    try {
      error = null;
      results = null;
      latencyData = [];

      await invoke('start_roaming_test', { target: targetHost, durationSecs, intervalMs });
      isRunning = true;

      pollInterval = setInterval(async () => {
        try {
          const [running, current, total] = await invoke<[boolean, number, number]>('get_roaming_test_status');
          currentProgress = { current, total };

          const res = await invoke<RoamingTestResult>('get_roaming_test_results');
          results = res;
          latencyData = res.ping_results.slice(-200).map((p) => ({
            time: p.timestamp_ms / 1000,
            latency: p.rtt_us ? p.rtt_us / 1000 : 0,
            lost: p.lost
          }));

          if (!running) {
            isRunning = false;
            if (pollInterval) clearInterval(pollInterval);
          }
        } catch (e) {
          error = String(e);
        }
      }, 500);
    } catch (e) {
      error = String(e);
    }
  }

  async function stopTest() {
    try {
      const res = await invoke<RoamingTestResult>('stop_roaming_test');
      results = res;
      isRunning = false;
      if (pollInterval) clearInterval(pollInterval);
    } catch (e) {
      error = String(e);
    }
  }

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval);
  });

  function getChartPath(): string {
    if (latencyData.length < 2) return '';

    const maxLatency = Math.max(...latencyData.map((d) => d.lost ? 0 : d.latency), 1);
    const maxTime = Math.max(...latencyData.map((d) => d.time), 1);

    return latencyData
      .filter((d) => !d.lost && d.latency > 0)
      .map((d, index) => {
        const x = (d.time / maxTime) * chartWidth;
        const y = chartHeight - (d.latency / maxLatency) * chartHeight * 0.9;
        return `${index === 0 ? 'M' : 'L'} ${x} ${y}`;
      })
      .join(' ');
  }

  function formatLatency(us: number | null): string {
    if (us === null) return '-';
    if (us < 1000) return `${us} us`;
    return `${(us / 1000).toFixed(1)} ms`;
  }

  function formatDuration(ms: number | null): string {
    if (ms === null) return '-';
    if (ms < 1000) return `${ms} ms`;
    return `${(ms / 1000).toFixed(2)} s`;
  }
</script>

<div class="space-y-4 p-4">
  <div class="flex items-center justify-between">
    <h2 class="text-lg font-bold text-gray-900 dark:text-white">{t($locale, 'roamingTest')}</h2>
    {#if isRunning}
      <Button variant="destructive" onclick={stopTest}>{t($locale, 'stopTest')}</Button>
    {:else}
      <Button onclick={startTest}>{t($locale, 'startTest')}</Button>
    {/if}
  </div>

  <div class="rounded-xl border border-gray-200/60 bg-gray-50 p-4 dark:border-gray-700/60 dark:bg-gray-800/50">
    <h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'testConfiguration')}</h3>
    <div class="grid grid-cols-1 gap-4 md:grid-cols-3">
      <div>
        <label for={targetInputId} class="mb-1 block text-xs text-gray-500 dark:text-gray-400">{t($locale, 'targetHost')}</label>
        <input id={targetInputId} bind:value={targetHost} disabled={isRunning} class="w-full rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm dark:border-gray-600 dark:bg-gray-700" />
      </div>
      <div>
        <label for={durationInputId} class="mb-1 block text-xs text-gray-500 dark:text-gray-400">{t($locale, 'durationSeconds')}</label>
        <input id={durationInputId} type="number" min="10" max="600" bind:value={durationSecs} disabled={isRunning} class="w-full rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm dark:border-gray-600 dark:bg-gray-700" />
      </div>
      <div>
        <label for={intervalInputId} class="mb-1 block text-xs text-gray-500 dark:text-gray-400">{t($locale, 'pingIntervalMs')}</label>
        <input id={intervalInputId} type="number" min="50" max="1000" bind:value={intervalMs} disabled={isRunning} class="w-full rounded-lg border border-gray-200 bg-white px-3 py-2 text-sm dark:border-gray-600 dark:bg-gray-700" />
      </div>
    </div>
  </div>

  {#if error}
    <div class="rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-700 dark:border-red-800 dark:bg-red-900/20 dark:text-red-300">{error}</div>
  {/if}

  {#if isRunning}
    <div class="rounded-lg border border-blue-200 bg-blue-50 p-3 dark:border-blue-800 dark:bg-blue-900/20">
      <div class="mb-2 flex items-center justify-between">
        <span class="text-sm font-medium text-blue-700 dark:text-blue-300">{t($locale, 'testInProgress')}</span>
        <span class="text-xs text-blue-600 dark:text-blue-400">{t($locale, 'pingsProgress', currentProgress)}</span>
      </div>
      <div class="h-2 overflow-hidden rounded-full bg-blue-200 dark:bg-blue-800">
        <div class="h-full bg-blue-500 transition-all duration-300" style="width: {currentProgress.total > 0 ? currentProgress.current / currentProgress.total * 100 : 0}%"></div>
      </div>
    </div>
  {/if}

  {#if latencyData.length > 0}
    <div class="rounded-xl border border-gray-200/60 bg-white p-4 dark:border-gray-700/60 dark:bg-gray-800/50">
      <h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'realtimeLatencyChart')}</h3>
      <svg width="100%" height={chartHeight} class="overflow-visible">
        {#each [0, 25, 50, 75, 100] as pct}
          <line x1="0" y1={chartHeight * (1 - pct / 100)} x2="100%" y2={chartHeight * (1 - pct / 100)} stroke="currentColor" class="text-gray-200 dark:text-gray-700" />
        {/each}
        <path d={getChartPath()} fill="none" stroke="#3b82f6" stroke-width="1.5" />
        {#each latencyData.filter((d) => d.lost) as lost}
          <circle cx={(lost.time / Math.max(...latencyData.map((d) => d.time), 1)) * chartWidth} cy={chartHeight / 2} r="2" fill="#ef4444" />
        {/each}
      </svg>
    </div>
  {/if}

  {#if results}
    <div class="space-y-4">
      <div class="rounded-xl border border-gray-200/60 bg-gray-50 p-4 dark:border-gray-700/60 dark:bg-gray-800/50">
        <h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'testSummary')}</h3>
        <div class="grid grid-cols-2 gap-4 md:grid-cols-4">
          <div class="text-center"><div class="text-2xl font-bold">{results.ping_stats.packets_sent}</div><div class="text-xs text-gray-500 dark:text-gray-400">{t($locale, 'packetsSent')}</div></div>
          <div class="text-center"><div class="text-2xl font-bold">{results.ping_stats.packets_received}</div><div class="text-xs text-gray-500 dark:text-gray-400">{t($locale, 'packetsReceived')}</div></div>
          <div class="text-center"><div class="text-2xl font-bold">{results.ping_stats.packet_loss_percent.toFixed(1)}%</div><div class="text-xs text-gray-500 dark:text-gray-400">{t($locale, 'packetLoss')}</div></div>
          <div class="text-center"><div class="text-2xl font-bold">{results.ping_stats.avg_rtt_ms?.toFixed(1) ?? '-'}</div><div class="text-xs text-gray-500 dark:text-gray-400">{t($locale, 'averageLatencyMs')}</div></div>
        </div>
      </div>

      <div class="rounded-xl border border-gray-200/60 bg-white p-4 dark:border-gray-700/60 dark:bg-gray-800/50">
        <h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'latencyStats')}</h3>
        <div class="grid grid-cols-2 gap-4 md:grid-cols-4">
          <div><div class="text-sm font-medium">{formatLatency(results.ping_stats.min_rtt_ms ? results.ping_stats.min_rtt_ms * 1000 : null)}</div><div class="text-xs text-gray-500 dark:text-gray-400">{t($locale, 'minimumRtt')}</div></div>
          <div><div class="text-sm font-medium">{formatLatency(results.ping_stats.max_rtt_ms ? results.ping_stats.max_rtt_ms * 1000 : null)}</div><div class="text-xs text-gray-500 dark:text-gray-400">{t($locale, 'maximumRtt')}</div></div>
          <div><div class="text-sm font-medium">{results.ping_stats.jitter_ms?.toFixed(2) ?? '-'} ms</div><div class="text-xs text-gray-500 dark:text-gray-400">{t($locale, 'jitterMs')}</div></div>
          <div><div class="text-sm font-medium">{results.ping_stats.std_dev_ms?.toFixed(2) ?? '-'} ms</div><div class="text-xs text-gray-500 dark:text-gray-400">{t($locale, 'stdDev')}</div></div>
        </div>
      </div>

      <div class="rounded-xl border border-gray-200/60 bg-white p-4 dark:border-gray-700/60 dark:bg-gray-800/50">
        <div class="mb-3 flex items-center justify-between">
          <h3 class="text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'roamingTest')}</h3>
          <span class="text-sm font-medium text-indigo-600 dark:text-indigo-400">{t($locale, 'roamingDetected', { count: results.roaming_count })}</span>
        </div>
        {#if results.roaming_events.length === 0}
          <p class="text-sm text-gray-500 dark:text-gray-400">{t($locale, 'noRoamingEvents')}</p>
        {:else}
          <div class="space-y-3">
            {#each results.roaming_events as event, index}
              <div class="rounded-lg border border-gray-200/70 p-3 dark:border-gray-700/70">
                <div class="mb-2 font-medium text-gray-900 dark:text-white">{t($locale, 'roamingEvent', { index: index + 1 })}</div>
                <div class="grid grid-cols-2 gap-3 text-sm md:grid-cols-4">
                  <div><div class="text-xs text-gray-500 dark:text-gray-400">{t($locale, 'sourceAp')}</div><div>{event.from_ssid ?? event.from_bssid ?? '-'}</div></div>
                  <div><div class="text-xs text-gray-500 dark:text-gray-400">{t($locale, 'targetAp')}</div><div>{event.to_ssid ?? event.to_bssid ?? '-'}</div></div>
                  <div><div class="text-xs text-gray-500 dark:text-gray-400">{t($locale, 'roamingDuration')}</div><div>{formatDuration(event.roaming_duration_ms)}</div></div>
                  <div><div class="text-xs text-gray-500 dark:text-gray-400">{t($locale, 'lostPackets')}</div><div>{event.packets_lost}</div></div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      {#if results.latency_spikes.length > 0}
        <div class="rounded-xl border border-gray-200/60 bg-white p-4 dark:border-gray-700/60 dark:bg-gray-800/50">
          <h3 class="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'latencySpikes', { count: results.latency_spikes.length })}</h3>
          <div class="space-y-2">
            {#each results.latency_spikes.slice(0, 10) as spike}
              <div class="flex items-center justify-between rounded-lg bg-gray-50 px-3 py-2 text-sm dark:bg-gray-700/60">
                <span>{new Date(spike.timestamp_ms).toLocaleTimeString()}</span>
                <span>{spike.latency_ms.toFixed(1)} ms</span>
                <span class="text-gray-500 dark:text-gray-400">{t($locale, 'baselineSuffix', { ratio: spike.spike_ratio.toFixed(1) })}</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

