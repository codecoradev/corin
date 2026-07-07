/// Pure graph utilities — no component state dependencies.
/// Used by GraphView for node coloring and edge generation.

export interface GraphMemory {
  id: string;
  content: string;
  tags: string[];
}

export interface GraphEdge {
  source: string;
  target: string;
  weight: number;
}

/** Catppuccin Mocha palette for node colors. */
export const NODE_COLORS = [
  '#89b4fa', '#a6e3a1', '#f9e2af', '#f38ba8', '#fab387',
  '#cba6f7', '#94e2d5', '#f5c2e7', '#89dceb', '#eba0ac',
];

/** Pick a color based on the first tag's char code. Falls back to muted. */
export function pickColor(tags: string[]): string {
  if (!tags?.length) return '#6c7086';
  return NODE_COLORS[(tags[0].charCodeAt(0) || 0) % NODE_COLORS.length];
}

/**
 * Client-side edge generation from shared tags.
 * Groups memories by tag and links every pair that shares a tag.
 * Caps per-tag connections to avoid clutter.
 */
export function buildTagEdges(
  mems: GraphMemory[],
  maxPerTag: number,
): GraphEdge[] {
  const result: GraphEdge[] = [];
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

  // Fallback: link unconnected nodes so the graph is never fully disconnected
  if (result.length < mems.length / 3 && mems.length > 1) {
    const connected = new Set<string>();
    for (const e of result) { connected.add(e.source); connected.add(e.target); }
    const unconnected = mems.filter(m => !connected.has(m.id));
    for (const m of unconnected) {
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
