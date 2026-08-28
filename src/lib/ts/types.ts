// View type for navigation
export type View = 'dashboard' | 'memories' | 'namespaces' | 'graph' | 'rooms' | 'documents' | 'lifecycle' | 'settings' | 'tools';

// Memory entry from uteke-serve (HTTP API)
export interface MemoryEntry {
  id: string;
  content: string;
  tags: string[];
  content_type: string | null;
  importance: number | null;
  namespace: string | null;
  created_at: string | null;
  updated_at: string | null;
}

// Search result with score
export interface SearchResult {
  id: string;
  content: string;
  score: number;
  tags: string[];
}

/**
 * A unified search hit from `/recall` with `search_type` set (uteke ≥ 0.9.0).
 * Can be a memory or a document chunk — check `result_type`.
 */
export interface UnifiedSearchResult {
  result_type: 'memory' | 'document';
  score: number;
  content: string;
  memory_id?: string;
  doc_slug?: string;
  doc_title?: string;
  chunk_heading?: string;
  chunk_snippet?: string;
  tags: string[];
  namespace?: string;
  memory_type?: string;
  importance?: number;
  pinned?: boolean;
  source?: string;
  metadata?: unknown;
}

// Graph edge
export interface GraphEdge {
  id: number | null;
  source: string;
  target: string;
  weight: number | null;
}

// Full graph data (nodes + edges)
export interface GraphData {
  nodes: MemoryEntry[];
  edges: GraphEdge[];
}

// Room entry
export interface RoomEntry {
  id: string;
  name: string;
  participant_count: number;
  memory_count: number;
  created_at: string | null;
}

// Installed uteke version vs the minimum required for Documents.
export interface VersionStatus {
  /** Detected "X.Y.Z", or null if the uteke CLI couldn't be probed. */
  current: string | null;
  /** Minimum uteke version supporting global documents ("0.7.0"). */
  required: string;
  /** True iff current >= required (false when current is unknown). */
  supported: boolean;
}

// Document entry from uteke-serve /doc/* API
export interface DocEntry {
  id: string;
  slug: string;
  title: string;
  content?: string;
  tags?: string[];
  metadata?: Record<string, unknown>;
  version?: number | null;
  content_type?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
  parent_id?: string | null;
  /** Materialized path string (e.g. "/uuid/uuid/") from uteke-core. */
  path?: string | null;
  depth?: number | null;
  sort_order?: number | null;
  has_children?: boolean | null;
}

// Document search result
export interface DocSearchResult {
  document: DocEntry;
  chunk_heading: string | null;
  chunk_snippet: string | null;
  score: number;
  mode: string | null;
}

// Cross-entity linking (#207): documents that reference a memory.
export interface MemoryDocRefsResponse {
  memory_id: string;
  doc_slugs: string[];
}

// Cross-entity linking (#207): memories that reference a document.
export interface DocMemRefsResponse {
  doc_slug: string;
  memory_ids: string[];
}

// Timeline event for a memory (created, updated, recalled, etc.)
export interface TimelineEvent {
  id: number;
  memory_id: string;
  event_type: string;
  event_data: string | null;
  created_at: string;
}

// Stats response
export interface StatsResponse {
  total_memories: number;
  total_namespaces: number;
  total_tags: number;
  total_edges: number;
  db_size_bytes: number;
}

// Trust feedback response (POST /memory/feedback)
export interface MemoryFeedbackResponse {
  id: string;
  feedback: string;
  delta: number;
  importance: number;
}

// ─────────────────────────────────────────────────────────────────
// Lifecycle types (uteke ≥ 0.13.0) — issue #227, #228
// ─────────────────────────────────────────────────────────────────

/** Response from GET /lifecycle/status */
export interface LifecycleStatus {
  active: number;
  deprecated: number;
  pruned: number;
}

/** Response from POST /lifecycle/cycle */
export interface LifecycleCycleResult {
  deprecated: number;
  pruned: number;
  skipped: number;
}

/** Orphaned memory from POST /orphans */
export interface OrphanMemory {
  id: string;
  content: string;
  tags: string[];
  namespace: string;
  importance: number;
  created_at: string;
}

/** One deprecated-memory entry from GET /lifecycle/deprecated */
export interface DeprecatedMemoryInfo {
  id: string;
  content: string;
  memory_type: string;
  namespace: string;
  tags: string[];
  importance: number;
  deprecated_at: string | null;
  deprecate_reason: string | null;
}

/** Response from GET /lifecycle/deprecated */
export interface DeprecatedListResponse {
  deprecated: DeprecatedMemoryInfo[];
  count: number;
}

// ── Endpoint Gap Types (#216 + #231) ────────────────────────────────────

// Import result (POST /import)
export interface ImportResult {
  imported: number;
  skipped: number;
}

// Memory update params (PUT /memory) — all fields optional except id
export interface MemoryUpdateParams {
  id: string;
  content?: string;
  tags?: string[];
  metadata?: Record<string, unknown>;
  importance?: number;
  pinned?: boolean;
  memory_type?: string;
}

// Room remember params (POST /room/remember)
export interface RoomRememberParams {
  room_id: string;
  content: string;
  tags: string[];
  namespace?: string;
  memory_type?: string;
  author?: string;
}

// ── Connection manager types (#37) ───────────────────────────────────────
// Moved here from ipc.ts so both transports (desktop/web-routes) share them.

export interface ConnectionInfo {
  id: string;
  name: string;
  product_type: 'uteke';
  url: string;
  has_token: boolean;
  capabilities: { read: boolean; write: boolean; search: boolean; realtime: boolean };
  status: string;
  is_primary: boolean;
  created_at: string;
  last_tested_at: string | null;
}

export interface HealthInfo {
  success: boolean;
  latency_ms: number;
  version: string | null;
  error: string | null;
}
