<script lang="ts">
  import { locale, t } from '$lib/i18n';
  import { byBand } from '$lib/stores';
  import { cn } from '$lib/utils';
  import type { ChannelScore, Network } from '$lib/types';

  let selectedBand = $state<'2.4' | '5'>('2.4');

  const channels24 = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
  const channels5 = [36, 40, 44, 48, 52, 56, 60, 64, 100, 104, 108, 112, 116, 120, 124, 128, 132, 136, 140, 144, 149, 153, 157, 161, 165];

  function scoreChannel(channel: number, networks: Network[]): ChannelScore {
    let score = 100;
    const nearby: string[] = [];

    for (const n of networks) {
      const overlaps = selectedBand === '2.4' ? Math.abs(n.channel - channel) <= 4 : n.channel === channel;
      if (overlaps) {
        const penalty = n.signal >= -50 ? 30 : n.signal >= -60 ? 20 : n.signal >= -70 ? 10 : 5;
        score -= penalty;
        nearby.push(n.ssid ?? t($locale, 'hiddenNetwork'));
      }
    }

    if (selectedBand === '5' && ((channel >= 52 && channel <= 64) || (channel >= 100 && channel <= 144))) {
      score -= 5;
    }

    score = Math.max(0, score);

    return {
      channel,
      score,
      networks: nearby,
      interference: 0,
      utilization: 0,
      recommendation: score >= 80 ? 'best' : score >= 60 ? 'good' : score >= 40 ? 'fair' : 'avoid'
    };
  }

  const channelScores = $derived(() => {
    const nets = $byBand[selectedBand];
    const channels = selectedBand === '2.4' ? channels24 : channels5;
    return channels.map((ch) => scoreChannel(ch, nets));
  });

  const recommendations = $derived(channelScores().filter((c) => c.recommendation === 'best' || c.recommendation === 'good').slice(0, 3));

  function scoreColor(score: ChannelScore): string {
    return score.recommendation === 'best' ? '#22c55e' : score.recommendation === 'good' ? '#84cc16' : score.recommendation === 'fair' ? '#eab308' : '#ef4444';
  }
</script>

<div class="p-4">
  <div class="flex gap-2 mb-4">
    <button class={cn('px-4 py-2 rounded transition-colors', selectedBand === '2.4' ? 'bg-blue-600 text-white' : 'bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-200')} onclick={() => selectedBand = '2.4'}>2.4 GHz</button>
    <button class={cn('px-4 py-2 rounded transition-colors', selectedBand === '5' ? 'bg-blue-600 text-white' : 'bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-200')} onclick={() => selectedBand = '5'}>5 GHz</button>
  </div>

  {#if recommendations.length > 0}
    <div class="mb-4 p-3 bg-green-50 dark:bg-green-900/30 rounded-lg border border-green-200 dark:border-green-800">
      <div class="text-sm font-medium text-green-800 dark:text-green-300">{t($locale, 'recommendChannels')}</div>
      <div class="flex flex-wrap gap-2 mt-1">
        {#each recommendations as rec}
          <span class="px-2 py-1 bg-green-100 dark:bg-green-800 text-green-700 dark:text-green-300 rounded text-sm">
            CH {rec.channel} ({t($locale, 'pointsSuffix', { score: rec.score })})
          </span>
        {/each}
      </div>
    </div>
  {/if}

  <div class="relative h-32 bg-gray-100 dark:bg-gray-800 rounded-lg overflow-hidden border border-gray-200 dark:border-gray-700">
    {#each channelScores() as cs, i}
      {@const channels = selectedBand === '2.4' ? channels24 : channels5}
      {@const x = ((i + 0.5) / channels.length) * 100}
      {@const w = (100 / channels.length) * 0.7}
      <div class="absolute bottom-6 rounded-t transition-all" style="left: {x}%; width: {w}%; height: {cs.score}%; background-color: {scoreColor(cs)}" title={t($locale, 'channelScoreTitle', { channel: cs.channel, score: cs.score })}></div>
    {/each}

    <div class="absolute bottom-0 left-0 right-0 h-6 bg-white dark:bg-gray-900 border-t border-gray-200 dark:border-gray-700 flex">
      {#each (selectedBand === '2.4' ? channels24 : channels5) as ch, i}
        {@const show = selectedBand === '2.4' || i % 3 === 0}
        <div class="flex-1 flex items-center justify-center text-xs text-gray-500 dark:text-gray-400">{show ? ch : ''}</div>
      {/each}
    </div>
  </div>

  <div class="mt-4 space-y-1 max-h-48 overflow-y-auto">
    {#each channelScores().filter((c) => c.networks.length > 0) as cs}
      <div class="flex items-center justify-between text-sm p-2 bg-gray-50 dark:bg-gray-800 rounded border border-gray-100 dark:border-gray-700">
        <span class="font-medium text-gray-900 dark:text-gray-100">CH {cs.channel}</span>
        <span class="text-gray-500 dark:text-gray-400 truncate mx-2">{cs.networks.join(', ') || t($locale, 'noOverlappingNetworks')}</span>
        <span class={cn('font-bold', cs.recommendation === 'best' && 'text-green-600', cs.recommendation === 'good' && 'text-lime-600', cs.recommendation === 'fair' && 'text-yellow-600', cs.recommendation === 'avoid' && 'text-red-600')}>{cs.score}</span>
      </div>
    {/each}
  </div>
</div>
