<script lang="ts">
  import { onMount } from 'svelte';
  import { locale, setLocale, t } from '$lib/i18n';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { Window } from '@tauri-apps/api/window';

  let isMaximized = $state(false);
  let isFocused = $state(true);
  let isTauri = $state(false);
  let appWindow: Window | null = $state(null);

  function toggleLanguage() {
    setLocale($locale === 'zh-CN' ? 'en-US' : 'zh-CN');
  }

  onMount(() => {
    let unlistenResize: (() => void) | undefined;
    let unlistenFocus: (() => void) | undefined;

    void (async () => {
      try {
        appWindow = getCurrentWindow();
        isTauri = true;
        isMaximized = await appWindow.isMaximized();

        unlistenResize = await appWindow.onResized(async () => {
          if (appWindow) {
            isMaximized = await appWindow.isMaximized();
          }
        });

        unlistenFocus = await appWindow.onFocusChanged(({ payload }) => {
          isFocused = payload;
        });
      } catch (e) {
        console.error('TitleBar: Failed to get Tauri window:', e);
        isTauri = false;
        appWindow = null;
      }
    })();

    return () => {
      unlistenResize?.();
      unlistenFocus?.();
    };
  });

  async function minimize() {
    if (appWindow) {
      await appWindow.minimize();
    }
  }

  async function toggleMaximize() {
    if (appWindow) {
      await appWindow.toggleMaximize();
      isMaximized = await appWindow.isMaximized();
    }
  }

  async function close() {
    if (appWindow) {
      await appWindow.close();
    }
  }
</script>

<header
  data-tauri-drag-region
  class="flex h-11 items-center justify-between border-b border-gray-200/50 bg-white/80 px-3 backdrop-blur-xl transition-all duration-200 dark:border-gray-700/50 dark:bg-gray-900/80"
  class:opacity-80={!isFocused}
  data-testid="title-bar"
>
  <!-- Drag region: Logo and title -->
  <div class="flex min-w-0 cursor-default items-center gap-2">
    <div class="pointer-events-none flex h-5 w-5 items-center justify-center rounded-lg bg-gradient-to-br from-blue-500 to-indigo-600 shadow-sm">
      <svg class="h-3 w-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
      </svg>
    </div>
    <h1 class="pointer-events-none truncate text-sm font-semibold text-gray-800 dark:text-gray-200" data-testid="app-title">UniFi</h1>
  </div>

  <!-- Drag region: Empty space in middle -->
  <div class="mx-3 min-w-0 flex-1 cursor-default"></div>

  <!-- Buttons: No drag region here - use pointer-events to prevent blocking -->
  <div class="flex items-center gap-1" style="pointer-events: auto;">
    <button
      type="button"
      class="h-7 rounded-md px-2 text-xs text-gray-600 transition-colors duration-150 hover:bg-gray-200/80 dark:text-gray-400 dark:hover:bg-gray-700/80"
      onclick={toggleLanguage}
      aria-label={t($locale, 'languageLabel')}
    >
      {$locale === 'zh-CN' ? t($locale, 'languageEn') : t($locale, 'languageZh')}
    </button>

    {#if isTauri}
      <button
        type="button"
        class="flex h-7 w-7 items-center justify-center rounded-md transition-colors duration-150 hover:bg-gray-200/80 dark:hover:bg-gray-700/80"
        onclick={minimize}
        aria-label={t($locale, 'minimize')}
      >
        <svg class="h-3.5 w-3.5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4" />
        </svg>
      </button>
      <button
        type="button"
        class="flex h-7 w-7 items-center justify-center rounded-md transition-colors duration-150 hover:bg-gray-200/80 dark:hover:bg-gray-700/80"
        onclick={toggleMaximize}
        aria-label={t($locale, 'maximize')}
      >
        {#if isMaximized}
          <svg class="h-3.5 w-3.5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 4H6a2 2 0 00-2 2v2m0 8v2a2 2 0 002 2h2m8-16h2a2 2 0 012 2v2m0 8v2a2 2 0 01-2 2h-2" />
          </svg>
        {:else}
          <svg class="h-3.5 w-3.5 text-gray-600 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" />
          </svg>
        {/if}
      </button>
      <button
        type="button"
        class="group flex h-7 w-7 items-center justify-center rounded-md transition-colors duration-150 hover:bg-red-500 hover:text-white"
        onclick={close}
        aria-label={t($locale, 'close')}
      >
        <svg class="h-3.5 w-3.5 text-gray-600 group-hover:text-white dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    {/if}
  </div>
</header>