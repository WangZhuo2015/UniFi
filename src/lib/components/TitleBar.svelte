<script lang="ts">
  import { onMount } from 'svelte';
  import { browser } from '$app/environment';

  let isMaximized = $state(false);
  let isFocused = $state(true);
  let isTauri = $state(false);

  onMount(async () => {
    // Check if running in Tauri
    isTauri = browser && !!(window as any).__TAURI__;

    if (isTauri) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      const appWindow = getCurrentWindow();

      isMaximized = await appWindow.isMaximized();

      await appWindow.onResized(async () => {
        isMaximized = await appWindow.isMaximized();
      });

      await appWindow.onFocusChanged(({ payload }) => {
        isFocused = payload;
      });
    }
  });

  async function minimize() {
    if (isTauri) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().minimize();
    }
  }

  async function toggleMaximize() {
    if (isTauri) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().toggleMaximize();
      isMaximized = !isMaximized;
    }
  }

  async function close() {
    if (isTauri) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().close();
    }
  }
</script>

<header
  class="drag-region h-11 flex items-center justify-between px-3 bg-white/80 dark:bg-gray-900/80 backdrop-blur-xl border-b border-gray-200/50 dark:border-gray-700/50 transition-all duration-200"
  class:opacity-80={!isFocused}
  data-testid="title-bar"
>
  <!-- Left: App Icon & Title -->
  <div class="flex items-center gap-2 no-drag">
    <div class="w-5 h-5 rounded-lg bg-gradient-to-br from-blue-500 to-indigo-600 flex items-center justify-center shadow-sm">
      <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
      </svg>
    </div>
    <h1 class="text-sm font-semibold text-gray-800 dark:text-gray-200" data-testid="app-title">UniFi</h1>
  </div>

  <!-- Center: Traffic lights spacer for macOS -->
  <div class="absolute left-3 top-1/2 -translate-y-1/2 w-16 flex items-center gap-2 md:hidden">
    <!-- macOS traffic lights area -->
  </div>

  <!-- Right: Window Controls (only in Tauri) -->
  {#if isTauri}
    <div class="flex items-center gap-1 no-drag">
      <button
        class="w-7 h-7 flex items-center justify-center rounded-md hover:bg-gray-200/80 dark:hover:bg-gray-700/80 transition-colors duration-150"
        onclick={minimize}
        aria-label="Minimize"
      >
        <svg class="w-3.5 h-3.5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4" />
        </svg>
      </button>
      <button
        class="w-7 h-7 flex items-center justify-center rounded-md hover:bg-gray-200/80 dark:hover:bg-gray-700/80 transition-colors duration-150"
        onclick={toggleMaximize}
        aria-label="Maximize"
      >
        {#if isMaximized}
          <svg class="w-3 h-3.5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 4H6a2 2 0 00-2 2v2m0 8v2a2 2 0 002 2h2m8-16h2a2 2 0 012 2v2m0 8v2a2 2 0 01-2 2h-2" />
          </svg>
        {:else}
          <svg class="w-3.5 h-3.5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" />
          </svg>
        {/if}
      </button>
      <button
        class="w-7 h-7 flex items-center justify-center rounded-md hover:bg-red-500 hover:text-white transition-colors duration-150 group"
        onclick={close}
        aria-label="Close"
      >
        <svg class="w-3.5 h-3.5 text-gray-600 dark:text-gray-400 group-hover:text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>
  {:else}
    <div class="w-24"></div>
  {/if}
</header>
