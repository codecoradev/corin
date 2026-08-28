import { describe, it, expect } from 'vitest';
import { buildActivity, intensityLevel } from '../activity';

function iso(daysAgo: number, hour = 12): string {
  const d = new Date();
  d.setDate(d.getDate() - daysAgo);
  d.setHours(hour, 0, 0, 0);
  return d.toISOString();
}

describe('buildActivity', () => {
  it('produces a fixed 7×weeks grid ending on Sunday', () => {
    const s = buildActivity([], 12);
    expect(s.days).toHaveLength(84);
    // last cell is the Sunday of the current week (may be future-padded)
    expect(s.days.at(-1)!.count).toBe(-1);
  });

  it('counts today and last-7 windows', () => {
    const memories = [
      { created_at: iso(0) },
      { created_at: iso(0) },
      { created_at: iso(2) },
      { created_at: iso(30) },
    ];
    const s = buildActivity(memories, 12);
    expect(s.today).toBe(2);
    expect(s.last7).toBe(3);
    expect(s.total).toBe(4);
  });

  it('ignores memories without created_at', () => {
    const s = buildActivity([{ created_at: null }, { created_at: iso(0) }]);
    expect(s.total).toBe(1);
  });

  it('marks future pad cells with -1 and keeps past cells >= 0', () => {
    const s = buildActivity([], 4);
    const future = s.days.filter((d) => d.count === -1);
    const past = s.days.filter((d) => d.count !== -1);
    expect(future.length).toBeGreaterThan(0);
    for (const d of past) expect(d.count).toBe(0);
  });
});

describe('intensityLevel', () => {
  it('buckets by ratio against the max', () => {
    expect(intensityLevel(0, 10)).toBe(0);
    expect(intensityLevel(1, 10)).toBe(1);
    expect(intensityLevel(3, 10)).toBe(2);
    expect(intensityLevel(7, 10)).toBe(3);
    expect(intensityLevel(10, 10)).toBe(4);
  });

  it('gives full intensity when the max is a single hit', () => {
    expect(intensityLevel(1, 1)).toBe(4);
  });
});
