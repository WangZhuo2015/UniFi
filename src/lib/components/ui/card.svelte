<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { MouseEventHandler } from 'svelte/elements';

  interface Props {
    class?: string;
    onclick?: () => void;
    children?: Snippet;
  }

  let { class: className, onclick, children }: Props = $props();
</script>

{#if onclick}
  <button
    type="button"
    class={cn(
      'w-full text-left rounded-lg border bg-white dark:bg-gray-800 shadow-sm',
      'cursor-pointer hover:border-blue-400 transition-colors',
      className
    )}
    onclick={onclick}
  >
    {#if children}
      {@render children()}
    {/if}
  </button>
{:else}
  <div
    class={cn(
      'rounded-lg border bg-white dark:bg-gray-800 shadow-sm',
      className
    )}
  >
    {#if children}
      {@render children()}
    {/if}
  </div>
{/if}