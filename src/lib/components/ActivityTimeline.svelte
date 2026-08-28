<script lang="ts">
  import { buildActivity, intensityLevel } from '../utils/activity';
  import type { MemoryEntry } from '../ts/types';
  import { EmptyState } from '../ui';
  import { CalendarDays } from 'lucide-svelte';

  interface Props {
    memories: MemoryEntry[];
    weeks?: number;
  }

  let { memories, weeks = 12 }: Props = $props();

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
      <div class="stat">
        <span class="stat-num">{summary.today}</span>
        <span class="stat-label">today</span>
      </div>
      <div class="stat">
        <span class="stat-num">{summary.last7}</span>
        <span class="stat-label">last 7 days</span>
      </div>
      <div class="stat">
        <span class="stat-num">{summary.total}</span>
        <span class="stat-label">in window</span>
      </div>
    </div>

    <div class="heatmap-scroll">
      <div class="heatmap">
        <div
          class="months"
          style="grid-template-columns: repeat({weeks}, 14px); min-width: {weeks * 17 - 3}px;"
        >
          {#each monthLabels as m}
            <span class="month" style="grid-column: {m.col + 1};">{m.name}</span>
          {/each}
        </div>
        <div class="grid" style="grid-template-columns: repeat({weeks}, 14px);">
          {#each summary.days as d (d.date)}
            <span
              class="cell lvl-{d.count === -1 ? 'pad' : intensityLevel(d.count, summary.maxCount)}"
              title="{d.date}: {d.count === -1 ? '' : d.count} {d.count === 1 ? 'memory' : 'memories'}"
            ></span>
          {/each}
        </div>
      </div>
      <div class="legend">
        <span>less</span>
        {#each [0, 1, 2, 3, 4] as lvl}
          <span class="cell lvl-{lvl}"></span>
        {/each}
        <span>more</span>
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
    gap: 12px;
    margin-bottom: 14px;
  }

  .stat {
    display: flex;
    flex-direction: column;
    align-items: center;
    flex: 1;
    padding: 10px 8px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }

  .stat-num {
    font-size: 1.2rem;
    font-weight: 700;
    color: var(--accent);
  }

  .stat-label {
    font-size: 0.72rem;
    color: var(--text-muted);
  }

  .heatmap-scroll {
    overflow-x: auto;
  }

  .months {
    display: grid;
    gap: 3px;
    font-size: 0.68rem;
    color: var(--text-muted);
    margin-bottom: 4px;
  }

  .grid {
    display: grid;
    grid-template-rows: repeat(7, 13px);
    grid-auto-flow: column;
    gap: 3px;
  }

  .cell {
    width: 13px;
    height: 13px;
    border-radius: 2px;
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

  .legend {
    display: flex;
    align-items: center;
    gap: 3px;
    justify-content: flex-end;
    margin-top: 8px;
    font-size: 0.68rem;
    color: var(--text-muted);
  }
</style>
