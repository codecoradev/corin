/**
 * Aggregation for the global activity timeline (#235).
 * Pure functions — no Svelte — so they stay unit-testable.
 */

export interface MemoryLike {
  created_at: string | null;
  namespace?: string | null;
}

export interface ActivityDay {
  /** Local calendar date, YYYY-MM-DD. */
  date: string;
  count: number;
}

export interface ActivitySummary {
  /** Last `weeks` weeks, oldest first, always 7×weeks entries (gaps = 0). */
  days: ActivityDay[];
  maxCount: number;
  today: number;
  last7: number;
  total: number;
}

function localDateKey(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

/**
 * Bucket memories into a fixed window of `weeks` calendar weeks ending today.
 * The grid is column-major friendly: index = week*7 + weekday(Mon=0..Sun=6).
 */
export function buildActivity(memories: MemoryLike[], weeks = 26): ActivitySummary {
  const counts = new Map<string, number>();
  let today = 0;
  let last7 = 0;
  let total = 0;

  const todayKey = localDateKey(new Date());
  const weekAgoKey = localDateKey(new Date(Date.now() - 6 * 86_400_000));

  for (const m of memories) {
    if (!m.created_at) continue;
    const key = localDateKey(new Date(m.created_at));
    counts.set(key, (counts.get(key) ?? 0) + 1);
    total++;
    if (key === todayKey) today++;
    if (key >= weekAgoKey && key <= todayKey) last7++;
  }

  // Build the fixed-size grid ending today, padded to whole weeks (Mon start).
  const days: ActivityDay[] = [];
  const todayDate = new Date();
  const daysFromMonday = (todayDate.getDay() + 6) % 7; // Mon=0..Sun=6
  const gridEnd = new Date(todayDate);
  gridEnd.setDate(gridEnd.getDate() + (6 - daysFromMonday)); // pad to Sunday
  const totalDays = weeks * 7;
  const start = new Date(gridEnd);
  start.setDate(start.getDate() - (totalDays - 1));

  let maxCount = 0;
  for (let i = 0; i < totalDays; i++) {
    const d = new Date(start);
    d.setDate(start.getDate() + i);
    const key = localDateKey(d);
    const inFuture = d > todayDate;
    const count = inFuture ? -1 : counts.get(key) ?? 0;
    if (count > maxCount) maxCount = count;
    days.push({ date: key, count });
  }

  return { days, maxCount, today, last7, total };
}

/** 0 = empty, 1..4 = rising intensity buckets for the heatmap. */
export function intensityLevel(count: number, maxCount: number): 0 | 1 | 2 | 3 | 4 {
  if (count <= 0) return 0;
  if (maxCount <= 1) return 4;
  const ratio = count / maxCount;
  if (ratio <= 0.25) return 1;
  if (ratio <= 0.5) return 2;
  if (ratio <= 0.75) return 3;
  return 4;
}
