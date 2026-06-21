<script lang="ts">
  import { onMount } from 'svelte';
  import { graph as graphApi, uteke } from '../ts/ipc';
  import type { GraphData, MemoryEntry } from '../ts/types';

  interface Props {
    namespace: string | null;
    onmemoryclick: (id: string) => void;
  }

  let { namespace, onmemoryclick }: Props = $props();

  let canvas: HTMLCanvasElement | null = $state(null);
  let data = $state<GraphData | null>(null);
  let loading = $state(true);
  let utekeReady = $state(false);
  let hoveredNode = $state<string | null>(null);

  // Simulation state
  let width = $state(800);
  let height = $state(600);
  let animFrame = 0;

  interface NodePos {
    x: number;
    y: number;
    vx: number;
    vy: number;
    id: string;
    label: string;
    tags: string[];
    namespace: string | null;
    connections: number;
  }

  let nodes = $state<Map<string, NodePos>>(new Map());

  async function loadData() {
    loading = true;
    try {
      utekeReady = await uteke.available();
      if (utekeReady) {
        data = await uteke.graph({ namespace: namespace ?? undefined, limit: 150 });
      } else {
        data = await graphApi.getData({ namespace: namespace ?? undefined, limit: 150 });
      }
    } catch {
      data = null;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    namespace;
    loadData();
  });

  // Tag color mapping (like Obsidian)
  const tagColors = [
    '#89b4fa', '#a6e3a1', '#f9e2af', '#f38ba8', '#fab387',
    '#cba6f7', '#94e2d5', '#f5c2e7', '#89dceb', '#eba0ac',
  ];

  function getTagColor(tags: string[]): string {
    if (tags.length === 0) return '#6c7086';
    // Hash first tag to a color
    const hash = tags[0].charCodeAt(0) % tagColors.length;
    return tagColors[hash];
  }

  $effect(() => {
    if (!data || data.nodes.length === 0 || !canvas) return;

    // Count connections per node
    const connCount = new Map<string, number>();
    for (const edge of data.edges) {
      connCount.set(edge.source, (connCount.get(edge.source) ?? 0) + 1);
      connCount.set(edge.target, (connCount.get(edge.target) ?? 0) + 1);
    }

    // Init node positions in a circle
    const newNodes = new Map<string, NodePos>();
    const cx = width / 2;
    const cy = height / 2;
    const radius = Math.min(width, height) * 0.3;

    data.nodes.forEach((node, i) => {
      const angle = (i / data!.nodes.length) * Math.PI * 2;
      newNodes.set(node.id, {
        x: cx + Math.cos(angle) * radius + (Math.random() - 0.5) * 50,
        y: cy + Math.sin(angle) * radius + (Math.random() - 0.5) * 50,
        vx: 0,
        vy: 0,
        id: node.id,
        label: node.content.slice(0, 30),
        tags: node.tags,
        namespace: node.namespace,
        connections: connCount.get(node.id) ?? 0,
      });
    });
    nodes = newNodes;
    startSimulation();
  });

  function startSimulation() {
    if (animFrame) cancelAnimationFrame(animFrame);

    function tick() {
      if (!data || !canvas) {
        animFrame = requestAnimationFrame(tick);
        return;
      }
      const ctx = canvas.getContext('2d');
      if (!ctx) {
        animFrame = requestAnimationFrame(tick);
        return;
      }

      ctx.clearRect(0, 0, width, height);

      const n = nodes;
      const nodeArr = Array.from(n.values());

      // ── Force simulation (Obsidian-like) ──

      // Repulsion between all nodes (Coulomb)
      const REPULSION = 1200;
      for (let i = 0; i < nodeArr.length; i++) {
        for (let j = i + 1; j < nodeArr.length; j++) {
          const a = nodeArr[i];
          const b = nodeArr[j];
          const dx = b.x - a.x;
          const dy = b.y - a.y;
          const distSq = dx * dx + dy * dy;
          const dist = Math.sqrt(distSq) || 1;
          const force = REPULSION / distSq;
          const fx = (dx / dist) * force;
          const fy = (dy / dist) * force;
          a.vx -= fx;
          a.vy -= fy;
          b.vx += fx;
          b.vy += fy;
        }
      }

      // Attraction along edges (spring)
      const SPRING_LENGTH = 80;
      const SPRING_K = 0.02;
      for (const edge of data.edges) {
        const a = n.get(edge.source);
        const b = n.get(edge.target);
        if (!a || !b) continue;
        const dx = b.x - a.x;
        const dy = b.y - a.y;
        const dist = Math.sqrt(dx * dx + dy * dy) || 1;
        const force = (dist - SPRING_LENGTH) * SPRING_K;
        const fx = (dx / dist) * force;
        const fy = (dy / dist) * force;
        a.vx += fx;
        a.vy += fy;
        b.vx -= fx;
        b.vy -= fy;
      }

      // Center gravity
      const GRAVITY = 0.0008;
      for (const pos of n.values()) {
        pos.vx += (width / 2 - pos.x) * GRAVITY;
        pos.vy += (height / 2 - pos.y) * GRAVITY;
      }

      // Apply velocity with damping
      const DAMPING = 0.85;
      for (const pos of n.values()) {
        pos.vx *= DAMPING;
        pos.vy *= DAMPING;
        pos.x += pos.vx;
        pos.y += pos.vy;

        // Keep within bounds
        pos.x = Math.max(20, Math.min(width - 20, pos.x));
        pos.y = Math.max(20, Math.min(height - 20, pos.y));
      }

      // ── Draw ──

      // Draw edges
      for (const edge of data.edges) {
        const a = n.get(edge.source);
        const b = n.get(edge.target);
        if (!a || !b) continue;

        const isHighlighted =
          hoveredNode === a.id || hoveredNode === b.id;
        ctx.strokeStyle = isHighlighted
          ? 'rgba(137, 180, 250, 0.5)'
          : 'rgba(49, 50, 68, 0.3)';
        ctx.lineWidth = isHighlighted ? 1.5 : 0.8;
        ctx.beginPath();
        ctx.moveTo(a.x, a.y);
        ctx.lineTo(b.x, b.y);
        ctx.stroke();
      }

      // Draw nodes
      for (const node of data.nodes) {
        const pos = n.get(node.id);
        if (!pos) continue;

        const color = getTagColor(node.tags);
        const isHovered = hoveredNode === node.id;
        const radius = Math.max(3, Math.min(10, 3 + pos.connections * 0.8));

        // Glow for hovered
        if (isHovered) {
          ctx.beginPath();
          ctx.arc(pos.x, pos.y, radius + 6, 0, Math.PI * 2);
          ctx.fillStyle = color + '33';
          ctx.fill();
        }

        // Node circle
        ctx.beginPath();
        ctx.arc(pos.x, pos.y, radius, 0, Math.PI * 2);
        ctx.fillStyle = isHovered ? '#cdd6f4' : color;
        ctx.fill();

        // Label for hovered or well-connected nodes
        if (isHovered || pos.connections >= 3) {
          ctx.font = '11px -apple-system, sans-serif';
          ctx.fillStyle = isHovered ? '#cdd6f4' : '#6c7086';
          ctx.textAlign = 'center';
          const label = pos.label.slice(0, 25);
          ctx.fillText(label, pos.x, pos.y - radius - 5);
        }
      }

      animFrame = requestAnimationFrame(tick);
    }

    tick();
  }

  function handleMouseMove(e: MouseEvent) {
    if (!data || !canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    let found: string | null = null;
    for (const [id, pos] of nodes) {
      const dx = pos.x - x;
      const dy = pos.y - y;
      if (dx * dx + dy * dy < 144) {
        found = id;
        break;
      }
    }
    hoveredNode = found;
    if (canvas) canvas.style.cursor = found ? 'pointer' : 'default';
  }

  function handleClick(e: MouseEvent) {
    if (!hoveredNode) return;
    onmemoryclick(hoveredNode);
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
    <div class="center-msg">Loading graph...</div>
  {:else if !data || data.nodes.length === 0}
    <div class="center-msg">
      <p>No memories to visualize.</p>
      <p class="sub">Create memories with tags to see connections.</p>
    </div>
  {:else}
    <div class="graph-toolbar">
      <span class="graph-info">{data.nodes.length} nodes · {data.edges.length} edges</span>
      {#if utekeReady}
        <span class="uteke-tag">Uteke</span>
      {/if}
    </div>
    <div class="canvas-container">
      <canvas
        bind:this={canvas}
        width={width}
        height={height}
        onmousemove={handleMouseMove}
        onclick={handleClick}
        onmouseleave={() => (hoveredNode = null)}
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

  .graph-toolbar {
    padding: 8px 16px;
    display: flex;
    align-items: center;
    gap: 12px;
    border-bottom: 1px solid var(--border);
  }

  .graph-info {
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .uteke-tag {
    font-size: 0.7rem;
    padding: 2px 8px;
    background: rgba(148, 226, 213, 0.15);
    color: var(--teal);
    border-radius: 3px;
    font-weight: 600;
  }

  .canvas-container {
    flex: 1;
    position: relative;
    overflow: hidden;
  }

  canvas {
    display: block;
    cursor: default;
  }

  .center-msg {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    text-align: center;
    gap: 8px;
  }

  .center-msg .sub {
    font-size: 0.85rem;
    opacity: 0.7;
  }
</style>
