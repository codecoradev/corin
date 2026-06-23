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

  // ── Client-side edge generation from shared tags ───────────────
  // Groups memories by tag and links every pair that shares a tag.
  // Caps per-tag connections to avoid clutter.
  function buildTagEdges(
    mems: { id: string; content: string; tags: string[] }[],
    maxPerTag: number,
  ): { source: string; target: string; weight: number }[] {
    const result: { source: string; target: string; weight: number }[] = [];
    const seen = new Set<string>();

    // Index: tag → list of memory ids that have it
    const tagMap = new Map<string, string[]>();
    for (const m of mems) {
      for (const t of m.tags ?? []) {
        const key = t.toLowerCase();
        if (!tagMap.has(key)) tagMap.set(key, []);
        tagMap.get(key)!.push(m.id);
      }
    }

    // For each tag with 2+ memories, connect pairs
    for (const [, ids] of tagMap) {
      if (ids.length < 2) continue;
      let count = 0;
      // Shuffle-ish: connect adjacent in array for determinism
      for (let i = 0; i < ids.length - 1 && count < maxPerTag; i++) {
        const a = ids[i];
        const b = ids[i + 1];
        const ek = a < b ? `${a}|${b}` : `${b}|${a}`;
        if (seen.has(ek)) continue;
        seen.add(ek);
        result.push({ source: a, target: b, weight: 0.6 });
        count++;
      }
    }

    // Fallback: if tags didn't produce enough edges (e.g. some memories
    // have no tags), link unconnected nodes to the nearest connected one
    // or into a chain so the graph is never fully disconnected.
    if (result.length < mems.length / 3 && mems.length > 1) {
      const connected = new Set<string>();
      for (const e of result) { connected.add(e.source); connected.add(e.target); }
      const unconnected = mems.filter(m => !connected.has(m.id));
      for (const m of unconnected) {
        // Link to a random connected node, or chain to previous unconnected
        const target = connected.size > 0
          ? [...connected][Math.floor(Math.random() * connected.size)]
          : mems[mems.indexOf(m) - 1]?.id;
        if (target && target !== m.id) {
          const ek = m.id < target ? `${m.id}|${target}` : `${target}|${m.id}`;
          if (!seen.has(ek)) {
            seen.add(ek);
            result.push({ source: m.id, target, weight: 0.3 });
          }
        }
      }
    }

    return result;
  }

  // ─── Initial seed: fetch recent memories ───────────────────────────
  async function loadSeed() {
    loading = true;
    // Reset graph state before repopulating (cora scan #30)
    nodes = [];
    edges = [];
    nodeId.clear();
    knownSet.clear();
    edgeSet.clear();
    expandedSet.clear();
    try {
      // Check server status
      try {
        const status = await utekeServer.status();
        serverOnline = status.available;
      } catch {
        serverOnline = false;
      }

      let seedMemories: { id: string; content: string; tags: string[] }[] = [];
      let seedEdges: { source: string; target: string; weight: number }[] = [];

      if (serverOnline) {
        // Fetch the server graph (nodes + edges) once.
        // The backend returns real cosine edges when available, or a
        // tag-based fallback graph so connections are always present.
        try {
          const sg = await utekeServer.graph();
          // Build the node pool from the server graph nodes. We only
          // seed INITIAL_SEED of them, but keep ALL edges that connect
          // the seeded nodes so lines appear on first paint.
          const serverNodeMap = new Map<string, { id: string; content: string; tags: string[] }>();
          for (const n of sg.nodes) {
            serverNodeMap.set(n.id, {
              id: n.id,
              content: n.label ?? n.id,
              tags: n.entity_type ? [n.entity_type] : [],
            });
          }
          seedMemories = [...serverNodeMap.values()];
          seedEdges = sg.edges.map(e => ({
            source: e.source,
            target: e.target,
            weight: e.weight ?? 0.5,
          }));

          // Enrich: the /graph endpoint only exposes entity_type (first tag).
          // Fetch full tags via /list so colors and labels are accurate.
          // Try all namespaces in parallel since memories may be spread.
          try {
            const namespaces = await uteke.namespaces();
            const tagMap = new Map<string, string[]>();
            const nsResults = await Promise.all(
              namespaces.slice(0, 12).map(ns =>
                uteke.list({ namespace: ns, limit: 50 }).catch(() => [])
              )
            );
            for (const list of nsResults) {
              for (const m of list) tagMap.set(m.id, m.tags ?? []);
            }
            for (const m of seedMemories) {
              const full = tagMap.get(m.id);
              if (full && full.length) m.tags = full;
            }
          } catch {
            // Tags enrichment is best-effort; entity_type is enough to draw.
          }
        } catch (err) {
          console.warn('[GraphView] utekeServer.graph() failed, trying recall()', err);
        }

        // Secondary fallback: recall() if graph endpoint returned nothing
        if (seedMemories.length === 0) {
          // The server may have memories in named namespaces (cto, hermes, etc.)
          // but /graph with no namespace returns 0. Try recalling across
          // all known namespaces.
          try {
            const namespaces = await uteke.namespaces();
            const nsResults = await Promise.all(
              namespaces.slice(0, 10).map(ns =>
                utekeServer.recall('knowledge memory code project idea', { namespace: ns, limit: 10 }).catch(() => [])
              )
            );
            for (const results of nsResults) {
              for (const m of results) {
                if (!seedMemories.some(s => s.id === m.id)) {
                  seedMemories.push({ id: m.id, content: m.content, tags: m.tags ?? [] });
                }
              }
            }
          } catch (err) {
            console.warn('[GraphView] namespace recall fallback failed', err);
          }

          // Last resort: recall without namespace
          if (seedMemories.length === 0) {
            try {
              const serverList = await utekeServer.recall('knowledge memory code project idea', { limit: INITIAL_SEED });
              seedMemories = serverList.map(m => ({
                id: m.id,
                content: m.content,
                tags: m.tags ?? [],
              }));
            } catch (err) {
              console.warn('[GraphView] recall() fallback failed', err);
            }
          }
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
          seedEdges = g.edges.map(e => ({
            source: e.source,
            target: e.target,
            weight: 0.5,
          }));
        } catch (err) {
          console.warn('[GraphView] local graph fallback failed', err);
        }
      }

      // ── Synthetic edge generation (client-side fallback) ──────────
      // If the backend returned no edges (cosine not yet computed,
      // server offline, etc.), derive edges from shared tags so the
      // graph is always visually connected on first paint.
      if (seedEdges.length === 0 && seedMemories.length > 1) {
        seedEdges = buildTagEdges(seedMemories, 3);
      }

      // Prioritise nodes that appear in edges so the seed graph is densely
      // connected on first paint. Nodes with no edges are still included (as
      // a tail) so isolated memories remain discoverable.
      const edgeDegree = new Map<string, number>();
      for (const e of seedEdges) {
        edgeDegree.set(e.source, (edgeDegree.get(e.source) ?? 0) + 1);
        edgeDegree.set(e.target, (edgeDegree.get(e.target) ?? 0) + 1);
      }
      const seedPool = [...seedMemories].sort((a, b) =>
        (edgeDegree.get(b.id) ?? 0) - (edgeDegree.get(a.id) ?? 0),
      );

      // Add seed nodes (only INITIAL_SEED become visible)
      const seededIds = new Set<string>();
      for (const m of seedPool.slice(0, INITIAL_SEED)) {
        if (addNode(m.id, m.content, m.tags)) seededIds.add(m.id);
      }

      // Add edges that connect the seeded nodes so lines appear on first paint.
      // Edges referencing not-yet-seeded nodes are skipped here (the count
      // stays accurate and they'll be discovered via expandNode later).
      for (const e of seedEdges) {
        if (seededIds.has(e.source) && seededIds.has(e.target)) {
          addEdge(e.source, e.target, e.weight);
        }
      }
    } catch (err) {
      console.error('[GraphView] loadSeed() failed', err);
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
