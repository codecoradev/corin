<script lang="ts">
  import { onMount } from 'svelte';
  import { graph as graphApi, uteke } from '../ts/ipc';
  import type { GraphData } from '../ts/types';

  interface Props {
    onmemoryclick: (id: string) => void;
  }

  let { onmemoryclick }: Props = $props();

  let canvasEl = $state<HTMLCanvasElement | null>(null);
  let data = $state<GraphData | null>(null);
  let loading = $state(true);
  let utekeReady = $state(false);
  let hoveredNode = $state<string | null>(null);

  // Simulation state — ALL plain variables, no $state
  let W = 800;
  let H = 600;
  let nodes: { id: string; x: number; y: number; vx: number; vy: number; label: string; tags: string[]; conns: number }[] = [];
  let edges: { source: string; target: string }[] = [];
  let nodeId = new Map<string, number>();
  let raf = 0;
  let physicsActive = true;
  let calmFrames = 0;
  let needRedraw = true; // flag for one-time redraw when settled

  const COLORS = ['#89b4fa', '#a6e3a1', '#f9e2af', '#f38ba8', '#fab387', '#cba6f7', '#94e2d5', '#f5c2e7', '#89dceb', '#eba0ac'];

  function color(tags: string[]): string {
    if (!tags?.length) return '#6c7086';
    return COLORS[(tags[0].charCodeAt(0) || 0) % COLORS.length];
  }

  async function loadData() {
    loading = true;
    try {
      utekeReady = await uteke.available();
      data = utekeReady
        ? await uteke.graph({ limit: 150 })
        : await graphApi.getData({ limit: 150 });
    } catch {
      data = null;
    }
    loading = false;
  }

  $effect(() => {
    if (!data) return;
    setupGraph();
  });

  function setupGraph() {
    if (!data || !data.nodes.length) {
      nodes = [];
      edges = [];
      return;
    }
    const d = data;
    const conns = new Map<string, number>();
    for (const e of d.edges) {
      conns.set(e.source, (conns.get(e.source) ?? 0) + 1);
      conns.set(e.target, (conns.get(e.target) ?? 0) + 1);
    }
    nodeId.clear();
    nodes = d.nodes.map((n, i) => {
      const angle = (i / d.nodes.length) * Math.PI * 2;
      const r = Math.min(W, H) * 0.3;
      nodeId.set(n.id, i);
      return {
        id: n.id,
        x: W / 2 + Math.cos(angle) * r + (Math.random() - 0.5) * 30,
        y: H / 2 + Math.sin(angle) * r + (Math.random() - 0.5) * 30,
        vx: 0, vy: 0,
        label: n.content.slice(0, 25),
        tags: n.tags,
        conns: conns.get(n.id) ?? 0,
      };
    });
    edges = d.edges.map(e => ({ source: e.source, target: e.target }));
    physicsActive = true;
    calmFrames = 0;
    needRedraw = true;
  }

  function draw(ctx: CanvasRenderingContext2D) {
    ctx.fillStyle = '#1e1e2e';
    ctx.fillRect(0, 0, W, H);

    if (!nodes.length) return;

    // Draw edges
    for (const e of edges) {
      const ai = nodeId.get(e.source), bi = nodeId.get(e.target);
      if (ai === undefined || bi === undefined) continue;
      const a = nodes[ai], b = nodes[bi];
      const hi = hoveredNode === a.id || hoveredNode === b.id;
      ctx.strokeStyle = hi ? 'rgba(137,180,250,0.5)' : 'rgba(49,50,68,0.3)';
      ctx.lineWidth = hi ? 1.5 : 0.8;
      ctx.beginPath();
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(b.x, b.y);
      ctx.stroke();
    }

    // Draw nodes
    for (const n of nodes) {
      const c = color(n.tags);
      const hi = hoveredNode === n.id;
      const r = Math.max(3, Math.min(9, 3 + n.conns * 0.7));

      if (hi) {
        ctx.beginPath();
        ctx.arc(n.x, n.y, r + 5, 0, 6.283);
        ctx.fillStyle = c + '33';
        ctx.fill();
      }
      ctx.beginPath();
      ctx.arc(n.x, n.y, r, 0, 6.283);
      ctx.fillStyle = hi ? '#cdd6f4' : c;
      ctx.fill();

      if (hi || n.conns >= 3) {
        ctx.font = '11px -apple-system, sans-serif';
        ctx.fillStyle = hi ? '#cdd6f4' : '#6c7086';
        ctx.textAlign = 'center';
        ctx.fillText(n.label, n.x, n.y - r - 4);
      }
    }
  }

  function tick() {
    const canvas = canvasEl;
    if (!canvas) { raf = requestAnimationFrame(tick); return; }
    if (canvas.width !== W) canvas.width = W;
    if (canvas.height !== H) canvas.height = H;
    const ctx = canvas.getContext('2d');
    if (!ctx) { raf = requestAnimationFrame(tick); return; }

    // Run physics only if active
    if (physicsActive && nodes.length > 0) {
      let totalV = 0;

      // Repulsion
      for (let i = 0; i < nodes.length; i++) {
        for (let j = i + 1; j < nodes.length; j++) {
          const a = nodes[i], b = nodes[j];
          let dx = b.x - a.x, dy = b.y - a.y;
          let d2 = dx * dx + dy * dy;
          if (d2 < 1) { d2 = 1; dx = Math.random(); dy = Math.random(); }
          const d = Math.sqrt(d2);
          const f = 1500 / d2;
          a.vx -= (dx / d) * f; a.vy -= (dy / d) * f;
          b.vx += (dx / d) * f; b.vy += (dy / d) * f;
        }
      }

      // Spring
      for (const e of edges) {
        const ai = nodeId.get(e.source), bi = nodeId.get(e.target);
        if (ai === undefined || bi === undefined) continue;
        const a = nodes[ai], b = nodes[bi];
        const dx = b.x - a.x, dy = b.y - a.y;
        const d = Math.sqrt(dx * dx + dy * dy) || 1;
        const f = (d - 90) * 0.025;
        a.vx += (dx / d) * f; a.vy += (dy / d) * f;
        b.vx -= (dx / d) * f; b.vy -= (dy / d) * f;
      }

      // Apply
      for (const p of nodes) {
        p.vx += (W / 2 - p.x) * 0.001;
        p.vy += (H / 2 - p.y) * 0.001;
        p.vx *= 0.8;
        p.vy *= 0.8;
        totalV += Math.abs(p.vx) + Math.abs(p.vy);
        p.x = Math.max(15, Math.min(W - 15, p.x + p.vx));
        p.y = Math.max(15, Math.min(H - 15, p.y + p.vy));
      }

      // Settle check
      if (totalV < 1.5) {
        calmFrames++;
        if (calmFrames > 20) {
          physicsActive = false; // STOP physics
          needRedraw = true;
          console.log('[Graph] physics settled');
        }
      } else {
        calmFrames = 0;
      }
      needRedraw = true; // always redraw while physics active
    }

    // Only redraw if needed (saves CPU when settled)
    if (needRedraw) {
      draw(ctx);
      needRedraw = false;
    }

    raf = requestAnimationFrame(tick);
  }

  function onMouseMove(e: MouseEvent) {
    const canvas = canvasEl;
    if (!canvas || !nodes.length) return;
    const rect = canvas.getBoundingClientRect();
    const sx = canvas.width / rect.width;
    const sy = canvas.height / rect.height;
    const x = (e.clientX - rect.left) * sx;
    const y = (e.clientY - rect.top) * sy;

    const prev = hoveredNode;
    hoveredNode = null;
    for (const n of nodes) {
      if ((n.x - x) ** 2 + (n.y - y) ** 2 < 200) {
        hoveredNode = n.id;
        break;
      }
    }
    if (prev !== hoveredNode) {
      needRedraw = true; // trigger redraw for highlight change
    }
    canvas.style.cursor = hoveredNode ? 'pointer' : 'default';
  }

  function onClick() {
    if (hoveredNode) onmemoryclick(hoveredNode);
  }

  onMount(() => {
    const canvas = canvasEl;
    if (canvas && canvas.parentElement) {
      W = canvas.parentElement.clientWidth || 800;
      H = canvas.parentElement.clientHeight || 600;

      const ro = new ResizeObserver(() => {
        const p = canvas.parentElement;
        if (p) {
          const nw = p.clientWidth, nh = p.clientHeight;
          if (nw > 0 && nh > 0 && (nw !== W || nh !== H)) {
            W = nw; H = nh;
            physicsActive = true; // re-simulate on resize
            calmFrames = 0;
            needRedraw = true;
          }
        }
      });
      ro.observe(canvas.parentElement);

      raf = requestAnimationFrame(tick);
      loadData();
      return () => { ro.disconnect(); cancelAnimationFrame(raf); };
    }
  });
</script>

<div class="graph-view">
  {#if !loading && data && data.nodes.length > 0}
    <div class="graph-toolbar">
      <span class="graph-info">{data.nodes.length} nodes · {data.edges.length} edges</span>
      {#if utekeReady}<span class="uteke-tag">Uteke</span>{/if}
    </div>
  {/if}
  <div class="canvas-wrap">
    <canvas
      bind:this={canvasEl}
      style="width:100%;height:100%;display:block;"
      onmousemove={onMouseMove}
      onclick={onClick}
      onmouseleave={() => { if (hoveredNode) { hoveredNode = null; needRedraw = true; } }}
    ></canvas>
    {#if loading}
      <div class="overlay">Loading graph...</div>
    {:else if !data || data.nodes.length === 0}
      <div class="overlay"><p>No memories to visualize.</p></div>
    {/if}
  </div>
</div>

<style>
  .graph-view { height: 100%; display: flex; flex-direction: column; }
  .graph-toolbar { padding: 8px 16px; display: flex; gap: 12px; border-bottom: 1px solid var(--border); }
  .graph-info { font-size: 0.8rem; color: var(--text-muted); }
  .uteke-tag { font-size: 0.7rem; padding: 2px 8px; background: rgba(148,226,213,0.15); color: var(--teal); border-radius: 3px; font-weight: 600; }
  .canvas-wrap { flex: 1; position: relative; overflow: hidden; }
  .overlay { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; color: var(--text-muted); pointer-events: none; }
</style>
