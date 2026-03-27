<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';

  interface Props {
    value: string;
    onchange?: (value: string) => void;
    class?: string;
    children?: Snippet;
  }

  let { value = $bindable(), onchange, class: className, children }: Props = $props();

  function selectTab(tab: string) {
    value = tab;
    onchange?.(tab);
  }
</script>

<div class={cn('w-full', className)}>
  {#if children}
    {@render children()}
  {/if}
</div>