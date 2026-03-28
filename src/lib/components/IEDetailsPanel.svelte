<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { locale, t } from '$lib/i18n';
  import type { IEDetails, ParsedIE, Network } from '$lib/types';
  import VendorLogo from '$lib/components/VendorLogo.svelte';

  export let network: Network | null = null;
  export let onClose: () => void = () => {};

  let ieDetails: IEDetails | null = null;
  let loading = false;
  let error = '';
  let selectedIE: ParsedIE | null = null;
  let loadedBssid = '';

  function handleBackdropKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ' || event.key === 'Escape') {
      event.preventDefault();
      onClose();
    }
  }

  function stopOverlayInteraction(event: Event) {
    event.stopPropagation();
  }

  function loadDetails(bssid: string) {
    loadedBssid = bssid;
    loading = true;
    error = '';
    ieDetails = null;
    selectedIE = null;

    invoke<IEDetails>('get_ie_details', { bssid })
      .then((details) => {
        ieDetails = details;
        selectedIE = details.elements[0] ?? null;
      })
      .catch((e) => {
        error = String(e);
      })
      .finally(() => {
        loading = false;
      });
  }

  $: if (network?.bssid) {
    if (network.bssid !== loadedBssid) {
      loadDetails(network.bssid);
    }
  }

  function getIEColor(elementId: number): string {
    switch (elementId) {
      case 255: return 'bg-purple-100 dark:bg-purple-900 text-purple-700 dark:text-purple-300';
      case 191:
      case 192: return 'bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300';
      case 45:
      case 61: return 'bg-green-100 dark:bg-green-900 text-green-700 dark:text-green-300';
      case 48: return 'bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-300';
      case 221: return 'bg-amber-100 dark:bg-amber-900 text-amber-700 dark:text-amber-300';
      default: return 'bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300';
    }
  }

  function ieSummary(ie: ParsedIE) {
    return ie.summary || ie.name;
  }
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4" role="button" tabindex="0" aria-label={t($locale, 'closeDialog')} onclick={onClose} onkeydown={handleBackdropKeydown}>
  <div class="max-h-[90vh] w-full max-w-6xl overflow-hidden rounded-xl bg-white shadow-2xl dark:bg-gray-800" role="dialog" aria-modal="true" tabindex="-1" onclick={stopOverlayInteraction} onkeydown={stopOverlayInteraction}>
    <div class="flex items-center justify-between bg-gradient-to-r from-indigo-600 to-blue-600 px-6 py-4">
      <div>
        <h2 class="text-xl font-bold text-white">{t($locale, 'ieTitle')}</h2>
        <p class="text-sm text-indigo-100">{network?.ssid ?? t($locale, 'hiddenNetwork')} - {network?.bssid}</p>
      </div>
      <button type="button" class="text-2xl text-white/80 hover:text-white" aria-label={t($locale, 'closeDialog')} onclick={onClose}>&times;</button>
    </div>

    {#if loading}
      <div class="p-20 text-center">
        <div class="mx-auto mb-4 h-12 w-12 animate-spin rounded-full border-4 border-indigo-500 border-t-transparent"></div>
        <p class="text-gray-500 dark:text-gray-400">{t($locale, 'loadingIe')}</p>
      </div>
    {:else if error}
      <div class="p-10 text-center text-red-500">{error}</div>
    {:else if ieDetails}
      <div class="flex h-[calc(90vh-80px)]">
        <div class="w-[46%] overflow-auto border-r border-gray-200 dark:border-gray-700">
          <div class="border-b border-gray-200 bg-gray-50 p-4 dark:border-gray-700 dark:bg-gray-900">
            <h3 class="mb-2 font-semibold text-gray-900 dark:text-white">{t($locale, 'wifiStandardDetection')}</h3>
            <div class="flex flex-wrap gap-2">
              <span class="rounded px-2 py-1 text-xs {ieDetails.detectionSummary.hasEhtCapabilities ? 'bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200' : 'bg-gray-100 text-gray-400 dark:bg-gray-800'}">EHT</span>
              <span class="rounded px-2 py-1 text-xs {ieDetails.detectionSummary.hasHeCapabilities ? 'bg-indigo-100 text-indigo-800 dark:bg-indigo-900 dark:text-indigo-200' : 'bg-gray-100 text-gray-400 dark:bg-gray-800'}">HE</span>
              <span class="rounded px-2 py-1 text-xs {ieDetails.detectionSummary.hasVhtCapabilities ? 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200' : 'bg-gray-100 text-gray-400 dark:bg-gray-800'}">VHT</span>
              <span class="rounded px-2 py-1 text-xs {ieDetails.detectionSummary.hasHtCapabilities ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200' : 'bg-gray-100 text-gray-400 dark:bg-gray-800'}">HT</span>
            </div>
            <p class="mt-3 text-sm text-gray-600 dark:text-gray-300">
              {t($locale, 'detectionResult')}: <span class="font-semibold text-indigo-600 dark:text-indigo-400">{ieDetails.detectionSummary.detectedStandard}</span>
            </p>
          </div>

          <table class="w-full text-sm">
            <thead class="sticky top-0 bg-gray-100 dark:bg-gray-900">
              <tr>
                <th class="px-3 py-2 text-left text-gray-600 dark:text-gray-400">{t($locale, 'idLabel')}</th>
                <th class="px-3 py-2 text-left text-gray-600 dark:text-gray-400">{t($locale, 'nameLabel')}</th>
                <th class="px-3 py-2 text-left text-gray-600 dark:text-gray-400">Summary</th>
                <th class="px-3 py-2 text-center text-gray-600 dark:text-gray-400">{t($locale, 'lengthLabel')}</th>
              </tr>
            </thead>
            <tbody>
              {#each ieDetails.elements as ie (ie.elementId + '-' + ie.dataHex)}
                <tr class={`cursor-pointer border-b border-gray-100 hover:bg-gray-50 dark:border-gray-700 dark:hover:bg-gray-700 ${selectedIE === ie ? 'bg-indigo-50 dark:bg-indigo-950/30' : ''}`} onclick={() => selectedIE = ie}>
                  <td class="px-3 py-2">
                    <span class="rounded px-1.5 py-0.5 text-xs font-mono {getIEColor(ie.elementId)}">{ie.elementIdHex}</span>
                  </td>
                  <td class="px-3 py-2 text-gray-900 dark:text-gray-100">
                    <div>{ie.name}</div>
                    {#if ie.vendorName}
                      <div class="mt-1 text-xs text-gray-400 dark:text-gray-500">{ie.vendorName}</div>
                    {/if}
                  </td>
                  <td class="px-3 py-2 text-xs text-gray-600 dark:text-gray-300">{ieSummary(ie)}</td>
                  <td class="px-3 py-2 text-center text-gray-500 dark:text-gray-400">{ie.length}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        <div class="w-[54%] overflow-auto">
          {#if selectedIE}
            <div class="p-4">
              <div class="mb-4 rounded-2xl border border-gray-200/70 bg-gradient-to-br from-white to-slate-50 p-4 shadow-sm dark:border-gray-700 dark:from-gray-800 dark:to-gray-900">
                <div class="flex items-start justify-between gap-4">
                  <div>
                    <h3 class="font-semibold text-gray-900 dark:text-white">
                      {selectedIE.name} <span class="text-sm font-normal text-gray-400">(ID {selectedIE.elementIdHex})</span>
                    </h3>
                    <p class="mt-2 text-sm leading-6 text-gray-600 dark:text-gray-300">{ieSummary(selectedIE)}</p>
                  </div>
                  {#if selectedIE.vendorName}
                    <div class="flex items-center gap-2 rounded-xl border border-gray-200/60 bg-white/80 px-3 py-2 dark:border-gray-700 dark:bg-gray-800/80">
                      <VendorLogo vendor={selectedIE.vendorName} size="sm" />
                      <div class="text-xs font-medium text-gray-600 dark:text-gray-300">{selectedIE.vendorName}</div>
                    </div>
                  {/if}
                </div>
              </div>

              {#if selectedIE.displayFields.length > 0}
                <div>
                  <p class="text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'parsedFields')}</p>
                  <div class="mt-2 overflow-hidden rounded-xl border border-gray-200/70 bg-white dark:border-gray-700 dark:bg-gray-800">
                    {#each selectedIE.displayFields as row, index}
                      <div class={`flex items-center justify-between gap-4 px-4 py-3 ${index > 0 ? 'border-t border-gray-100 dark:border-gray-700' : ''}`}>
                        <span class="text-sm text-gray-500 dark:text-gray-400">{row.label}</span>
                        <span class={`text-right font-mono text-sm ${row.highlighted ? 'font-semibold text-indigo-600 dark:text-indigo-300' : 'text-gray-900 dark:text-white'}`}>{row.value}</span>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}

              <div class="mt-4">
                <p class="text-xs uppercase tracking-wide text-gray-500 dark:text-gray-400">{t($locale, 'rawDataHex')}</p>
                <div class="mt-1 overflow-auto rounded-lg bg-gray-900 p-3 dark:bg-gray-950">
                  <pre class="break-all font-mono text-xs text-green-400">{selectedIE.dataHex}</pre>
                </div>
              </div>
            </div>
          {:else}
            <div class="flex h-full items-center justify-center text-gray-400 dark:text-gray-500">
              <p>{t($locale, 'clickIePrompt')}</p>
            </div>
          {/if}
        </div>
      </div>

      <div class="border-t border-gray-200 bg-gray-50 p-3 dark:border-gray-700 dark:bg-gray-900">
        <details>
          <summary class="cursor-pointer text-sm text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white">
            {t($locale, 'fullBeaconIeData', { bytes: ieDetails.totalLength })}
          </summary>
          <div class="mt-2 max-h-40 overflow-auto rounded bg-gray-900 p-3 dark:bg-gray-950">
            <pre class="break-all font-mono text-xs text-cyan-400">{ieDetails.rawHex}</pre>
          </div>
        </details>
      </div>
    {/if}
  </div>
</div>


