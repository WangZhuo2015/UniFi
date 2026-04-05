<script lang="ts">
  import Button from "./button.svelte";
  import { onMount, onDestroy } from 'svelte';

  let {
    label,
    disabled = false,
    open = $bindable(false),
    children
  }: {
    label: string;
    disabled?: boolean;
    open?: boolean;
    children?: import('svelte').Snippet;
  } = $props();

  let containerEl: HTMLDivElement | undefined = $state();

  function toggle(e: MouseEvent) {
    e.stopPropagation();
    open = !open;
  }

  function close() {
    open = false;
  }

  function handleClickOutside(e: MouseEvent) {
    if (containerEl && !containerEl.contains(e.target as Node)) {
      close();
    }
  }

  onMount(() => {
    document.addEventListener('click', handleClickOutside);
  });

  onDestroy(() => {
    document.removeEventListener('click', handleClickOutside);
  });
</script>

<div class="relative inline-block" bind:this={containerEl}>
  <Button
    variant="ghost"
    size="sm"
    {disabled}
    onclick={toggle}
  >
    {label}
    <svg class="ml-1 h-3 w-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </Button>
  {#if open}
    <div
      class="absolute right-0 top-full z-[9999] mt-1 min-w-40 rounded-lg border border-gray-200/80 bg-white py-1 shadow-xl dark:border-gray-700/80 dark:bg-gray-800"
    >
      {#if children}
        {@render children()}
      {/if}
    </div>
  {/if}
</div>