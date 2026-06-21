<script lang="ts">
  import { onMount } from 'svelte';
  import { graph as graphApi } from '../ts/ipc';
  import type { GraphData } from '../ts/types';

  interface Props {
    namespace: string | null;
    onmemoryclick: (id: string) => void;
  }

  let { namespace, onmemoryclick }: Props = $props();

  let data = $state<GraphData | null>(null);
  let loading = $state(true);

  onMount(async () => {
    await loadData();
  });

  async function loadData() {
    loading = true;
    try {
      data = await graphApi.getData({ namespace: namespace ?? undefined, limit: 200 });
    } catch {
      // not initialized
    } finally {
      loading = false;
    }
  }

  // Reload when namespace changes
  $effect(() => {
    namespace;
    loadData();
  });

  // Simple canvas force-directed graph (Phase 1 minimal)
  let canvas: HTMLCanvasElement;
  let width = $state(800);
  let height = $state(600);
  let nodes = $state<Map<string, { x: number; y: number; vx: number; vy: number }>>(new Map());
  let animFrame: number;

  $effect(() => {
    if (!data || data.nodes.length === 0 || !canvas) return;

    // Init node positions
    const newNodes = new Map();
    for (const node of data.nodes) {
      newNodes.set(node.id, {
        x: width / 2 + (Math.random() - 0.5) * 200,
        y: height / 2 + (Math.random() - 0.5) * 200,
        vx: 0,
        vy: 0,
      });
    }
    nodes = newNodes;
    startSimulation();
  });

  function startSimulation() {
    if (animFrame) cancelAnimationFrame(animFrame);

    function tick() {
      if (!data) return;
      const ctx = canvas.getContext('2d');
      if (!ctx) return;

      ctx.clearRect(0, 0, width, height);
      ctx.fillStyle = '#1e1e2e';
      ctx.fillRect(0, 0, width, height);

      // Force simulation
      const n = nodes;
      const nodeArr = Array.from(n.entries());

      // Repulsion
      for (let i = 0; i < nodeArr.length; i++) {
        for (let j = i + 1; j < nodeArr.length; j++) {
          const [idA, a] = nodeArr[i];
          const [idB, b] = nodeArr[j];
          const dx = b.x - a.x;
          const dy = b.y - a.y;
          const dist = Math.sqrt(dx * dx + dy * dy) || 1;
          const force = 800 / (dist * dist);
          const fx = (dx / dist) * force;
          const fy = (dy / dist) * force;
          a.vx -= fx;
          a.vy -= fy;
          b.vx += fx;
          b.vy += fy;
        }
      }

      // Attraction (edges)
      for (const edge of data.edges) {
        const a = n.get(edge.source);
        const b = n.get(edge.target);
        if (!a || !b) continue;
        const dx = b.x - a.x;
        const dy = b.y - a.y;
        const dist = Math.sqrt(dx * dx + dy * dy) || 1;
        const force = (dist - 100) * 0.01;
        const fx = (dx / dist) * force;
        const fy = (dy / dist) * force;
        a.vx += fx;
        a.vy += fy;
        b.vx -= fx;
        b.vy -= fy;
      }

      // Center gravity + damping + apply
      for (const [, pos] of n) {
        pos.vx += (width / 2 - pos.x) * 0.0005;
        pos.vy += (height / 2 - pos.y) * 0.0005;
        pos.vx *= 0.9;
        pos.vy *= 0.9;
        pos.x += pos.vx;
        pos.y += pos.vy;
      }

      // Draw edges
      ctx.strokeStyle = '#313244';
      ctx.lineWidth = 1;
      for (const edge of data.edges) {
        const a = n.get(edge.source);
        const b = n.get(edge.target);
        if (!a || !b) continue;
        ctx.beginPath();
        ctx.moveTo(a.x, a.y);
        ctx.lineTo(b.x, b.y);
        ctx.stroke();
      }

      // Draw nodes
      for (const node of data.nodes) {
        const pos = n.get(node.id);
        if (!pos) continue;
        ctx.fillStyle = '#89b4fa';
        ctx.beginPath();
        ctx.arc(pos.x, pos.y, 4, 0, Math.PI * 2);
        ctx.fill();

        // Label (first 20 chars)
        ctx.fillStyle = '#6c7086';
      }

      animFrame = requestAnimationFrame(tick);
    }

    tick();
  }

  function handleCanvasClick(e: MouseEvent) {
    if (!data || !canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    for (const node of data.nodes) {
      const pos = nodes.get(node.id);
      if (!pos) continue;
      const dx = pos.x - x;
      const dy = pos.y - y;
      if (dx * dx + dy * dy < 100) {
        onmemoryclick(node.id);
        return;
      }
    }
  }

  function handleResize() {
    if (!canvas) return;
    const parent = canvas.parentElement;
    if (!parent) return;
    width = parent.clientWidth;
    height = parent.clientHeight;
    canvas.width = width;
    canvas.height = height;
  }

  onMount(() => {
    handleResize();
    window.addEventListener('resize', handleResize);
    return () => {
      window.removeEventListener('resize', handleResize);
      if (animFrame) cancelAnimationFrame(animFrame);
    };
  });
</script>

<div class="graph-view">
  {#if loading}
    <div class="loading">Loading graph...</div>
  {:else if !data || data.nodes.length === 0}
    <div class="empty">
      <p>No memories to visualize yet.</p>
      <p>Create memories and add edges to see the knowledge graph.</p>
    </div>
  {:else}
    <div class="graph-stats">
      <span>{data.nodes.length} nodes</span>
      <span>{data.edges.length} edges</span>
    </div>
    <div class="canvas-container">
      <canvas
        bind:this={canvas}
        width={width}
        height={height}
        onclick={handleCanvasClick}
      ></canvas>
    </div>
  {/if}
</div>

<style>
  .graph-view {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .graph-stats {
    padding: 8px 24px;
    display: flex;
    gap: 16px;
    font-size: 0.8rem;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
  }

  .canvas-container {
    flex: 1;
    position: relative;
    overflow: hidden;
  }

  canvas {
    display: block;
    cursor: pointer;
  }

  .loading,
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    text-align: center;
    gap: 8px;
  }
</style>
