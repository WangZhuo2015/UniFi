<script lang="ts">
  import { locale, t } from '$lib/i18n';
  import { byBand } from '$lib/stores';
  import { cn } from '$lib/utils';
  import type { ChannelScore, Network } from '$lib/types';

  let selectedBand = $state<'2.4' | '5'>('2.4');

  const channels24 = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
  const channels5 = [36, 40, 44, 48, 52, 56, 60, 64, 100, 104, 108, 112, 116, 120, 124, 128, 132, 136, 140, 144, 149, 153, 157, 161, 165];
  const preferred24 = new Set([1, 6, 11]);

  function scoreChannel(channel: number, networks: Network[]): ChannelScore {
    let score = 100;
    let interference = 0;
    const nearby: string[] = [];

    for (const network of networks) {
      const overlaps = selectedBand === '2.4' ? Math.abs(network.channel - channel) <= 4 : network.channel === channel;
      if (!overlaps) {
        continue;
      }

      const penalty = network.signal >= -50 ? 30 : network.signal >= -60 ? 20 : network.signal >= -70 ? 10 : 5;
      score -= penalty;
      interference += penalty;
      nearby.push(network.ssid ?? t($locale, 'hiddenNetwork'));
    }

    if (selectedBand === '2.4' && !preferred24.has(channel)) {
      score -= 8;
    }

    if (selectedBand === '5' && ((channel >= 52 && channel <= 64) || (channel >= 100 && channel <= 144))) {
      score -= 5;
    }

    score = Math.max(0, score);

    return {
      channel,
      score,
      networks: nearby,
      interference,
      utilization: Math.min(100, Math.round(interference / 1.6)),
      recommendation: score >= 80 ? 'best' : score >= 60 ? 'good' : score >= 40 ? 'fair' : 'avoid'
    };
  }

  const visibleNetworks = $derived($byBand[selectedBand]);

  const channelScores = $derived.by(() => {
    const channels = selectedBand === '2.4' ? channels24 : channels5;
    return channels.map((channel) => scoreChannel(channel, visibleNetworks));
  });

  const topRecommendations = $derived(
    channelScores.filter((channel) => channel.recommendation === 'best' || channel.recommendation === 'good').slice(0, 3)
  );

  const busiestChannels = $derived(
    [...channelScores]
      .sort((left, right) => right.networks.length - left.networks.length || left.score - right.score)
      .filter((channel) => channel.networks.length > 0)
      .slice(0, 5)
  );

  const channelSummary = $derived.by(() => {
    const scores = channelScores;
    const best = [...scores].sort((left, right) => right.score - left.score)[0];
    const congested = [...scores].sort((left, right) => right.networks.length - left.networks.length)[0];

    return {
      scanned: visibleNetworks.length,
      best,
      congested,
      averageScore: scores.length ? Math.round(scores.reduce((sum, entry) => sum + entry.score, 0) / scores.length) : 0
    };
  });

  function scoreColor(channel: ChannelScore): string {
    return channel.recommendation === 'best'
      ? 'bg-emerald-500/15 text-emerald-300 ring-emerald-500/30'
      : channel.recommendation === 'good'
        ? 'bg-lime-500/15 text-lime-300 ring-lime-500/30'
        : channel.recommendation === 'fair'
          ? 'bg-amber-500/15 text-amber-300 ring-amber-500/30'
          : 'bg-rose-500/15 text-rose-300 ring-rose-500/30';
  }

  function barTone(channel: ChannelScore): string {
    return channel.recommendation === 'best'
      ? 'from-emerald-400 to-teal-500'
      : channel.recommendation === 'good'
        ? 'from-lime-400 to-emerald-500'
        : channel.recommendation === 'fair'
          ? 'from-amber-400 to-orange-500'
          : 'from-rose-400 to-red-500';
  }

  function recommendationLabel(channel: ChannelScore): string {
    if (channel.recommendation === 'best') return t($locale, 'channelBest');
    if (channel.recommendation === 'good') return t($locale, 'channelGood');
    if (channel.recommendation === 'fair') return t($locale, 'channelFair');
    return t($locale, 'channelAvoid');
  }
</script>

<div class="flex h-full flex-col gap-5 p-5" data-testid="channels-view">
  <section class="rounded-2xl border border-slate-700/70 bg-slate-900/80 p-4 shadow-lg shadow-slate-950/25">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <p class="text-xs font-semibold uppercase tracking-[0.24em] text-slate-400">{t($locale, 'channelsSummaryTitle')}</p>
        <h2 class="mt-1 text-lg font-semibold text-white">{t($locale, 'channelsTab')}</h2>
        <p class="mt-1 text-sm text-slate-400">{t($locale, 'channelsSummaryHint')}</p>
      </div>

      <div class="inline-flex rounded-xl border border-slate-700 bg-slate-950/80 p-1">
        {#each [
          { value: '2.4', label: '2.4 GHz' },
          { value: '5', label: '5 GHz' }
        ] as band}
          <button
            type="button"
            data-testid={`channel-band-${band.value}`}
            class={cn(
              'rounded-lg px-4 py-2 text-sm font-medium transition-colors',
              selectedBand === band.value
                ? 'bg-blue-500 text-white shadow-sm shadow-blue-950/50'
                : 'text-slate-300 hover:bg-slate-800 hover:text-white'
            )}
            onclick={() => selectedBand = band.value as typeof selectedBand}
          >
            {band.label}
          </button>
        {/each}
      </div>
    </div>

    <div class="mt-4 grid gap-3 md:grid-cols-4">
      <div class="rounded-xl border border-slate-800 bg-slate-950/70 p-4" data-testid="channels-summary-scanned">
        <div class="text-xs uppercase tracking-wide text-slate-500">{t($locale, 'networksTab')}</div>
        <div class="mt-2 text-2xl font-semibold text-white">{channelSummary.scanned}</div>
        <div class="mt-1 text-xs text-slate-400">{selectedBand} GHz</div>
      </div>
      <div class="rounded-xl border border-slate-800 bg-slate-950/70 p-4" data-testid="channels-summary-best">
        <div class="text-xs uppercase tracking-wide text-slate-500">{t($locale, 'recommendChannels')}</div>
        <div class="mt-2 text-2xl font-semibold text-white">{channelSummary.best ? `CH ${channelSummary.best.channel}` : '-'}</div>
        <div class="mt-1 text-xs text-slate-400">{channelSummary.best ? recommendationLabel(channelSummary.best) : t($locale, 'emptyStateHint')}</div>
      </div>
      <div class="rounded-xl border border-slate-800 bg-slate-950/70 p-4" data-testid="channels-summary-congested">
        <div class="text-xs uppercase tracking-wide text-slate-500">{t($locale, 'channelMostCongested')}</div>
        <div class="mt-2 text-2xl font-semibold text-white">{channelSummary.congested ? `CH ${channelSummary.congested.channel}` : '-'}</div>
        <div class="mt-1 text-xs text-slate-400">{channelSummary.congested?.networks.length ?? 0} {t($locale, 'devicesLabel')}</div>
      </div>
      <div class="rounded-xl border border-slate-800 bg-slate-950/70 p-4" data-testid="channels-summary-average">
        <div class="text-xs uppercase tracking-wide text-slate-500">{t($locale, 'channelAverageHealth')}</div>
        <div class="mt-2 text-2xl font-semibold text-white">{channelSummary.averageScore}</div>
        <div class="mt-1 text-xs text-slate-400">{t($locale, 'pointsSuffix', { score: channelSummary.averageScore })}</div>
      </div>
    </div>
  </section>

  {#if topRecommendations.length > 0}
    <section class="rounded-2xl border border-emerald-500/20 bg-emerald-500/5 p-4">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p class="text-xs font-semibold uppercase tracking-[0.24em] text-emerald-300">{t($locale, 'recommendChannels')}</p>
          <p class="mt-1 text-sm text-slate-300">{t($locale, 'channelRecommendationHint')}</p>
        </div>
        <div class="flex flex-wrap gap-2">
          {#each topRecommendations as channel}
            <div class="rounded-full border border-emerald-500/30 bg-emerald-500/10 px-3 py-1 text-sm font-medium text-emerald-200" data-testid="channel-recommendation">
              CH {channel.channel} · {t($locale, 'pointsSuffix', { score: channel.score })}
            </div>
          {/each}
        </div>
      </div>
    </section>
  {/if}

  <section class="grid gap-5 xl:grid-cols-[minmax(0,1.35fr)_minmax(18rem,0.65fr)]">
    <div class="rounded-2xl border border-slate-700/70 bg-slate-900/80 p-4 shadow-lg shadow-slate-950/25">
      <div class="mb-4">
        <h3 class="text-sm font-semibold uppercase tracking-[0.24em] text-slate-400">{t($locale, 'channelHealthGrid')}</h3>
        <p class="mt-1 text-sm text-slate-500">{t($locale, 'channelHealthGridHint')}</p>
      </div>

      <div class="grid gap-3 sm:grid-cols-2 2xl:grid-cols-3">
        {#each channelScores as channel}
          <article class="rounded-2xl border border-slate-800 bg-slate-950/75 p-4" data-testid="channel-card">
            <div class="flex items-start justify-between gap-3">
              <div>
                <p class="text-base font-semibold text-white">CH {channel.channel}</p>
                <p class="mt-1 text-xs text-slate-400">{recommendationLabel(channel)}</p>
              </div>
              <span class={cn('rounded-full px-2.5 py-1 text-xs font-semibold ring-1', scoreColor(channel))}>
                {channel.score}
              </span>
            </div>

            <div class="mt-4 h-2 overflow-hidden rounded-full bg-slate-800">
              <div class={cn('h-full rounded-full bg-gradient-to-r', barTone(channel))} style={`width:${Math.max(4, channel.score)}%`}></div>
            </div>

            <dl class="mt-4 grid grid-cols-2 gap-3 text-sm">
              <div class="rounded-xl bg-slate-900/60 p-3">
                <dt class="text-xs uppercase tracking-wide text-slate-500">{t($locale, 'channelHealth')}</dt>
                <dd class="mt-1 font-medium text-white">{t($locale, 'pointsSuffix', { score: channel.score })}</dd>
              </div>
              <div class="rounded-xl bg-slate-900/60 p-3">
                <dt class="text-xs uppercase tracking-wide text-slate-500">{t($locale, 'overlappingAps')}</dt>
                <dd class="mt-1 font-medium text-white">{channel.networks.length}</dd>
              </div>
            </dl>
          </article>
        {/each}
      </div>
    </div>

    <div class="flex flex-col gap-5">
      <section class="rounded-2xl border border-slate-700/70 bg-slate-900/80 p-4 shadow-lg shadow-slate-950/25">
        <h3 class="text-sm font-semibold uppercase tracking-[0.24em] text-slate-400">{t($locale, 'channelInterferenceDetail')}</h3>
        <p class="mt-1 text-sm text-slate-500">{t($locale, 'channelInterferenceHint')}</p>

        <div class="mt-4 space-y-3" data-testid="channel-busiest-list">
          {#if busiestChannels.length === 0}
            <div class="rounded-xl border border-dashed border-slate-700 bg-slate-950/70 px-4 py-6 text-sm text-slate-400">
              {t($locale, 'channelEmptyState')}
            </div>
          {:else}
            {#each busiestChannels as channel}
              <div class="rounded-xl border border-slate-800 bg-slate-950/75 p-4">
                <div class="flex items-start justify-between gap-3">
                  <div>
                    <p class="text-sm font-semibold text-white">CH {channel.channel}</p>
                    <p class="mt-1 text-xs text-slate-400">{recommendationLabel(channel)}</p>
                  </div>
                  <span class="rounded-full bg-slate-800 px-2.5 py-1 text-xs font-medium text-slate-200">
                    {channel.networks.length} {t($locale, 'devicesLabel')}
                  </span>
                </div>
                <div class="mt-3 text-sm text-slate-300">
                  {channel.networks.join(', ')}
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </section>
    </div>
  </section>
</div>
