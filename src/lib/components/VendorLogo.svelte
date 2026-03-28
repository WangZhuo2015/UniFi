<script lang="ts">
  import { cn } from '$lib/utils';
  import { getVendorBrand } from '$lib/vendors';

  export let vendor: string | null | undefined;
  export let size: 'sm' | 'md' | 'lg' = 'md';

  const sizeClass = {
    sm: 'h-7 w-7 text-[10px]',
    md: 'h-9 w-9 text-xs',
    lg: 'h-12 w-12 text-sm'
  };

  $: brand = getVendorBrand(vendor);
</script>

<div
  class={cn(
    'inline-flex items-center justify-center rounded-xl bg-gradient-to-br font-semibold shadow-sm ring-1',
    sizeClass[size],
    brand.bgClass,
    brand.ringClass
  )}
  aria-label={brand.name}
  title={brand.name}
>
  {brand.monogram}
</div>
