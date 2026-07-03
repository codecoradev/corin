// TTL cache store for uteke HTTP responses.
//
// Bottleneck is network RTT to a remote uteke-serve (esp. fan-out per
// namespace). This module caches the small, frequently-reused responses
// (namespaces list, stats) with a short TTL so view switches and reloads
// don't re-fire N round-trips.
//
// No external infra (Redis etc.) — this is a desktop app; in-process only.
// Entries expire by time and are also invalidated explicitly on mutations
// (remember/forget/reconnect/disconnect).

import { uteke, system } from '../ts/ipc';

interface CacheEntry<T> {
  data: T;
  ts: number; // ms epoch
}

const TTL_NAMESPACES_MS = 30_000; // 30s
const TTL_STATS_MS = 60_000; // 60s

let namespacesCache = $state<CacheEntry<string[]> | null>(null);
let statsCache = $state<CacheEntry<StatsResponse | null> | null>(null);

function fresh<T>(entry: CacheEntry<T> | null, ttl: number): entry is CacheEntry<T> {
  return entry !== null && Date.now() - entry.ts < ttl;
}

/** List of namespaces (uteke if available, else local).  Cached 30s. */
export async function getNamespaces(): Promise<string[]> {
  if (fresh(namespacesCache, TTL_NAMESPACES_MS)) return namespacesCache!.data;
  const ok = await uteke.available().catch(() => false);
  const data = ok
    ? await uteke.namespaces().catch(() => [])
    : await system.listNamespaces().catch(() => []);
  namespacesCache = { data, ts: Date.now() };
  return data;
}

interface StatsResponse {
  total_memories: number;
  total_namespaces: number;
  total_tags: number;
  total_edges: number;
  db_size_bytes: number;
}

/** Aggregate stats.  Cached 60s. */
export async function getStats(): Promise<StatsResponse | null> {
  if (fresh(statsCache, TTL_STATS_MS)) return statsCache!.data;
  const data = (await system.stats().catch(() => null)) as StatsResponse | null;
  statsCache = { data, ts: Date.now() };
  return data;
}

/** Invalidate everything (call on remember/forget/reconnect/disconnect). */
export function invalidateAll() {
  namespacesCache = null;
  statsCache = null;
}

export function invalidateNamespaces() {
  namespacesCache = null;
}

export function invalidateStats() {
  statsCache = null;
}
