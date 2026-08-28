<script lang="ts">
  import { buildActivity, intensityLevel } from '../utils/activity';
  import type { MemoryEntry } from '../ts/types';
  import { EmptyState } from '../ui';
  import { CalendarDays } from 'lucide-svelte';

  interface Props {
    memories: MemoryEntry[];
    weeks?: number;
  }

  let { memories, weeks = 26 }: Props = $props();

  const summary = $derived(buildActivity(memories, weeks));
  const monthLabels = $derived.by(() => {
    // Label the month whenever a column starts a new month.
    const labels: { col: number; name: string }[] = [];
    let last = '';
    summary.days.forEach((d, i) => {
      const col = Math.floor(i / 7);
      const name = new Date(d.date + 'T00:00:00').toLocaleString('en', { month: 'short' });
      if (name !== last) {
        labels.push({ col, name });
        last = name;
      }
    });
    return labels;
  });
</script>

<div class="activity-timeline">
  {#if summary.total === 0}
    <EmptyState
      icon={CalendarDays}
      title="No activity yet."
      subtitle="Memories saved by your agents will chart here — a glance tells you whether things are flowing or stalled."
    />
  {:else}
    <div class="stats-row">
      <span class="stat"><b>{summary.today}</b> today</span>
      <span class="stat"><b>{summary.last7}</b> last 7 days</span>
      <span class="stat"><b>{summary.total}</b> in window</span>
      <span class="stat legend-stat">
        less
        {#each [0, 1, 2, 3, 4] as lvl}
          <span class="cell lvl-{lvl}"></span>
        {/each}
        more
      </span>
    </div>

    <div class="heatmap-scroll">
      <div class="heatmap">
        <div
          class="months"
          style="grid-template-columns: repeat({weeks}, minmax(0, 1fr));"
        >
          {#each monthLabels as m}
            <span class="month" style="grid-column: {m.col + 1};">{m.name}</span>
          {/each}
        </div>
        <div class="grid" style="grid-template-columns: repeat({weeks}, minmax(0, 1fr));">
          {#each summary.days as d (d.date)}
            <span
              class="cell lvl-{d.count === -1 ? 'pad' : intensityLevel(d.count, summary.maxCount)}"
              title="{d.date}: {d.count === -1 ? '' : d.count} {d.count === 1 ? 'memory' : 'memories'}"
            ></span>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .activity-timeline {
    padding: 4px 0;
  }

  .stats-row {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
    margin-bottom: 12px;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .stat b {
    font-size: 0.95rem;
    color: var(--text-primary);
    margin-right: 2px;
  }

  .legend-stat {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .heatmap-scroll {
    overflow-x: auto;
  }

  .months {
    display: grid;
    gap: 4px;
    font-size: 0.68rem;
    color: var(--text-muted);
    margin-bottom: 4px;
  }

  .grid {
    display: grid;
    grid-template-rows: repeat(7, auto);
    grid-auto-flow: column;
    gap: 4px;
  }

  .cell {
    width: 100%;
    aspect-ratio: 1;
    border-radius: 3px;
    background: var(--bg-hover);
    display: inline-block;
  }

  .cell.lvl-pad {
    visibility: hidden;
  }

  .cell.lvl-1 { background: color-mix(in srgb, var(--green) 30%, transparent); }
  .cell.lvl-2 { background: color-mix(in srgb, var(--green) 55%, transparent); }
  .cell.lvl-3 { background: color-mix(in srgb, var(--green) 78%, transparent); }
  .cell.lvl-4 { background: var(--green); }
</style>
