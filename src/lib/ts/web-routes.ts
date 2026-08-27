/**
 * Web-mode command handlers — maps CorIn IPC commands onto the uteke-serve
 * REST API via the `/api` Vite proxy, replacing the Rust bridge when the UI
 * runs in a plain browser (no `__TAURI_INTERNALS__`).
 *
 * Conventions mirror src-tauri/src/uteke_client.rs: snake_case bodies,
 * Bearer-free local access, non-2xx → thrown string errors.
 */
import type {
  ConnectionInfo, DeprecatedListResponse, DocEntry, DocMemRefsResponse, DocSearchResult,
  GraphData, HealthInfo, ImportResult, LifecycleCycleResult, LifecycleStatus,
  MemoryDocRefsResponse, MemoryEntry, MemoryFeedbackResponse, OrphanMemory,
  RoomEntry, SearchResult, StatsResponse, TimelineEvent, UnifiedSearchResult,
} from './types';

export type Payload = Record<string, unknown>;
type Handler = (p: Payload) => Promise<unknown>;

const API = '/api';
const DREAM_TIMEOUT_MS = 310_000;

// ── HTTP plumbing ────────────────────────────────────────────────────────

async function req<T>(
  method: string,
  path: string,
  opts?: { body?: unknown; query?: Record<string, string | number | boolean | null | undefined> },
): Promise<T> {
  let url = `${API}${path}`;
  if (opts?.query) {
    const qs = Object.entries(opts.query)
      .filter(([, v]) => v !== undefined && v !== null)
      .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`)
      .join('&');
    if (qs) url += `?${qs}`;
  }
  const res = await fetch(url, {
    method,
    headers: opts?.body !== undefined ? { 'Content-Type': 'application/json' } : undefined,
    body: opts?.body !== undefined ? JSON.stringify(opts.body) : undefined,
  });
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    throw new Error(`${path}: server returned ${res.status}: ${text}`);
  }
  if (res.status === 204) return undefined as T;
  return (await res.json()) as T;
}

/** Raw-body endpoints (/export returns JSONL text, /context returns markdown). */
async function reqText(
  method: string,
  path: string,
  opts?: { query?: Record<string, string | number | boolean | null | undefined>; body?: unknown },
): Promise<string> {
  const res = await fetch(buildUrl(path, opts?.query), {
    method,
    headers: opts?.body !== undefined ? { 'Content-Type': 'application/json' } : undefined,
    body: opts?.body !== undefined ? JSON.stringify(opts.body) : undefined,
  });
  if (!res.ok) throw new Error(`${path}: server returned ${res.status}`);
  return res.text();
}

function buildUrl(path: string, query?: Record<string, string | number | boolean | null | undefined>): string {
  let url = `${API}${path}`;
  if (query) {
    const qs = Object.entries(query)
      .filter(([, v]) => v !== undefined && v !== null)
      .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`)
      .join('&');
    if (qs) url += `?${qs}`;
  }
  return url;
}

/** Drop null/undefined keys one level deep — serde Option fields prefer absent. */
function body(p: Payload): Payload {
  return Object.fromEntries(Object.entries(p).filter(([, v]) => v !== null && v !== undefined));
}

// ── Shape helpers ────────────────────────────────────────────────────────

/** Server memory DTO (uteke /list /memory rows) — id & content always present. */
interface UtekeMemoryRaw {
  id: string;
  content: string;
  tags?: string[] | null;
  content_type?: string | null;
  importance?: number | null;
  namespace?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
  pinned?: boolean | null;
}

function toMemory(m: UtekeMemoryRaw): MemoryEntry {
  return {
    id: m.id,
    content: m.content ?? '',
    tags: Array.isArray(m.tags) ? m.tags : [],
    content_type: m.content_type ?? null,
    importance: m.importance ?? null,
    namespace: m.namespace ?? null,
    created_at: m.created_at ?? null,
    updated_at: m.updated_at ?? null,
  };
}

/** GET /memory → UtekeMemory (full DTO). */
function getMemory(id: string): Promise<UtekeMemoryRaw> {
  return req<UtekeMemoryRaw>('GET', '/memory', { query: { id } });
}

/** POST /list → full DTOs for one namespace. */
function listNs(namespace: string | null | undefined, tag: string | null | undefined, limit: number, offset: number): Promise<UtekeMemoryRaw[]> {
  return req<UtekeMemoryRaw[]>('POST', '/list', { body: body({ namespace: namespace ?? null, tag: tag ?? null, limit, offset }) as Payload });
}

/** POST /recall → [{memory, score}] rows. */
interface RecallRow { memory: UtekeMemoryRaw & { id: string; content: string }; score: number }
function recallRows(query: string, namespace: string | null | undefined, limit: number): Promise<RecallRow[]> {
  return req<RecallRow[]>('POST', '/recall', { body: body({ query, namespace: namespace ?? null, limit }) as Payload });
}

async function namespaces(): Promise<string[]> {
  const ns = await req<string[]>('GET', '/namespaces');
  return Array.isArray(ns) ? ns : [];
}

async function health(): Promise<{ ok: boolean; version: string | null }> {
  try {
    const h = await req<{ version?: string; status?: string }>('GET', '/health');
    return { ok: true, version: h?.version ?? h?.status ?? null };
  } catch {
    return { ok: false, version: null };
  }
}

function versionMeets(current: string | null, required: string): boolean {
  if (!current) return false;
  const parse = (s: string) => s.split('.').map((x) => parseInt(x, 10) || 0);
  const [c, r] = [parse(current), parse(required)];
  for (let i = 0; i < Math.max(c.length, r.length); i++) {
    const d = (c[i] ?? 0) - (r[i] ?? 0);
    if (d !== 0) return d > 0;
  }
  return true;
}

function nanoid12(): string {
  const chars = 'useandom-26T198340PX75pxJACKVERYMINDBUSHWOLFTMEQ';
  let out = '';
  for (let i = 0; i < 12; i++) out += chars[Math.floor(Math.random() * chars.length)];
  return out;
}

// ── Mixed logic ported from commands.rs ──────────────────────────────────

/// commands.rs:list_multi_namespace — per-namespace fan-out, dedup, newest-first.
async function listMultiNamespace(namespacesArg: string[] | null, tag: string | null, limit: number, offset: number): Promise<MemoryEntry[]> {
  const PER_NS_CAP = 200;
  const fetchN = Math.min(offset + limit, PER_NS_CAP);
  const targets = namespacesArg && namespacesArg.length ? namespacesArg : await namespaces();
  const seen = new Set<string>();
  const all: UtekeMemoryRaw[] = [];
  for (const ns of targets) {
    try {
      for (const m of await listNs(ns, tag, fetchN, 0)) {
        if (seen.add(m.id)) all.push(m);
      }
    } catch (e) {
      console.error(`CorIn: list: skipping namespace '${ns}':`, e);
    }
  }
  all.sort((a, b) => String(b.created_at ?? '').localeCompare(String(a.created_at ?? '')));
  return all.slice(offset, offset + limit).map(toMemory);
}

/// commands.rs:build_tag_graph — fallback graph when /graph has no edges.
function buildTagGraph(memories: UtekeMemoryRaw[]): { nodes: unknown[]; edges: unknown[]; relation_types: string[] } {
  const nodes = memories.map((m) => ({
    id: m.id,
    label: String(m.content ?? '').slice(0, 60),
    entity_type: m.tags?.[0] ?? null,
  }));
  const tagToIds = new Map<string, string[]>();
  for (const m of memories) {
    for (const t of m.tags ?? []) {
      if (!tagToIds.has(t)) tagToIds.set(t, []);
      tagToIds.get(t)!.push(m.id!);
    }
  }
  const edges: unknown[] = [];
  const seen = new Set<string>();
  const MAX_PAIRS = 5;
  for (const ids of tagToIds.values()) {
    let count = 0;
    for (let i = 0; i < ids.length && count < MAX_PAIRS; i++) {
      for (let j = i + 1; j < ids.length && count < MAX_PAIRS; j++) {
        const key = `${ids[i]}|${ids[j]}`;
        if (!seen.has(key)) {
          seen.add(key);
          edges.push({ source: ids[i], target: ids[j], relation: 'shared_tag', weight: 0.5 });
          count++;
        }
      }
    }
  }
  return { nodes, edges, relation_types: edges.length ? ['shared_tag'] : [] };
}

/// commands.rs:get_graph_data — GET /graph remapped to frontend GraphData.
async function graphData(p: Payload): Promise<GraphData> {
  if (!(await health()).ok) return { nodes: [], edges: [] };
  const limit = typeof p.limit === 'number' ? p.limit : 100;
  const gd = await req<{ nodes: Array<{ id: string; label: string; entity_type: string | null }>; edges: Array<{ source: string; target: string; weight: number }> }>(
    'GET', '/graph', { query: p.namespace ? { namespace: String(p.namespace) } : undefined },
  );
  const nodes: MemoryEntry[] = gd.nodes.slice(0, limit).map((n) => ({
    id: n.id, content: n.label, tags: [], content_type: n.entity_type,
    importance: null, namespace: null, created_at: null, updated_at: null,
  }));
  const edges = gd.edges.slice(0, limit).map((e, i) => ({ id: i, source: e.source, target: e.target, weight: e.weight }));
  return { nodes, edges };
}

/// commands.rs:uteke_server_graph — real cosine edges, tag-graph fallback.
async function serverGraph(p: Payload): Promise<unknown> {
  const empty = { nodes: [], edges: [], stats: { node_count: 0, edge_count: 0, relation_types: [] as string[] } };
  if (!(await health()).ok) return { ...empty, hint: "Run 'uteke serve' to enable semantic search and auto-linking" };
  const nsList = Array.isArray(p.namespaces) ? (p.namespaces as string[]) : p.namespace ? [String(p.namespace)] : null;

  // Multi-namespace scope → always tag-based fan-out (commands.rs:build_multi_namespace_graph).
  if (nsList && nsList.length > 1) {
    const PER_NS = 60;
    const mems: UtekeMemoryRaw[] = [];
    const seen = new Set<string>();
    for (const ns of nsList) {
      try {
        for (const m of await listNs(ns, null, PER_NS, 0)) if (seen.add(m.id)) mems.push(m);
      } catch (e) {
        console.error(`CorIn: graph: skipping namespace '${ns}':`, e);
      }
    }
    if (!mems.length) return { ...empty, hint: 'No memories in the selected namespace(s)' };
    const g = buildTagGraph(mems);
    return { ...g, stats: { node_count: g.nodes.length, edge_count: g.edges.length, relation_types: g.relation_types }, hint: `Tag-based graph across ${nsList.length} namespace(s)` };
  }

  const q: Record<string, string> = nsList && nsList.length === 1 ? { namespace: nsList[0] } : {};
  const gd = await req<{ nodes: Array<{ id: string; label: string; entity_type: string | null }>; edges: Array<{ source: string; target: string; relation: string; weight: number }> }>('GET', '/graph', { query: q });
  if (gd.edges.length === 0) {
    const mems = await listNs(nsList?.[0] ?? null, null, 100, 0).catch(() => []);
    if (!mems.length) return { ...empty, hint: 'No memories yet' };
    const g = buildTagGraph(mems);
    return { ...g, stats: { node_count: g.nodes.length, edge_count: g.edges.length, relation_types: g.relation_types }, hint: 'Tag-based graph (cosine auto-link not generated yet)' };
  }
  return {
    nodes: gd.nodes,
    edges: gd.edges,
    stats: { node_count: gd.nodes.length, edge_count: gd.edges.length, relation_types: [...new Set(gd.edges.map((e) => e.relation))] },
  };
}

/// commands.rs:uteke_remember — duplicate pre-check via recall ≥ 0.92.
async function rememberWithDupCheck(p: Payload): Promise<{ id?: string; duplicate: boolean; existing_id?: string; existing_content?: string; score?: number; hint?: string }> {
  const content = String(p.content ?? '');
  const tags = Array.isArray(p.tags) ? (p.tags as string[]) : [];
  const ns = (p.namespace as string | null) ?? 'default';
  try {
    const existing = await recallRows(content, ns, 3);
    const dup = existing.find((r) => r.score >= 0.92);
    if (dup) {
      return { duplicate: true, existing_id: dup.memory.id, existing_content: dup.memory.content, score: dup.score, hint: 'This memory appears to be a duplicate of an existing one.' };
    }
  } catch { /* recall failure must not block insertion */ }
  const { id } = await req<{ id: string }>('POST', '/remember', { body: { content, tags, namespace: p.namespace ?? undefined } });
  return { id, duplicate: false };
}

/// commands.rs:uteke_neighbors — self-recall semantic neighbors.
async function semanticNeighbors(id: string, limit: number): Promise<unknown[]> {
  const m = await getMemory(id);
  const results = await recallRows(String(m.content ?? ''), m.namespace ?? null, limit + 1);
  return results
    .filter((r) => r.memory.id !== id)
    .slice(0, limit)
    .map((r) => ({
      id: r.memory.id,
      content: r.memory.content,
      tags: r.memory.tags ?? [],
      namespace: r.memory.namespace ?? null,
      importance: r.memory.importance ?? null,
      content_type: r.memory.content_type ?? null,
      created_at: r.memory.created_at ?? null,
      updated_at: r.memory.updated_at ?? null,
      relationship: r.score >= 0.92 ? 'possible_duplicate' : 'semantic',
      score: r.score,
      shared_tags: [],
    }));
}

/// commands.rs:get_neighbors — /graph adjacency + per-id hydration.
async function graphNeighbors(id: string): Promise<MemoryEntry[]> {
  if (!(await health()).ok) return [];
  const gd = await req<{ edges: Array<{ source: string; target: string }> }>('GET', '/graph', { query: { node_id: id } });
  const ids = new Set<string>();
  for (const e of gd.edges) {
    if (e.source === id) ids.add(e.target);
    else if (e.target === id) ids.add(e.source);
  }
  const entries: MemoryEntry[] = [];
  for (const nid of ids) {
    try {
      entries.push(toMemory(await getMemory(nid)));
    } catch { /* skip missing */ }
  }
  return entries;
}

/// commands.rs:format_room_document — defensive JSON → markdown.
function formatRoomDocument(doc: Record<string, unknown>): string {
  const title = (doc.title as string) ?? (doc.heading as string) ?? 'unnamed';
  let out = `# Room: ${title}\n\n`;
  const sections = (doc.sections as Array<Record<string, unknown>>) ?? [];
  if (!sections.length) return out + '_No memories in this room yet._\n';
  for (const section of sections) {
    const heading = (section.heading ?? section.title ?? section.label) as string | undefined;
    if (heading) out += `## ${heading}\n\n`;
    const items = ((section.items ?? section.memories ?? section.entries) as unknown[]) ?? [];
    for (const item of items) {
      const text = typeof item === 'string'
        ? item
        : String(((item as Record<string, unknown>).content ?? (item as Record<string, unknown>).text ?? (item as Record<string, unknown>).body) ?? '');
      if (text) out += `- ${text}\n`;
    }
    out += '\n';
  }
  return out;
}

async function aggregateStats(): Promise<StatsResponse> {
  if (!(await health()).ok) return { total_memories: 0, total_namespaces: 0, total_tags: 0, total_edges: 0, db_size_bytes: 0 };
  const s = await req<{ total_memories: number; unique_tags: number; db_size_bytes: number }>('GET', '/stats');
  const nsCount = await namespaces().catch(() => [] as string[]);
  return { total_memories: s.total_memories, total_namespaces: nsCount.length, total_tags: s.unique_tags, total_edges: 0, db_size_bytes: s.db_size_bytes };
}

/** Normalized /room/list row — desktop maps these into RoomEntry & uteke.rooms. */
interface RawRoomRow { id: string; name: string; namespace: string; memory_count: number; participant_count: number; created_at: string; updated_at: string }
async function roomsRaw(): Promise<RawRoomRow[]> {
  const rows = await req<Array<Partial<Record<string, unknown>>>>('GET', '/room/list');
  return rows.map((r) => ({
    id: String(r.id ?? r.room_id ?? ''),
    name: String(r.name ?? r.title ?? ''),
    namespace: String(r.namespace ?? 'default'),
    memory_count: Number(r.memory_count ?? 0),
    participant_count: Number(r.participant_count ?? 0),
    created_at: String(r.created_at ?? ''),
    updated_at: String(r.updated_at ?? r.created_at ?? ''),
  }));
}

/** GET /namespaces?with_counts=true with plain fallback. */
async function namespacesWithCounts(): Promise<Array<{ name: string; count: number }>> {
  try {
    const rows = await req<Array<{ name: string; count: number }>>('GET', '/namespaces', { query: { with_counts: 'true' } });
    if (Array.isArray(rows)) return rows;
  } catch { /* legacy server without the param */ }
  const plain = await namespaces();
  return plain.map((name) => ({ name, count: 0 }));
}

// ── Local-only state (browser equivalents of the SQLite stores) ──────────

const LS_SETTINGS = 'corin_web_settings';
const LS_DREAM_HISTORY = 'corin_web_dream_history';

function lsGet<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : fallback;
  } catch {
    return fallback;
  }
}
function lsSet(key: string, value: unknown): void {
  localStorage.setItem(key, JSON.stringify(value));
}

interface DreamHistoryRow { id: number; ran_at: string; success: boolean; total_changes: number; total_warnings: number; total_errors: number; duration_ms: number; phases: Array<{ phase: string; status: string; summary: string; changes: number; warnings: number }> }

// ── Import/export (pragmatic browser ports of commands.rs engines) ──────

async function exportData(format: string, namespace: string | null): Promise<string> {
  const memories = (await listNs(namespace, null, 100_000, 0)).map(toMemory);
  if (format === 'csv') {
    const esc = (s: string) => `"${s.replaceAll('"', '""')}"`;
    const rows = memories.map((m) => [m.id, m.content, m.tags.join('|'), m.namespace ?? '', m.created_at ?? ''].map(esc).join(','));
    return ['id,content,tags,namespace,created_at', ...rows].join('\n');
  }
  if (format === 'markdown') {
    const parts = memories.map((m) => [`---`, `id: ${m.id}`, `tags: [${m.tags.join(', ')}]`, `namespace: ${m.namespace ?? ''}`, `created_at: ${m.created_at ?? ''}`, `---`, '', m.content, ''].join('\n'));
    return [`# Corin Export${namespace ? ` (${namespace})` : ''}`, '', ...parts].join('\n');
  }
  // json — v2-style envelope; edges are approximated from /graph (desktop walks /edges per memory).
  const rooms = await req<unknown[]>('GET', '/room/list').catch(() => []);
  const gd = await req<{ edges: unknown[] }>('GET', '/graph').catch(() => ({ edges: [] }));
  return JSON.stringify({ version: 2, exported_at: new Date().toISOString(), namespace, memories, edges: gd.edges, rooms }, null, 2);
}

function extractMemories(data: string): Array<Payload> {
  const parsed = JSON.parse(data) as unknown;
  if (Array.isArray(parsed)) return parsed as Array<Payload>;
  if (parsed && typeof parsed === 'object' && Array.isArray((parsed as Payload).memories)) return (parsed as Payload).memories as Array<Payload>;
  throw new Error('Format JSON tidak dikenali (harus array atau envelope {memories})');
}

// ── Connections (single implicit proxied connection in web mode) ─────────

function primaryConnection(status: string): ConnectionInfo {
  return {
    id: 'web-proxy',
    name: 'Uteke (via proxy web dev)',
    product_type: 'uteke',
    // Bukan URL yang bisa diedit — koneksi ini implisit lewat proxy Vite.
    url: '/api (dev proxy)',
    has_token: false,
    capabilities: { read: true, write: true, search: true, realtime: false },
    status,
    is_primary: true,
    created_at: new Date(0).toISOString(),
    last_tested_at: null,
  };
}

async function probeHealthInfo(): Promise<HealthInfo> {
  const start = performance.now();
  const h = await health();
  return {
    success: h.ok,
    latency_ms: Math.round(performance.now() - start),
    version: h.version,
    error: h.ok ? null : 'Uteke serve tidak dapat dihubungi lewat /api',
  };
}

// ── The route table ──────────────────────────────────────────────────────

export const webHandlers: Record<string, Handler> = {
  // Desktop/workspace
  init_data_dir: async () => {
    const h = await health();
    if (!h.ok) throw new Error("Server Uteke tidak berjalan di 127.0.0.1:8767 — jalankan 'uteke serve'.");
    return '~/.codecora';
  },
  get_settings: () => Promise.resolve(lsGet<Record<string, string>>(LS_SETTINGS, {})),
  set_settings: async (p) => { lsSet(LS_SETTINGS, p.settings); },

  // Memories
  remember: async (p) => (await req<{ id: string }>('POST', '/remember', { body: { content: p.content, tags: p.tags, namespace: p.namespace ?? undefined } })).id,
  recall: async (p): Promise<SearchResult[]> =>
    (await recallRows(String(p.query), p.namespace as string | null, typeof p.limit === 'number' ? p.limit : 10))
      .map((r) => ({ id: r.memory.id, content: r.memory.content, score: r.score, tags: r.memory.tags ?? [] })),
  search: (p) => webHandlers.recall(p),
  list: async (p): Promise<MemoryEntry[]> =>
    (await listNs(p.namespace as string | null, p.tag as string | null, typeof p.limit === 'number' ? p.limit : 50, typeof p.offset === 'number' ? p.offset : 0)).map(toMemory),
  forget: async (p) => { void (await req<unknown>('DELETE', '/forget', { query: { id: p.id as string } })); },
  get_memory: async (p) => toMemory(await getMemory(String(p.id))),

  // Graph
  get_graph_data: graphData,
  get_neighbors: async (p) => graphNeighbors(String(p.id)),
  add_edge: async (p) => {
    const [source, target] = [String(p.source), String(p.target)];
    if (source === target) throw new Error('Self-loop edge tidak diizinkan');
    await getMemory(source); // existence check seperti versi desktop
    await getMemory(target);
    await req('POST', '/graph/edge', { body: { source, target, relation: (p.edge_type as string) ?? 'related', weight: p.weight ?? undefined } });
    return 0;
  },
  remove_edge: async (p) => {
    if (typeof p.source !== 'string' || typeof p.target !== 'string') {
      throw new Error('remove_edge butuh source & target (bukan id)');
    }
    await req('DELETE', '/graph/edge', { query: { source: p.source, target: p.target, relation: (p.relation as string) ?? undefined } });
  },

  // Rooms
  list_rooms: async (): Promise<RoomEntry[]> =>
    (await roomsRaw()).map((r) => ({
      id: r.id,
      name: r.name,
      participant_count: r.participant_count,
      memory_count: r.memory_count,
      created_at: r.created_at || null,
    })),
  get_room_summary: async (p) => {
    const summary = await req<unknown>('POST', '/room/summary', { body: { room_id: p.roomId } });
    return JSON.stringify(summary, null, 2);
  },
  get_room_document: async (p) => {
    try {
      const doc = await req<Record<string, unknown>>('POST', '/room/summary-document', { body: { room_id: p.roomId } });
      return formatRoomDocument(doc);
    } catch {
      const legacy = await req<Record<string, unknown>>('POST', '/room/document', { body: { room_id: p.roomId } });
      return formatRoomDocument(legacy);
    }
  },
  create_room: async (p) => {
    const roomId = nanoid12();
    await req('POST', '/room/create', { body: { room_id: roomId, title: p.name, namespace: p.namespace ?? undefined } });
    return roomId;
  },
  delete_room: async (p) => { await req('DELETE', '/room/delete', { query: { room_id: p.roomId as string } }); },

  // System / stats
  stats: aggregateStats,
  uteke_stats: aggregateStats,
  list_namespaces: namespaces,
  uteke_namespaces: namespaces,
  uteke_namespaces_with_counts: namespacesWithCounts,
  // commands.rs:list_tags mengaproksimasi dengan namespaces_with_counts
  list_tags: async () => {
    const counts = await namespacesWithCounts();
    return Object.fromEntries(counts.map((nc) => [nc.name, nc.count]));
  },

  // Import/export data desktop (JSON/markdown/CSV kustom — bukan /import JSONL)
  export_data: async (p) => exportData(String(p.format), (p.namespace as string | null) ?? null),
  import_preview: async (p) => {
    if (p.format === 'markdown') throw new Error('Preview import Markdown hanya tersedia di aplikasi desktop.');
    const mems = extractMemories(String(p.data));
    const namespacesSeen = new Set<string>();
    const tagsSeen = new Set<string>();
    for (const m of mems) {
      if (m.namespace) namespacesSeen.add(String(m.namespace));
      for (const t of (Array.isArray(m.tags) ? m.tags : []) as string[]) tagsSeen.add(t);
    }
    return { format: 'json', memories: mems.length, edges: 0, rooms: 0, namespaces: [...namespacesSeen], tags: [...tagsSeen] };
  },
  import_data: async (p) => {
    if (p.format === 'markdown') throw new Error('Import Markdown hanya tersedia di aplikasi desktop.');
    let imported = 0;
    for (const m of extractMemories(String(p.data))) {
      try {
        await req('POST', '/remember', { body: { content: String(m.content ?? ''), tags: Array.isArray(m.tags) ? m.tags : [], namespace: (m.namespace as string) ?? undefined } });
        imported++;
      } catch { /* lanjut baris berikutnya */ }
    }
    return imported;
  },

  // Uteke read-only mirror
  uteke_available: async () => (await health()).ok,
  uteke_get: async (p) => toMemory(await getMemory(String(p.id))),
  uteke_graph: graphData,
  uteke_rooms: async (p) => {
    const base = await roomsRaw();
    const nsFilter = p.namespace as string | null;
    const filtered = !nsFilter ? base : base.filter((r) => r.namespace === 'default' || r.namespace === nsFilter);
    return Promise.all(filtered.slice(0, 100).map(async (r) => {
      const stats = await req<{ memory_count?: number; participant_count?: number }>('POST', '/room/stats', { body: { room_id: r.id } }).catch(() => null);
      return {
        id: r.id,
        title: r.name || null,
        namespace: r.namespace,
        memory_count: stats?.memory_count ?? r.memory_count,
        participant_count: stats?.participant_count ?? r.participant_count,
        created_at: r.created_at,
        updated_at: r.updated_at,
      };
    }));
  },
  uteke_room_recall: async (p) => {
    // Endpoint khusus room — /recall generik memakai namespace, bukan room.
    const rows = await req<RecallRow[]>('POST', '/room/recall', {
      body: { room_id: p.roomId as string, query: '', limit: (typeof p.limit === 'number' ? p.limit : 20) },
    });
    return rows.map((r) => toMemory(r.memory));
  },
  uteke_room_memories: async (p) => {
    try {
      const rows = await req<UtekeMemoryRaw[]>('GET', '/room/memories', {
        query: { room_id: p.roomId as string, limit: (p.limit as number) ?? undefined, author: (p.author as string) ?? undefined },
      });
      return rows.map(toMemory);
    } catch {
      return webHandlers.uteke_room_recall(p);
    }
  },
  uteke_room_stats: async (p) => req<{ memory_count: number; participant_count: number; participant_namespaces?: string[] }>('POST', '/room/stats', { body: { room_id: p.roomId } }),
  uteke_list: async (p) => {
    const multi = Array.isArray(p.namespaces) && p.namespaces.length > 0 ? (p.namespaces as string[]) : null;
    const single = p.namespace as string | null;
    const limit = typeof p.limit === 'number' ? p.limit : 50;
    const offset = typeof p.offset === 'number' ? p.offset : 0;
    // Fan-out dipakai juga untuk satu/semua namespace agar dedup+sort konsisten
    // dengan perilaku desktop (commands.rs:uteke_list).
    return listMultiNamespace(multi ?? (single ? [single] : null), (p.tag as string) ?? null, limit, offset);
  },
  uteke_search: async (p): Promise<SearchResult[]> => {
    if (!(await health()).ok) return [];
    const rows = await req<Array<{ memory: UtekeMemoryRaw & { id: string; content: string }; score: number }>>('POST', '/search', {
      body: body({ query: p.query, namespace: p.namespace ?? null, limit: typeof p.limit === 'number' ? p.limit : 20 }) as Payload,
    });
    return rows.map((r) => ({ id: r.memory.id, content: r.memory.content, score: r.score, tags: r.memory.tags ?? [] }));
  },
  uteke_neighbors: async (p) => semanticNeighbors(String(p.id), typeof p.limit === 'number' ? p.limit : 20),

  // Trust feedback
  memory_feedback: async (p): Promise<MemoryFeedbackResponse> =>
    req<MemoryFeedbackResponse>('POST', '/memory/feedback', { body: { id: p.id, feedback: p.feedback } }),

  // Uteke server integration
  uteke_server_status: async () => {
    const h = await health();
    if (!h.ok) return { available: false, hint: "Jalankan 'uteke serve' untuk pencarian semantik & auto-linking" };
    const s = await req<{ total_memories: number; unique_tags: number; db_size_bytes: number; hot: number; warm: number; cold: number }>('GET', '/stats').catch(() => null);
    return { available: true, url: API, stats: s ?? undefined };
  },
  uteke_recall: async (p): Promise<Array<MemoryEntry & { score: number }>> => {
    if (!(await health()).ok) return [];
    const rows = await recallRows(String(p.query), p.namespace as string | null, typeof p.limit === 'number' ? p.limit : 20);
    return rows.map((r) => ({ ...toMemory(r.memory), score: r.score }));
  },
  recall_unified: async (p): Promise<UnifiedSearchResult[]> => {
    const stype = (p.searchType as string) ?? 'all';
    const rows = await req<Array<Record<string, unknown>>>('POST', '/recall', {
      body: body({ query: p.query, search_type: stype, namespace: p.namespace ?? null, limit: typeof p.limit === 'number' ? p.limit : 20 }) as Payload,
    });
    return rows as unknown as UnifiedSearchResult[];
  },
  uteke_remember: rememberWithDupCheck,
  uteke_forget: async (p) => { await req('DELETE', '/forget', { query: { id: p.id as string } }); },
  uteke_server_graph: serverGraph,
  uteke_server_stats: async () => {
    if (!(await health()).ok) return { available: false, hint: 'Server tidak terjangkau' };
    const s = await req<Record<string, unknown>>('GET', '/stats');
    return { ...s, available: true };
  },
  uteke_recent: async (p) => {
    const ns = p.namespace as string | null;
    return listMultiNamespace(ns ? [ns] : null, null, typeof p.limit === 'number' ? p.limit : 10, 0);
  },

  // Agents (desktop filesystem probing — stub untuk web)
  detect_agents: async () => [],
  generate_agent_md: async () => { throw new Error("Generate AGENT.md memeriksa filesystem lokal — hanya tersedia di aplikasi desktop."); },
  run_dream_cycle: async (p): Promise<DreamHistoryRow & { success: boolean; dry_run: boolean; hint?: string }> => {
    const started = Date.now();
    const result = await req<Record<string, unknown>>('POST', '/dream', {
      body: body({ dry_run: Boolean(p.dryRun), namespace: (p.namespace as string) ?? undefined }) as Payload,
    }).catch((e: unknown): Record<string, unknown> => ({ error: String(e) }));
    const durationMs = Date.now() - started;
    const phases = (Array.isArray(result.phases) ? result.phases : []) as DreamHistoryRow['phases'];
    const row: DreamHistoryRow & { success: boolean; dry_run: boolean; hint?: string } = {
      id: Date.now(),
      ran_at: new Date().toISOString(),
      success: !result.error,
      phases,
      total_changes: Number(result.total_changes ?? phases.reduce((a, x) => a + Number(x.changes ?? 0), 0)),
      total_warnings: Number(result.total_warnings ?? phases.reduce((a, x) => a + Number(x.warnings ?? 0), 0)),
      total_errors: result.error ? 1 : 0,
      duration_ms: durationMs,
      dry_run: Boolean(p.dryRun),
      hint: (result.error as string | undefined) ?? (result.hint as string | undefined),
    };
    if (!result.error) {
      const history = lsGet<DreamHistoryRow[]>(LS_DREAM_HISTORY, []);
      history.unshift(row);
      lsSet(LS_DREAM_HISTORY, history.slice(0, 50));
    }
    return row;
  },
  get_dream_history: async (p) => lsGet<DreamHistoryRow[]>(LS_DREAM_HISTORY, []).slice(0, typeof p.limit === 'number' ? p.limit : 50),

  // Connections — satu profil implisit via proxy
  list_connections: async () => {
    const info = await probeHealthInfo();
    return [primaryConnection(info.success ? 'connected' : 'unreachable')];
  },
  add_connection: async () => 'web-proxy',
  update_connection: async () => {},
  delete_connection: async () => {},
  test_connection: async (): Promise<HealthInfo> => probeHealthInfo(),
  set_primary_connection: async () => {},
  reconnect_connection: async (): Promise<HealthInfo> => probeHealthInfo(),
  disconnect_connection: async () => {},

  // Documents
  uteke_version_status: async () => {
    const h = await health();
    return { current: h.version, required: '0.7.0', supported: h.ok ? versionMeets(h.version, '0.7.0') : false };
  },
  uteke_self_update: async () => { throw new Error("'uteke upgrade' hanya bisa dijalankan dari aplikasi desktop."); },
  doc_list: async (p): Promise<DocEntry[]> =>
    req<DocEntry[]>('POST', '/doc/list', {
      body: body({ limit: (p.limit as number) ?? 1000, roots_only: p.roots_only ?? undefined, parent: p.parent ?? undefined }) as Payload,
    }),
  doc_get: async (p) => req<DocEntry>('POST', '/doc/get', { body: body({ slug: p.slug ?? undefined, id: p.id ?? undefined }) as Payload }),
  doc_create: async (p) => req<DocEntry>('POST', '/doc/create', { body: body({ slug: p.slug, title: p.title, content: p.content, tags: p.tags ?? undefined, parent: p.parent ?? undefined }) as Payload }),
  doc_update: async (p) => {
    // Heuristik uuid-vs-slug dari uteke_client.rs
    const isUuid = (v: string) => /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(v);
    const key: Payload = p.id && isUuid(String(p.id)) ? { id: p.id } : { slug: p.slug ?? p.id };
    return req<DocEntry>('POST', '/doc/update', { body: { ...key, title: p.title ?? undefined, content: p.content ?? undefined, tags: p.tags ?? undefined } });
  },
  doc_search: async (p): Promise<DocSearchResult[]> =>
    req<DocSearchResult[]>('POST', '/doc/search', {
      body: { query: p.query, limit: (p.limit as number) ?? undefined, mode: (p.mode as string) ?? undefined },
    }),
  doc_delete: async (p) => { await req('DELETE', '/doc/delete', { query: { id: (p.id as string) ?? undefined, slug: (p.slug as string) ?? undefined } }); },
  doc_move: async (p) => req<unknown>('POST', '/doc/move', { body: body({ id: p.id ?? undefined, slug: p.slug ?? undefined, new_parent: p.new_parent ?? undefined }) as Payload }),

  // Cross-entity linking
  memory_doc_refs: async (p): Promise<MemoryDocRefsResponse> =>
    req<MemoryDocRefsResponse>('POST', '/memory/doc-refs', { body: { memory_id: p.memoryId } }),
  doc_mem_refs: async (p): Promise<DocMemRefsResponse> =>
    req<DocMemRefsResponse>('POST', '/doc/mem-refs', { body: { doc_slug: p.docSlug } }),

  // Timeline
  memory_timeline: async (p): Promise<TimelineEvent[]> =>
    req<TimelineEvent[]>('GET', '/timeline', { query: { id: p.id as string, limit: (p.limit as number) ?? 50 } }),

  // Lifecycle
  lifecycle_status: async (p): Promise<LifecycleStatus> =>
    req<LifecycleStatus>('GET', '/lifecycle/status', { query: { namespace: (p.namespace as string) ?? undefined } }),
  lifecycle_cycle: async (p): Promise<LifecycleCycleResult> =>
    req<LifecycleCycleResult>('POST', '/lifecycle/cycle', { body: body({ namespace: (p.namespace as string) ?? undefined }) as Payload }),
  lifecycle_promote: async (p) => req<unknown>('POST', '/lifecycle/promote', { body: { id: p.id } }),
  lifecycle_deprecated: async (p): Promise<DeprecatedListResponse> =>
    req<DeprecatedListResponse>('GET', '/lifecycle/deprecated', { query: { namespace: (p.namespace as string) ?? undefined, limit: (p.limit as number) ?? undefined } }),
  find_orphans: async (p): Promise<OrphanMemory[]> =>
    req<OrphanMemory[]>('POST', '/orphans', { body: body({ namespace: (p.namespace as string) ?? undefined }) as Payload }),
  consolidate_memories: async (p) =>
    req<unknown>('POST', '/consolidate', {
      body: { threshold: (p.threshold as number) ?? undefined, dry_run: p.dryRun !== false, namespace: (p.namespace as string) ?? undefined },
    }),

  // Endpoint-gap wrappers (#216 + #231)
  memory_update: async (p) =>
    req<Record<string, unknown>>('PUT', '/memory', {
      body: body({
        id: p.id, content: p.content ?? null, tags: p.tags ?? null, metadata: p.metadata ?? null,
        importance: p.importance ?? null, pinned: p.pinned ?? null, memory_type: p.memoryType ?? null,
      }) as Payload,
    }),
  room_remember: async (p) =>
    req<Record<string, unknown>>('POST', '/room/remember', {
      body: body({ room_id: p.roomId, content: p.content, tags: p.tags ?? [], namespace: (p.namespace as string) ?? undefined, type: (p.memoryType as string) ?? undefined, author: (p.author as string) ?? undefined }) as Payload,
    }),
  uteke_import: async (p): Promise<ImportResult> =>
    req<ImportResult>('POST', '/import', { body: { content: p.jsonlContent, namespace: (p.namespace as string) ?? undefined } }),
  uteke_export: async (p) => reqText('GET', '/export', { query: { namespace: (p.namespace as string) ?? undefined } }),
  uteke_context: async (p) =>
    reqText('POST', '/context', { body: body({ namespace: (p.namespace as string) ?? undefined }) }),
  room_doc_list: async (p) => (await req<{ doc_slugs: string[] }>('POST', '/room/document/list', { body: { room_id: p.roomId } })).doc_slugs,
  room_doc_add: async (p) => { await req('PUT', '/room/document/add', { body: { room_id: p.roomId, doc_slug: p.docSlug } }); },
  room_doc_remove: async (p) => { await req('DELETE', '/room/document/remove', { body: { room_id: p.roomId, doc_slug: p.docSlug } }); },
  doc_room_list: async (p) => (await req<{ room_ids: string[] }>('POST', '/doc/room/list', { body: { doc_slug: p.docSlug } })).room_ids,
};
