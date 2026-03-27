<script lang="ts">
  import { onMount } from 'svelte';
  import type { SignalPoint } from '$lib/types';

  interface Props {
    data: SignalPoint[];
    width?: number;
    height?: number;
  }

  let { data, width = 600, height = 200 }: Props = $props();

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D | null = null;

  const padding = { top: 20, right: 20, bottom: 30, left: 50 };

  function draw() {
    if (!ctx || data.length < 2) return;

    const chartWidth = width - padding.left - padding.right;
    const chartHeight = height - padding.top - padding.bottom;
    const context = ctx; // Local reference for TypeScript

    // Clear
    context.clearRect(0, 0, width, height);

    // Data range
    const signals = data.map(d => d.signal);
    const minSignal = Math.min(...signals) - 5;
    const maxSignal = Math.max(...signals) + 5;
    const timeRange = data[data.length - 1].time - data[0].time;

    // Grid
    context.strokeStyle = '#e5e7eb';
    context.lineWidth = 1;

    // Horizontal lines
    for (let i = 0; i <= 4; i++) {
      const y = padding.top + (chartHeight / 4) * i;
      context.beginPath();
      context.moveTo(padding.left, y);
      context.lineTo(width - padding.right, y);
      context.stroke();

      // Y-axis labels
      const value = Math.round(maxSignal - (maxSignal - minSignal) * (i / 4));
      context.fillStyle = '#6b7280';
      context.font = '12px sans-serif';
      context.textAlign = 'right';
      context.fillText(`${value}`, padding.left - 10, y + 4);
    }

    // Draw line
    context.strokeStyle = '#3b82f6';
    context.lineWidth = 2;
    context.beginPath();

    data.forEach((point, i) => {
      const x = padding.left + ((point.time - data[0].time) / timeRange) * chartWidth;
      const y = padding.top + ((maxSignal - point.signal) / (maxSignal - minSignal)) * chartHeight;

      if (i === 0) {
        context.moveTo(x, y);
      } else {
        context.lineTo(x, y);
      }
    });

    context.stroke();

    // Current point
    const last = data[data.length - 1];
    const lastX = width - padding.right;
    const lastY = padding.top + ((maxSignal - last.signal) / (maxSignal - minSignal)) * chartHeight;

    context.fillStyle = '#3b82f6';
    context.beginPath();
    context.arc(lastX, lastY, 4, 0, Math.PI * 2);
    context.fill();
  }

  $effect(() => {
    if (data && ctx) {
      draw();
    }
  });

  onMount(() => {
    ctx = canvas.getContext('2d');
    if (ctx) draw();
  });
</script>

<canvas
  bind:this={canvas}
  {width}
  {height}
  class="border rounded-lg dark:border-gray-700"
></canvas>