<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { IEDetails, ParsedIE, Network } from '$lib/types';

  export let network: Network | null = null;
  export let onClose: () => void = () => {};

  let ieDetails: IEDetails | null = null;
  let loading = false;
  let error = '';
  let selectedIE: ParsedIE | null = null;

  $: if (network?.bssid) {
    loading = true;
    error = '';
    invoke<IEDetails>('get_ie_details', { bssid: network.bssid })
      .then((details) => {
        ieDetails = details;
        loading = false;
      })
      .catch((e) => {
        error = e.toString();
        loading = false;
      });
  }

  function getIEColor(elementId: number): string {
    switch (elementId) {
      case 255: return 'bg-purple-100 dark:bg-purple-900 text-purple-700 dark:text-purple-300';
      case 191: case 192: return 'bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300';
      case 45: case 61: return 'bg-green-100 dark:bg-green-900 text-green-700 dark:text-green-300';
      case 48: return 'bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-300';
      case 221: return 'bg-amber-100 dark:bg-amber-900 text-amber-700 dark:text-amber-300';
      default: return 'bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300';
    }
  }
</script>

<div class="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4" on:click={onClose}>
  <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-6xl w-full max-h-[90vh] overflow-hidden" on:click|stopPropagation={() => {}}>
    <!-- Header -->
    <div class="bg-gradient-to-r from-indigo-600 to-purple-600 px-6 py-4 flex items-center justify-between">
      <div>
        <h2 class="text-xl font-bold text-white">Beacon Frame 解析</h2>
        <p class="text-indigo-200 text-sm">{network?.ssid || 'Unknown'} - {network?.bssid}</p>
      </div>
      <button on:click={onClose} class="text-white/80 hover:text-white text-2xl">&times;</button>
    </div>

    {#if loading}
      <div class="p-20 text-center">
        <div class="animate-spin w-12 h-12 border-4 border-indigo-500 border-t-transparent rounded-full mx-auto mb-4"></div>
        <p class="text-gray-500 dark:text-gray-400">解析 IE 数据...</p>
      </div>
    {:else if error}
      <div class="p-10 text-center">
        <p class="text-red-500">{error}</p>
      </div>
    {:else if ieDetails}
      <div class="flex h-[calc(90vh-80px)]">
        <!-- Left: IE List -->
        <div class="w-1/2 border-r border-gray-200 dark:border-gray-700 overflow-auto">
          <!-- Detection Summary -->
          <div class="p-4 bg-gray-50 dark:bg-gray-900 border-b border-gray-200 dark:border-gray-700">
            <h3 class="font-semibold text-gray-900 dark:text-white mb-2">WiFi 标准检测</h3>
            <div class="flex flex-wrap gap-2">
              <span class="px-2 py-1 rounded text-xs {ieDetails.detectionSummary.hasEhtCapabilities ? 'bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200' : 'bg-gray-100 text-gray-400 dark:bg-gray-800'}">
                EHT Cap (Ext 108)
              </span>
              <span class="px-2 py-1 rounded text-xs {ieDetails.detectionSummary.hasHeCapabilities ? 'bg-indigo-100 text-indigo-800 dark:bg-indigo-900 dark:text-indigo-200' : 'bg-gray-100 text-gray-400 dark:bg-gray-800'}">
                HE Cap (Ext 35)
              </span>
              <span class="px-2 py-1 rounded text-xs {ieDetails.detectionSummary.hasVhtCapabilities ? 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200' : 'bg-gray-100 text-gray-400 dark:bg-gray-800'}">
                VHT (ID 191)
              </span>
              <span class="px-2 py-1 rounded text-xs {ieDetails.detectionSummary.hasHtCapabilities ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200' : 'bg-gray-100 text-gray-400 dark:bg-gray-800'}">
                HT (ID 45)
              </span>
            </div>
            <p class="mt-2 text-lg font-bold text-gray-900 dark:text-white">
              检测结果: <span class="text-indigo-600 dark:text-indigo-400">{ieDetails.detectionSummary.detectedStandard}</span>
            </p>
          </div>

          <!-- IE Elements Table -->
          <table class="w-full text-sm">
            <thead class="bg-gray-100 dark:bg-gray-900 sticky top-0">
              <tr>
                <th class="px-3 py-2 text-left text-gray-600 dark:text-gray-400">ID</th>
                <th class="px-3 py-2 text-left text-gray-600 dark:text-gray-400">名称</th>
                <th class="px-3 py-2 text-center text-gray-600 dark:text-gray-400">Len</th>
              </tr>
            </thead>
            <tbody>
              {#each ieDetails.elements as ie (ie.elementId + '-' + ie.dataHex)}
                <tr 
                  class="border-b border-gray-100 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-700 cursor-pointer"
                  class:selectedIE={selectedIE === ie}
                  on:click={() => selectedIE = ie}
                >
                  <td class="px-3 py-2">
                    <span class="px-1.5 py-0.5 rounded text-xs font-mono {getIEColor(ie.elementId)}">
                      {ie.elementIdHex}
                    </span>
                  </td>
                  <td class="px-3 py-2 text-gray-900 dark:text-gray-100">{ie.name}</td>
                  <td class="px-3 py-2 text-center text-gray-500 dark:text-gray-400">{ie.length}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        <!-- Right: IE Details -->
        <div class="w-1/2 overflow-auto">
          {#if selectedIE}
            <div class="p-4">
              <h3 class="font-semibold text-gray-900 dark:text-white mb-3">
                {selectedIE.name} <span class="text-gray-400 text-sm font-normal">(ID {selectedIE.elementIdHex})</span>
              </h3>

              <!-- Raw Hex -->
              <div class="mb-4">
                <label class="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wide">原始数据 (Hex)</label>
                <div class="mt-1 p-3 bg-gray-900 dark:bg-gray-950 rounded-lg overflow-auto">
                  <pre class="text-xs text-green-400 font-mono break-all">{selectedIE.dataHex}</pre>
                </div>
              </div>

              <!-- Parsed Fields -->
              {#if Object.keys(selectedIE.parsed).length > 0}
                <div>
                  <label class="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wide">解析结果</label>
                  <div class="mt-2 space-y-2">
                    {#each Object.entries(selectedIE.parsed) as [key, value]}
                      <div class="flex items-center justify-between p-2 bg-gray-50 dark:bg-gray-700 rounded">
                        <span class="text-gray-600 dark:text-gray-300 text-sm">{key}</span>
                        <span class="font-mono text-sm text-gray-900 dark:text-white">
                          {#if typeof value === 'boolean'}
                            <span class={value ? 'text-green-600 dark:text-green-400' : 'text-red-500 dark:text-red-400'}>
                              {value ? '✓' : '✗'}
                            </span>
                          {:else}
                            {value}
                          {/if}
                        </span>
                      </div>
                    {/each}
                  </div>
                </div>
              {/if}
            </div>
          {:else}
            <div class="h-full flex items-center justify-center text-gray-400 dark:text-gray-500">
              <div class="text-center">
                <svg class="w-16 h-16 mx-auto mb-4 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                </svg>
                <p>点击左侧 IE 元素查看详情</p>
              </div>
            </div>
          {/if}
        </div>
      </div>

      <!-- Footer: Raw Data -->
      <div class="border-t border-gray-200 dark:border-gray-700 p-3 bg-gray-50 dark:bg-gray-900">
        <details>
          <summary class="cursor-pointer text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white">
            完整 Beacon IE 数据 ({ieDetails.totalLength} bytes)
          </summary>
          <div class="mt-2 p-3 bg-gray-900 dark:bg-gray-950 rounded max-h-40 overflow-auto">
            <pre class="text-xs text-cyan-400 font-mono break-all">{ieDetails.rawHex}</pre>
          </div>
        </details>
      </div>
    {/if}
  </div>
</div>

<style>
  tr.selectedIE {
    background-color: rgb(238 242 255);
  }
  :global(.dark) tr.selectedIE {
    background-color: rgb(30 27 75 / 0.3);
  }
</style>
