<script lang="ts">
  import { onMount } from 'svelte';
  import { graph as graphApi, uteke, utekeServer } from '../ts/ipc';
  import type { GraphData } from '../ts/types';

  interface Props {
    onmemoryclick: (id: string) => void;
  }

  let { onmemoryclick }: Props = $props();

  let canvasEl = $state<HTMLCanvasElement | null>(null);
  let loading = $state(true);
  let serverOnline = $state(false);
  let hoveredNode = $state<string | null>(null);
  let expandingNode = $state<string | null>(null);
  let totalNodesShown = $state(0);
  let totalEdgesShown = $state(0);

  // Simulation state
  let W = 800;
  let H = 600;

  interface SimNode {
    id: string;
    x: number; y: number; vx: number; vy: number;
    label: string;
    tags: string[];
    conns: number;
    expanded: boolean;      // has been expanded by user click?
    color: string;
  }

  interface SimEdge {
    source: string;
    target: string;
    weight: number;
  }

  let nodes: SimNode[] = [];
  let edges: SimEdge[] = [];
  let nodeId = new Map<string, number>();
  let expandedSet = new Set<string>();       // node IDs already expanded
  let knownSet = new Set<string>();           // node IDs in the graph
  let edgeSet = new Set<string>();            // "a|b" sorted pairs to dedup edges
  let raf = 0;
  let physicsActive = true;
  let calmFrames = 0;
  let needRedraw = true;

  const INITIAL_SEED = 30;
  const EXPAND_LIMIT = 5;
  const COLORS = ['#89b4fa', '#a6e3a1', '#f9e2af', '#f38ba8', '#fab387', '#cba6f7', '#94e2d5', '#f5c2e7', '#89dceb', '#eba0ac'];

  function pickColor(tags: string[]): string {
    if (!tags?.length) return '#6c7086';
    return COLORS[(tags[0].charCodeAt(0) || 0) % COLORS.length];
  }

  // ─── Initial seed: fetch recent memories ───────────────────────────
  async function loadSeed() {
    loading = true;
    try {
      // Check server status
      try {
        const status = await utekeServer.status();
        serverOnline = status.available;
      } catch {
        serverOnline = false;
      }

      let seedMemories: { id: string; content: string; tags: string[] }[] = [];

      if (serverOnline) {
        // Fetch recent memories via server /list
        try {
          const serverList = await utekeServer.recall('knowledge memory code project idea', { limit: INITIAL_SEED });
          seedMemories = serverList.map(m => ({
            id: m.id,
            content: m.content,
            tags: m.tags ?? [],
          }));
        } catch {
          // Fallback below
        }
      }

      // Fallback: use local graph API
      if (seedMemories.length === 0) {
        try {
          const utekeReady = await uteke.available();
          const g: GraphData = utekeReady
            ? await uteke.graph({ limit: INITIAL_SEED })
            : await graphApi.getData({ limit: INITIAL_SEED });
          seedMemories = g.nodes.map(n => ({
            id: n.id,
            content: n.content,
            tags: n.tags ?? [],
          }));
          // Also seed edges from the graph data
          for (const e of g.edges) {
            addEdge(e.source, e.target, 0.5);
          }
        } catch {
          // No data at all
        }
      }

      // Add seed nodes
      for (const m of seedMemories.slice(0, INITIAL_SEED)) {
        addNode(m.id, m.content, m.tags);
      }
    } catch {
      // ignore
    }
    loading = false;
    physicsActive = true;
    calmFrames = 0;
    needRedraw = true;
    updateCounts();
  }

  // ─── Node / Edge management ────────────────────────────────────────
  function addNode(id: string, content: string, tags: string[]): boolean {
    if (knownSet.has(id)) return false;
    knownSet.add(id);
    const angle = (nodes.length / Math.max(1, nodes.length + 1)) * Math.PI * 2;
    const r = Math.min(W, H) * (0.15 + Math.random() * 0.15);
    const idx = nodes.length;
    nodeId.set(id, idx);
    nodes.push({
      id,
      x: W / 2 + Math.cos(angle) * r + (Math.random() - 0.5) * 40,
      y: H / 2 + Math.sin(angle) * r + (Math.random() - 0.5) * 40,
      vx: 0, vy: 0,
      label: content.slice(0, 30),
      tags: tags ?? [],
      conns: 0,
      expanded: false,
      color: pickColor(tags),
    });
    return true;
  }

  function addEdge(source: string, target: string, weight: number): boolean {
    if (source === target) return false;
    const key = source < target ? `${source}|${target}` : `${target}|${source}`;
    if (edgeSet.has(key)) return false;
    edgeSet.add(key);
    edges.push({ source, target, weight });

    // Update conn count
    const si = nodeId.get(source);
    const ti = nodeId.get(target);
    if (si !== undefined) nodes[si].conns++;
    if (ti !== undefined) nodes[ti].conns++;
    return true;
  }

  // ─── Expand: single click on a node ────────────────────────────────
  async function expandNode(id: string) {
    if (expandedSet.has(id)) return;  // already expanded
    expandingNode = id;
    needRedraw = true;

    try {
      const neighbors = await uteke.neighbors(id, EXPAND_LIMIT);
      const si = nodeId.get(id);
      if (si !== undefined) nodes[si].expanded = true;
      expandedSet.add(id);

      let added = 0;
      for (const nb of neighbors) {
        const isNew = addNode(nb.id, nb.content, nb.tags);
        addEdge(id, nb.id, nb.score ?? 0.5);
        if (isNew) added++;
      }

      if (added > 0 || neighbors.length > 0) {
        physicsActive = true;
        calmFrames = 0;
        needRedraw = true;
      }
      updateCounts();
    } catch {
      // neighbors not available — mark as expanded anyway
      expandedSet.add(id);
      const si = nodeId.get(id);
      if (si !== undefined) nodes[si].expanded = true;
    }

    expandingNode = null;
    needRedraw = true;
  }

  function updateCounts() {
    totalNodesShown = nodes.length;
    totalEdgesShown = edges.length;
  }

  // ─── Drawing ───────────────────────────────────────────────────────
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
      ctx.strokeStyle = hi ? 'rgba(137,180,250,1)' : 'rgba(137,180,250,0.45)';
      ctx.lineWidth = hi ? 3 : 1.5;
      ctx.beginPath();
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(b.x, b.y);
      ctx.stroke();
    }

    // Draw nodes
    for (const n of nodes) {
      const hi = hoveredNode === n.id;
      const isExpanding = expandingNode === n.id;
      const r = Math.max(3, Math.min(10, 3 + n.conns * 0.8));

      // Expansion ring for expanded nodes
      if (n.expanded) {
        ctx.beginPath();
        ctx.arc(n.x, n.y, r + 4, 0, 6.283);
        ctx.strokeStyle = n.color + '55';
        ctx.lineWidth = 1.5;
        ctx.stroke();
      }

      // Hover halo
      if (hi || isExpanding) {
        ctx.beginPath();
        ctx.arc(n.x, n.y, r + 6, 0, 6.283);
        ctx.fillStyle = n.color + '22';
        ctx.fill();
      }

      // Pulsing indicator for nodes that haven't been expanded (clickable)
      if (!n.expanded && !isExpanding) {
        const pulse = 0.5 + 0.5 * Math.sin(Date.now() / 600 + n.id.charCodeAt(0));
        ctx.beginPath();
        ctx.arc(n.x, n.y, r + 2 + pulse * 2, 0, 6.283);
        ctx.strokeStyle = `rgba(137,180,250,${0.1 + pulse * 0.15})`;
        ctx.lineWidth = 1;
        ctx.stroke();
      }

      // Node circle
      ctx.beginPath();
      ctx.arc(n.x, n.y, r, 0, 6.283);
      ctx.fillStyle = hi ? '#cdd6f4' : n.color;
      ctx.fill();

      // Label: show on hover, or for highly-connected nodes
      if (hi || n.conns >= 3) {
        ctx.font = '11px -apple-system, sans-serif';
        ctx.fillStyle = hi ? '#cdd6f4' : '#6c7086';
        ctx.textAlign = 'center';
        // Truncate long labels
        const label = n.label.length > 30 ? n.label.slice(0, 27) + '…' : n.label;
        ctx.fillText(label, n.x, n.y - r - 5);
      }

      // Expanding spinner
      if (isExpanding) {
        ctx.font = '10px -apple-system, sans-serif';
        ctx.fillStyle = '#89b4fa';
        ctx.textAlign = 'center';
        ctx.fillText('…', n.x, n.y - r - 5);
      }
    }
  }

  // ─── Physics ───────────────────────────────────────────────────────
  function tick() {
    const canvas = canvasEl;
    if (!canvas) { raf = requestAnimationFrame(tick); return; }
    if (canvas.width !== W) canvas.width = W;
    if (canvas.height !== H) canvas.height = H;
    const ctx = canvas.getContext('2d');
    if (!ctx) { raf = requestAnimationFrame(tick); return; }

    if (physicsActive && nodes.length > 0) {
      let totalV = 0;

      // Repulsion (O(n²) but fine for <200 nodes)
      for (let i = 0; i < nodes.length; i++) {
        for (let j = i + 1; j < nodes.length; j++) {
          const a = nodes[i], b = nodes[j];
          let dx = b.x - a.x, dy = b.y - a.y;
          let d2 = dx * dx + dy * dy;
          if (d2 < 1) { d2 = 1; dx = Math.random(); dy = Math.random(); }
          const d = Math.sqrt(d2);
          const f = 1800 / d2;
          a.vx -= (dx / d) * f; a.vy -= (dy / d) * f;
          b.vx += (dx / d) * f; b.vy += (dy / d) * f;
        }
      }

      // Spring (edges)
      for (const e of edges) {
        const ai = nodeId.get(e.source), bi = nodeId.get(e.target);
        if (ai === undefined || bi === undefined) continue;
        const a = nodes[ai], b = nodes[bi];
        const dx = b.x - a.x, dy = b.y - a.y;
        const d = Math.sqrt(dx * dx + dy * dy) || 1;
        const f = (d - 100) * 0.03;
        a.vx += (dx / d) * f; a.vy += (dy / d) * f;
        b.vx -= (dx / d) * f; b.vy -= (dy / d) * f;
      }

      // Apply velocity + damping + center gravity
      for (const p of nodes) {
        p.vx += (W / 2 - p.x) * 0.002;
        p.vy += (H / 2 - p.y) * 0.002;
        p.vx *= 0.72;
        p.vy *= 0.72;
        totalV += Math.abs(p.vx) + Math.abs(p.vy);
        p.x = Math.max(15, Math.min(W - 15, p.x + p.vx));
        p.y = Math.max(15, Math.min(H - 15, p.y + p.vy));
      }

      // Settle check
      const settleThreshold = Math.max(1.5, nodes.length * 0.15);
      if (totalV < settleThreshold) {
        calmFrames++;
        if (calmFrames > 10) {
          physicsActive = false;
          needRedraw = true;
        }
      } else {
        calmFrames = 0;
      }
      needRedraw = true;
    }

    // Always redraw if there are un-expanded nodes (they have pulse animation)
    const hasPulse = nodes.some(n => !n.expanded);
    if (needRedraw || hasPulse) {
      draw(ctx);
      if (!physicsActive) needRedraw = false;
    }

    raf = requestAnimationFrame(tick);
  }

  // ─── Mouse interaction ─────────────────────────────────────────────
  let clickTimer: ReturnType<typeof setTimeout> | null = null;
  const DOUBLE_CLICK_MS = 280;

  function getCanvasPoint(e: MouseEvent): { x: number; y: number } | null {
    const canvas = canvasEl;
    if (!canvas) return null;
    const rect = canvas.getBoundingClientRect();
    const sx = canvas.width / rect.width;
    const sy = canvas.height / rect.height;
    return {
      x: (e.clientX - rect.left) * sx,
      y: (e.clientY - rect.top) * sy,
    };
  }

  function findNodeAt(x: number, y: number): string | null {
    for (let i = nodes.length - 1; i >= 0; i--) {
      const n = nodes[i];
      const r = Math.max(3, Math.min(10, 3 + n.conns * 0.8));
      if ((n.x - x) ** 2 + (n.y - y) ** 2 < (r + 4) ** 2) {
        return n.id;
      }
    }
    return null;
  }

  function onMouseMove(e: MouseEvent) {
    const pt = getCanvasPoint(e);
    if (!pt) return;
    const prev = hoveredNode;
    hoveredNode = findNodeAt(pt.x, pt.y);
    if (prev !== hoveredNode) needRedraw = true;
    const canvas = canvasEl;
    if (canvas) canvas.style.cursor = hoveredNode ? 'pointer' : 'default';
  }

  function onCanvasClick(e: MouseEvent) {
    const pt = getCanvasPoint(e);
    if (!pt) return;
    const id = findNodeAt(pt.x, pt.y);
    if (!id) return;

    // Double-click detection
    if (clickTimer) {
      // This is a double-click → open detail
      clearTimeout(clickTimer);
      clickTimer = null;
      onmemoryclick(id);
      return;
    }

    // Single click — wait to see if it becomes a double-click
    clickTimer = setTimeout(() => {
      clickTimer = null;
      expandNode(id);
    }, DOUBLE_CLICK_MS);
  }

  // ─── Lifecycle ─────────────────────────────────────────────────────
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
            physicsActive = true;
            calmFrames = 0;
            needRedraw = true;
          }
        }
      });
      ro.observe(canvas.parentElement);

      raf = requestAnimationFrame(tick);
      loadSeed();
      return () => { ro.disconnect(); cancelAnimationFrame(raf); };
    }
  });
</script>

<div class="graph-view">
  {#if !loading && totalNodesShown > 0}
    <div class="graph-toolbar">
      <span class="graph-info">{totalNodesShown} nodes · {totalEdgesShown} edges</span>
      {#if serverOnline}
        <span class="mode-tag semantic">Semantic</span>
      {:else}
        <span class="mode-tag local">Local</span>
      {/if}
      <span class="hint-text">click to expand · double-click for detail</span>
    </div>
  {/if}
  <div class="canvas-wrap">
    <canvas
      bind:this={canvasEl}
      style="width:100%;height:100%;display:block;"
      onmousemove={onMouseMove}
      onclick={onCanvasClick}
      onmouseleave={() => { if (hoveredNode) { hoveredNode = null; needRedraw = true; } }}
    ></canvas>
    {#if loading}
      <div class="overlay">Loading graph...</div>
    {:else if totalNodesShown === 0}
      <div class="overlay"><p>No memories to visualize.</p></div>
    {/if}
  </div>
</div>

<style>
  .graph-view { height: 100%; display: flex; flex-direction: column; }
  .graph-toolbar { padding: 8px 16px; display: flex; gap: 10px; align-items: center; border-bottom: 1px solid var(--border); }
  .graph-info { font-size: 0.8rem; color: var(--text-muted); }
  .hint-text { font-size: 0.7rem; color: var(--text-muted); opacity: 0.6; margin-left: auto; }
  .mode-tag { font-size: 0.7rem; padding: 2px 8px; border-radius: 3px; font-weight: 600; }
  .mode-tag.semantic { background: rgba(166,227,161,0.15); color: var(--green); }
  .mode-tag.local { background: rgba(148,226,213,0.15); color: var(--teal); }
  .canvas-wrap { flex: 1; position: relative; overflow: hidden; }
  .overlay { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; color: var(--text-muted); pointer-events: none; }
</style>
