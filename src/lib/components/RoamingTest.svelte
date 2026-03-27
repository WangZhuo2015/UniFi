<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import Button from './ui/button.svelte';

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

  // Configuration
  let targetHost = $state('8.8.8.8');
  let durationSecs = $state(60);
  let intervalMs = $state(100);
  
  // State
  let isRunning = $state(false);
  let results = $state<RoamingTestResult | null>(null);
  let currentProgress = $state({ current: 0, total: 0 });
  let error = $state<string | null>(null);
  
  // Chart data
  let latencyData = $state<{ time: number; latency: number; lost: boolean }[]>([]);
  let chartWidth = 600;
  let chartHeight = 200;
  
  let pollInterval: ReturnType<typeof setInterval> | null = null;

  async function startTest() {
    try {
      error = null;
      results = null;
      latencyData = [];
      
      await invoke('start_roaming_test', {
        target: targetHost,
        durationSecs,
        intervalMs
      });
      
      isRunning = true;
      
      // Start polling for results
      pollInterval = setInterval(async () => {
        try {
          const [running, current, total] = await invoke<[boolean, number, number]>('get_roaming_test_status');
          currentProgress = { current, total };
          
          const res = await invoke<RoamingTestResult>('get_roaming_test_results');
          results = res;
          
          // Update chart data
          if (res.ping_results.length > 0) {
            latencyData = res.ping_results.slice(-200).map(p => ({
              time: p.timestamp_ms / 1000,
              latency: p.rtt_us ? p.rtt_us / 1000 : 0,
              lost: p.lost
            }));
          }
          
          if (!running) {
            isRunning = false;
            if (pollInterval) clearInterval(pollInterval);
          }
        } catch (e) {
          console.error('Failed to get results:', e);
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

  // Calculate chart path
  function getChartPath(): string {
    if (latencyData.length < 2) return '';
    
    const maxLatency = Math.max(...latencyData.map(d => d.lost ? 0 : d.latency), 1);
    const maxTime = Math.max(...latencyData.map(d => d.time), 1);
    
    const points = latencyData
      .filter(d => !d.lost && d.latency > 0)
      .map((d, i) => {
        const x = (d.time / maxTime) * chartWidth;
        const y = chartHeight - (d.latency / maxLatency) * chartHeight * 0.9;
        return `${i === 0 ? 'M' : 'L'} ${x} ${y}`;
      });
    
    return points.join(' ');
  }

  function formatLatency(us: number | null): string {
    if (us === null) return '-';
    if (us < 1000) return `${us}µs`;
    return `${(us / 1000).toFixed(1)}ms`;
  }

  function formatDuration(ms: number | null): string {
    if (ms === null) return '-';
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  }
</script>

<div class="p-4 space-y-4">
  <div class="flex items-center justify-between">
    <h2 class="text-lg font-bold">漫游测试</h2>
    <div class="flex items-center gap-2">
      {#if isRunning}
        <Button variant="destructive" onclick={stopTest}>
          <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 10a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1v-4z" />
          </svg>
          停止测试
        </Button>
      {:else}
        <Button onclick={startTest}>
          <svg class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          开始测试
        </Button>
      {/if}
    </div>
  </div>

  <!-- Configuration -->
  <div class="bg-gray-50 dark:bg-gray-800/50 rounded-xl p-4 border border-gray-200/50 dark:border-gray-700/50">
    <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-3">测试配置</h3>
    <div class="grid grid-cols-3 gap-4">
      <div>
        <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">目标地址</label>
        <input
          type="text"
          bind:value={targetHost}
          disabled={isRunning}
          class="w-full px-3 py-2 text-sm bg-white dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded-lg disabled:opacity-50"
        />
      </div>
      <div>
        <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">测试时长 (秒)</label>
        <input
          type="number"
          bind:value={durationSecs}
          disabled={isRunning}
          min="10"
          max="600"
          class="w-full px-3 py-2 text-sm bg-white dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded-lg disabled:opacity-50"
        />
      </div>
      <div>
        <label class="block text-xs text-gray-500 dark:text-gray-400 mb-1">Ping间隔 (ms)</label>
        <input
          type="number"
          bind:value={intervalMs}
          disabled={isRunning}
          min="50"
          max="1000"
          class="w-full px-3 py-2 text-sm bg-white dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded-lg disabled:opacity-50"
        />
      </div>
    </div>
  </div>

  {#if error}
    <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-3 text-sm text-red-700 dark:text-red-300">
      {error}
    </div>
  {/if}

  <!-- Progress -->
  {#if isRunning}
    <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-3">
      <div class="flex items-center justify-between mb-2">
        <span class="text-sm font-medium text-blue-700 dark:text-blue-300">测试进行中...</span>
        <span class="text-xs text-blue-600 dark:text-blue-400">
          {currentProgress.current} / {currentProgress.total} pings
        </span>
      </div>
      <div class="h-2 bg-blue-200 dark:bg-blue-800 rounded-full overflow-hidden">
        <div 
          class="h-full bg-blue-500 transition-all duration-300"
          style="width: {currentProgress.total > 0 ? (currentProgress.current / currentProgress.total * 100) : 0}%"
        ></div>
      </div>
    </div>
  {/if}

  <!-- Real-time Chart -->
  {#if latencyData.length > 0}
    <div class="bg-white dark:bg-gray-800/50 rounded-xl p-4 border border-gray-200/50 dark:border-gray-700/50">
      <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-3">实时延迟图表</h3>
      <svg width="100%" height={chartHeight} class="overflow-visible">
        <!-- Grid lines -->
        {#each [0, 25, 50, 75, 100] as pct}
          <line 
            x1="0" 
            y1={chartHeight * (1 - pct/100)} 
            x2="100%" 
            y2={chartHeight * (1 - pct/100)} 
            stroke="currentColor" 
            class="text-gray-200 dark:text-gray-700"
          />
        {/each}
        
        <!-- Latency line -->
        <path 
          d={getChartPath()} 
          fill="none" 
          stroke="#3b82f6" 
          stroke-width="1.5"
        />
        
        <!-- Lost packets as dots -->
        {#each latencyData.filter(d => d.lost) as lost}
          <circle 
            cx={(lost.time / Math.max(...latencyData.map(d => d.time), 1)) * chartWidth}
            cy={chartHeight / 2}
            r="2"
            fill="#ef4444"
          />
        {/each}
      </svg>
    </div>
  {/if}

  <!-- Results -->
  {#if results}
    <div class="space-y-4">
      <!-- Summary Stats -->
      <div class="bg-gray-50 dark:bg-gray-800/50 rounded-xl p-4 border border-gray-200/50 dark:border-gray-700/50">
        <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-3">测试结果摘要</h3>
        <div class="grid grid-cols-4 gap-4">
          <div class="text-center">
            <div class="text-2xl font-bold text-gray-900 dark:text-white">
              {results.ping_stats.packets_sent}
            </div>
            <div class="text-xs text-gray-500 dark:text-gray-400">发送包数</div>
          </div>
          <div class="text-center">
            <div class="text-2xl font-bold text-gray-900 dark:text-white">
              {results.ping_stats.packet_loss_percent.toFixed(1)}%
            </div>
            <div class="text-xs text-gray-500 dark:text-gray-400">丢包率</div>
          </div>
          <div class="text-center">
            <div class="text-2xl font-bold text-gray-900 dark:text-white tabular-nums">
              {results.ping_stats.avg_rtt_ms?.toFixed(1) ?? '-'}
            </div>
            <div class="text-xs text-gray-500 dark:text-gray-400">平均延迟 (ms)</div>
          </div>
          <div class="text-center">
            <div class="text-2xl font-bold text-gray-900 dark:text-white tabular-nums">
              {results.ping_stats.jitter_ms?.toFixed(1) ?? '-'}
            </div>
            <div class="text-xs text-gray-500 dark:text-gray-400">抖动 (ms)</div>
          </div>
        </div>
      </div>

      <!-- Detailed Stats -->
      <div class="bg-gray-50 dark:bg-gray-800/50 rounded-xl p-4 border border-gray-200/50 dark:border-gray-700/50">
        <h3 class="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-3">延迟统计</h3>
        <div class="grid grid-cols-2 gap-2">
          <div class="flex justify-between px-3 py-2 bg-white dark:bg-gray-700/50 rounded-lg">
            <span class="text-gray-500 dark:text-gray-400 text-sm">最小延迟</span>
            <span class="font-medium tabular-nums">{results.ping_stats.min_rtt_ms?.toFixed(2) ?? '-'} ms</span>
          </div>
          <div class="flex justify-between px-3 py-2 bg-white dark:bg-gray-700/50 rounded-lg">
            <span class="text-gray-500 dark:text-gray-400 text-sm">最大延迟</span>
            <span class="font-medium tabular-nums">{results.ping_stats.max_rtt_ms?.toFixed(2) ?? '-'} ms</span>
          </div>
          <div class="flex justify-between px-3 py-2 bg-white dark:bg-gray-700/50 rounded-lg">
            <span class="text-gray-500 dark:text-gray-400 text-sm">标准差</span>
            <span class="font-medium tabular-nums">{results.ping_stats.std_dev_ms?.toFixed(2) ?? '-'} ms</span>
          </div>
          <div class="flex justify-between px-3 py-2 bg-white dark:bg-gray-700/50 rounded-lg">
            <span class="text-gray-500 dark:text-gray-400 text-sm">测试时长</span>
            <span class="font-medium">{results.duration_secs} 秒</span>
          </div>
        </div>
      </div>

      <!-- Roaming Events -->
      {#if results.roaming_count > 0}
        <div class="bg-orange-50 dark:bg-orange-900/20 rounded-xl p-4 border border-orange-200/50 dark:border-orange-800/50">
          <h3 class="text-xs font-semibold text-orange-600 dark:text-orange-300 uppercase tracking-wide mb-3 flex items-center gap-2">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
            </svg>
            检测到 {results.roaming_count} 次漫游
          </h3>
          <div class="space-y-2">
            {#each results.roaming_events as event, i}
              <div class="bg-white dark:bg-gray-800/50 rounded-lg p-3 text-sm">
                <div class="flex items-center justify-between mb-2">
                  <span class="font-medium">漫游 #{i + 1}</span>
                  <span class="text-xs text-gray-500">{(event.timestamp_ms / 1000).toFixed(1)}s</span>
                </div>
                <div class="grid grid-cols-2 gap-2 text-xs">
                  <div>
                    <span class="text-gray-500">源 AP:</span>
                    <span class="ml-1 font-mono">{event.from_bssid ?? 'Unknown'}</span>
                  </div>
                  <div>
                    <span class="text-gray-500">目标 AP:</span>
                    <span class="ml-1 font-mono">{event.to_bssid ?? 'Unknown'}</span>
                  </div>
                  <div>
                    <span class="text-gray-500">漫游耗时:</span>
                    <span class="ml-1 font-medium text-orange-600">{formatDuration(event.roaming_duration_ms)}</span>
                  </div>
                  <div>
                    <span class="text-gray-500">丢包:</span>
                    <span class="ml-1 font-medium text-red-500">{event.packets_lost} 个</span>
                  </div>
                </div>
              </div>
            {/each}
          </div>
          <div class="mt-3 pt-3 border-t border-orange-200 dark:border-orange-800 grid grid-cols-2 gap-4 text-sm">
            <div>
              <span class="text-orange-700 dark:text-orange-300">平均漫游耗时:</span>
              <span class="ml-2 font-medium">{formatDuration(results.avg_roaming_duration_ms)}</span>
            </div>
            <div>
              <span class="text-orange-700 dark:text-orange-300">漫游总丢包:</span>
              <span class="ml-2 font-medium">{results.total_roaming_packet_loss} 个</span>
            </div>
          </div>
        </div>
      {:else}
        <div class="bg-green-50 dark:bg-green-900/20 rounded-xl p-4 border border-green-200/50 dark:border-green-800/50 text-sm text-green-700 dark:text-green-300">
          <div class="flex items-center gap-2">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            测试期间未检测到漫游事件
          </div>
        </div>
      {/if}

      <!-- Latency Spikes -->
      {#if results.latency_spikes.length > 0}
        <div class="bg-yellow-50 dark:bg-yellow-900/20 rounded-xl p-4 border border-yellow-200/50 dark:border-yellow-800/50">
          <h3 class="text-xs font-semibold text-yellow-600 dark:text-yellow-300 uppercase tracking-wide mb-3">
            延迟突变 ({results.latency_spikes.length} 次)
          </h3>
          <div class="max-h-32 overflow-y-auto space-y-1">
            {#each results.latency_spikes.slice(0, 20) as spike}
              <div class="flex items-center justify-between text-xs bg-white dark:bg-gray-800/50 rounded px-2 py-1">
                <span>{(spike.timestamp_ms / 1000).toFixed(1)}s</span>
                <span class="font-medium">{spike.latency_ms.toFixed(1)}ms</span>
                <span class="text-yellow-600">{spike.spike_ratio.toFixed(1)}x baseline</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
