import { call, isWebMode } from './transport';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import type {
  MemoryEntry, SearchResult, UnifiedSearchResult, GraphData, GraphEdge,
  RoomEntry, StatsResponse, DocEntry, DocSearchResult, VersionStatus,
  MemoryDocRefsResponse, DocMemRefsResponse, MemoryFeedbackResponse,
  TimelineEvent,
  LifecycleStatus, LifecycleCycleResult, OrphanMemory, DeprecatedListResponse,
  ImportResult, MemoryUpdateParams, RoomRememberParams,
} from './types';

export const memory = {
  remember: (content: string, opts?: {
    tags?: string[];
    namespace?: string;
    content_type?: string;
    importance?: number;
  }) => call<string>('remember', {
    content,
    tags: opts?.tags ?? [],
    namespace: opts?.namespace ?? null,
    content_type: opts?.content_type ?? null,
    importance: opts?.importance ?? null,
  }),
  recall: (query: string, opts?: { namespace?: string; limit?: number }) =>
    call<SearchResult[]>('recall', { query, namespace: opts?.namespace ?? null, limit: opts?.limit ?? null }),
  search: (query: string, opts?: { namespace?: string; limit?: number }) =>
    call<SearchResult[]>('search', { query, namespace: opts?.namespace ?? null, limit: opts?.limit ?? null }),
  list: (opts?: { namespace?: string; tag?: string; limit?: number; offset?: number }) =>
    call<MemoryEntry[]>('list', {
      namespace: opts?.namespace ?? null,
      tag: opts?.tag ?? null,
      limit: opts?.limit ?? null,
      offset: opts?.offset ?? null,
    }),
  forget: (id: string) => call<void>('forget', { id }),
  get: (id: string) => call<MemoryEntry>('get_memory', { id }),
};

export const graph = {
  getData: (opts?: { namespace?: string; limit?: number }) =>
    call<GraphData>('get_graph_data', { namespace: opts?.namespace ?? null, limit: opts?.limit ?? null }),
  getNeighbors: (id: string, depth?: number) =>
    call<MemoryEntry[]>('get_neighbors', { id, depth: depth ?? null }),
  addEdge: (source: string, target: string, opts?: { edgeType?: string; weight?: number }) =>
    call<number>('add_edge', {
      source,
      target,
      edge_type: opts?.edgeType ?? null,
      weight: opts?.weight ?? null,
    }),
  removeEdge: (id: number) => call<void>('remove_edge', { id }),
};

export const room = {
  list: () => call<RoomEntry[]>('list_rooms'),
  getSummary: (id: string) => call<string>('get_room_summary', { roomId: id }),
  getDocument: (id: string) => call<string>('get_room_document', { roomId: id }),
  create: (name: string, opts?: { namespace?: string; tags?: string[] }) =>
    call<string>('create_room', {
      name,
      namespace: opts?.namespace ?? null,
      tags: opts?.tags ?? null,
    }),
  delete: (id: string) => call<void>('delete_room', { roomId: id }),
};

export const system = {
  stats: () => call<StatsResponse>('stats'),
  listNamespaces: () => call<string[]>('list_namespaces'),
  listTags: (namespace?: string) => call<Record<string, number>>('list_tags', { namespace: namespace ?? null }),
  getSettings: () => call<Record<string, string>>('get_settings'),
  setSettings: (settings: Record<string, string>) => call<void>('set_settings', { settings }),
  exportData: (format: 'json' | 'markdown' | 'csv', namespace?: string | null) => call<string>('export_data', { format, namespace: namespace ?? null }),
  importPreview: (format: 'json' | 'markdown', data: string) => call<{ format: string; memories: number; edges: number; rooms: number; namespaces: string[]; tags?: string[] }>('import_preview', { format, data }),
  importData: (format: 'json' | 'markdown', data: string) => call<number>('import_data', { format, data }),
  openDataDir: () => call<string>('init_data_dir'), // returns path, doesn't open file manager
};

// Tauri event listener
export { listen } from '@tauri-apps/api/event';
export type { UnlistenFn } from '@tauri-apps/api/event';

// Updater (desktop-only; plugin-updater throws in a plain browser)
import { check } from '@tauri-apps/plugin-updater';
export const updater = {
  check: () => (isWebMode ? Promise.resolve(null) : check()),
};

// Uteke Integration (read-only)
export const uteke = {
  available: () => call<boolean>('uteke_available'),
  get: (id: string) => call<MemoryEntry>('uteke_get', { id }),
  graph: (opts?: { namespace?: string; limit?: number }) =>
    call<GraphData>('uteke_graph', {
      namespace: opts?.namespace ?? null,
      limit: opts?.limit ?? null,
    }),
  namespaces: () => call<string[]>('uteke_namespaces'),
  namespacesWithCounts: () => call<Array<{ name: string; count: number }>>('uteke_namespaces_with_counts'),
  rooms: (namespace?: string) =>
    call<{ id: string; title: string | null; namespace: string; memory_count: number; participant_count: number; created_at: string; updated_at: string }[]>('uteke_rooms', {
      namespace: namespace ?? null,
    }),
  roomRecall: (roomId: string, limit?: number) =>
    call<MemoryEntry[]>('uteke_room_recall', { roomId, limit: limit ?? null }),
  roomMemories: (roomId: string, opts?: { limit?: number; author?: string }) =>
    call<MemoryEntry[]>('uteke_room_memories', {
      roomId,
      limit: opts?.limit ?? null,
      author: opts?.author ?? null,
    }),
  roomStats: (roomId: string) =>
    call<{ memory_count: number; participant_count: number; participant_namespaces?: string[] }>('uteke_room_stats', { roomId }),
  list: (opts?: { namespace?: string; namespaces?: string[]; tag?: string; limit?: number; offset?: number }) =>
    call<MemoryEntry[]>('uteke_list', {
      namespace: opts?.namespace ?? null,
      namespaces: opts?.namespaces ?? null,
      tag: opts?.tag ?? null,
      limit: opts?.limit ?? null,
      offset: opts?.offset ?? null,
    }),
  search: (query: string, opts?: { namespace?: string; limit?: number }) =>
    call<SearchResult[]>('uteke_search', {
      query,
      namespace: opts?.namespace ?? null,
      limit: opts?.limit ?? null,
    }),
  stats: () => call<StatsResponse>('uteke_stats'),
  neighbors: (id: string, limit?: number) =>
    call<{ id: string; content: string; tags: string[]; namespace: string | null; importance: number | null; content_type: string | null; created_at: string | null; relationship: string; score: number | null; shared_tags: string[] }[]>('uteke_neighbors', { id, limit: limit ?? null }),
};

// Trust feedback (POST /memory/feedback)
export const memoryFeedback = (id: string, feedback: 'helpful' | 'unhelpful') =>
  call<MemoryFeedbackResponse>('memory_feedback', { id, feedback });

// Uteke Server Integration (HTTP — semantic search, auto-linking)
export const utekeServer = {
  status: () => call<{
    available: boolean;
    url?: string;
    hint?: string;
    stats?: { total_memories: number; unique_tags: number; db_size_bytes: number; hot: number; warm: number; cold: number };
  }>('uteke_server_status'),

  recall: (query: string, opts?: { namespace?: string; limit?: number }) =>
    call<Array<MemoryEntry & { score: number }>>('uteke_recall', {
      query,
      namespace: opts?.namespace ?? null,
      limit: opts?.limit ?? null,
    }),
  /** Unified semantic search across memories AND documents (uteke ≥ 0.9.0). */
  recallUnified: (
    query: string,
    opts?: { searchType?: 'all' | 'memory' | 'document'; namespace?: string; limit?: number },
  ) =>
    call<UnifiedSearchResult[]>('recall_unified', {
      query,
      searchType: opts?.searchType ?? null,
      namespace: opts?.namespace ?? null,
      limit: opts?.limit ?? null,
    }),

  remember: (content: string, opts?: { tags?: string[]; namespace?: string }) =>
    call<{ id?: string; duplicate: boolean; existing_id?: string; existing_content?: string; score?: number; hint?: string }>('uteke_remember', {
      content,
      tags: opts?.tags ?? null,
      namespace: opts?.namespace ?? null,
    }),

  forget: (id: string) => call<void>('uteke_forget', { id }),

  graph: (namespace?: string, namespaces?: string[]) =>
    call<{
      nodes: Array<{ id: string; label: string; entity_type: string | null }>;
      edges: Array<{ source: string; target: string; relation: string; weight: number }>;
      stats: { node_count: number; edge_count: number; relation_types: string[] };
      hint?: string;
    }>('uteke_server_graph', { namespace: namespace ?? null, namespaces: namespaces ?? null }),

  stats: () => call<{
    total_memories?: number;
    unique_tags?: number;
    db_size_bytes?: number;
    hot?: number;
    warm?: number;
    cold?: number;
    available?: boolean;
    hint?: string;
  }>('uteke_server_stats'),

  recent: (opts?: { namespace?: string | null; limit?: number }) =>
    call<MemoryEntry[]>('uteke_recent', {
      namespace: opts?.namespace ?? null,
      limit: opts?.limit ?? null,
    }),
};

// AI Agent Integration (#55)
export const agents = {
  detect: () => call<Array<{ name: string; config_path: string; found: boolean }>>('detect_agents'),
  generateAgentMd: (projectDir?: string) => call<string>('generate_agent_md', { projectDir: projectDir ?? null }),
  runDream: (opts?: { namespace?: string; dryRun?: boolean }) =>
    call<{
      success: boolean;
      phases: Array<{ phase: string; status: string; summary: string; changes: number; warnings: number }>;
      total_changes: number;
      total_warnings: number;
      total_errors: number;
      dry_run: boolean;
      duration_ms: number;
      hint?: string;
    }>('run_dream_cycle', {
      namespace: opts?.namespace ?? null,
      dryRun: opts?.dryRun ?? null,
    }),
  getDreamHistory: (limit?: number) =>
    call<Array<{
      id: number;
      ran_at: string;
      success: boolean;
      total_changes: number;
      total_warnings: number;
      total_errors: number;
      duration_ms: number;
      phases: Array<{ phase: string; status: string; summary: string; changes: number; warnings: number }>;
    }>>('get_dream_history', { limit: limit ?? null }),
};

// Connection Manager (#37) — types live in types.ts (shared with web mode)
import type { ConnectionInfo, HealthInfo } from './types';
export type { ConnectionInfo, HealthInfo };

export const connection = {
  list: () => call<ConnectionInfo[]>('list_connections'),
  add: (opts: {
    name: string;
    productType: string;
    url: string;
    authToken?: string;
    authType?: string;
  }) => call<string>('add_connection', opts),
  update: (opts: {
    id: string;
    name?: string;
    url?: string;
    authToken?: string;
    authType?: string;
  }) => call<void>('update_connection', opts),
  delete: (id: string) => call<void>('delete_connection', { id }),
  test: (id: string) => call<HealthInfo>('test_connection', { id }),
  setPrimary: (id: string) => call<void>('set_primary_connection', { id }),
  reconnect: (id: string) => call<HealthInfo>('reconnect_connection', { id }),
  disconnect: () => call<void>('disconnect_connection'),
};

// Document Engine (#137) — uteke-serve /doc/* API
//
// Since uteke v0.7.0 (#614) documents are global — no namespace. The backend
// gates all doc commands on uteke >= 0.7.0; use `versionStatus()` to detect an
// outdated install and `selfUpdate()` to run `uteke upgrade`.
export const docs = {
  /** Installed uteke version + whether it meets the Documents requirement. */
  versionStatus: () => call<VersionStatus>('uteke_version_status'),

  /** Run `uteke upgrade`, then re-detect. Resolves with the new status. */
  selfUpdate: () => call<VersionStatus>('uteke_self_update'),

  list: (opts?: { limit?: number; roots_only?: boolean; parent?: string }) =>
    call<DocEntry[]>('doc_list', {
      limit: opts?.limit ?? null,
      roots_only: opts?.roots_only ?? null,
      parent: opts?.parent ?? null,
    }),

  get: (opts: { slug?: string; id?: string }) =>
    call<DocEntry>('doc_get', {
      slug: opts.slug ?? null,
      id: opts.id ?? null,
    }),

  create: (slug: string, title: string, content: string, opts?: { tags?: string[]; parent?: string }) =>
    call<DocEntry>('doc_create', {
      slug,
      title,
      content,
      tags: opts?.tags ?? null,
      parent: opts?.parent ?? null,
    }),

  /** Update an existing document (by id or slug). */
  update: (opts: {
    id?: string;
    slug?: string;
    title?: string;
    content?: string;
    tags?: string[];
  }) =>
    call<DocEntry>('doc_update', {
      id: opts.id ?? null,
      slug: opts.slug ?? null,
      title: opts.title ?? null,
      content: opts.content ?? null,
      tags: opts.tags ?? null,
    }),

  search: (query: string, opts?: { limit?: number; mode?: string }) =>
    call<DocSearchResult[]>('doc_search', {
      query,
      limit: opts?.limit ?? null,
      mode: opts?.mode ?? null,
    }),

  delete: (opts: { id?: string; slug?: string }) =>
    call<void>('doc_delete', {
      id: opts.id ?? null,
      slug: opts.slug ?? null,
    }),

  move: (opts: { id?: string; slug?: string; new_parent?: string }) =>
    call<unknown>('doc_move', {
      id: opts.id ?? null,
      slug: opts.slug ?? null,
      new_parent: opts.new_parent ?? null,
    }),

  /** Export document content as a downloadable .md file.
   *  Desktop: native save dialog. Web: browser Blob download. */
  exportFile: async (doc: DocEntry) => {
    const content = doc.content ?? '';
    const filename = `${doc.slug || doc.title || 'document'}.md`;
    if (isWebMode) {
      const url = URL.createObjectURL(new Blob([content], { type: 'text/markdown' }));
      const a = document.createElement('a');
      a.href = url;
      a.download = filename;
      a.click();
      URL.revokeObjectURL(url);
      return;
    }
    const filePath = await save({
      defaultPath: filename,
      filters: [{ name: 'Markdown', extensions: ['md'] }],
    });
    if (!filePath) return; // user cancelled
    await writeTextFile(filePath, content);
  },
};

// Cross-entity linking (#207) — memory ↔ document references.
// Top-level (not nested in an object) since these bridge two domains.

/** Documents that reference a given memory (POST /memory/doc-refs). */
export async function memoryDocRefs(memoryId: string): Promise<MemoryDocRefsResponse> {
  return call<MemoryDocRefsResponse>('memory_doc_refs', { memoryId });
}

/** Memories that reference a given document (POST /doc/mem-refs). */
export async function docMemRefs(docSlug: string): Promise<DocMemRefsResponse> {
  return call<DocMemRefsResponse>('doc_mem_refs', { docSlug });
}

// Memory timeline (GET /timeline?id=...&limit=...)
// Returns chronological event history for a memory (created, updated, recalled, etc.)

/** Fetch timeline events for a memory. Returns empty array if server is unavailable. */
export async function memoryTimeline(id: string, limit = 50): Promise<TimelineEvent[]> {
  return call<TimelineEvent[]>('memory_timeline', { id, limit });
}

// ─────────────────────────────────────────────────────────────────
// Lifecycle IPC wrappers (uteke ≥ 0.13.0) — issues #227, #228
// ─────────────────────────────────────────────────────────────────

/** Get lifecycle status: counts of active vs deprecated memories. */
export async function lifecycleStatus(namespace?: string): Promise<LifecycleStatus> {
  return call<LifecycleStatus>('lifecycle_status', { namespace: namespace ?? null });
}

/** Run lifecycle cycle: deprecate aged memories, prune expired ones. */
export async function lifecycleCycle(namespace?: string): Promise<LifecycleCycleResult> {
  return call<LifecycleCycleResult>('lifecycle_cycle', { namespace: namespace ?? null });
}

/** Restore a deprecated memory back to active. */
export async function lifecyclePromote(id: string): Promise<unknown> {
  return call('lifecycle_promote', { id });
}

/** List deprecated memories (the recycle bin). */
export async function lifecycleDeprecated(
  namespace?: string,
  limit?: number,
): Promise<DeprecatedListResponse> {
  return call<DeprecatedListResponse>('lifecycle_deprecated', {
    namespace: namespace ?? null,
    limit: limit ?? null,
  });
}

/** Find orphaned memories (no room, no edges). */
export async function findOrphans(namespace?: string): Promise<OrphanMemory[]> {
  return call<OrphanMemory[]>('find_orphans', { namespace: namespace ?? null });
}

/** Consolidate (merge preview) similar memories. Always dry_run from Corin. */
export async function consolidateMemories(opts?: {
  threshold?: number;
  dryRun?: boolean;
  namespace?: string;
}): Promise<unknown> {
  return call('consolidate_memories', {
    threshold: opts?.threshold ?? null,
    dryRun: opts?.dryRun ?? true,
    namespace: opts?.namespace ?? null,
  });
}

// ── Endpoint Gap IPC Wrappers (#216 + #231) ─────────────────────────────

// #216 — Fill UtekeClient gaps

/** Update a memory (PUT /memory). Returns updated memory as JSON. */
export async function memoryUpdate(params: MemoryUpdateParams): Promise<Record<string, unknown>> {
  return call<Record<string, unknown>>('memory_update', {
    id: params.id,
    content: params.content ?? null,
    tags: params.tags ?? null,
    metadata: params.metadata ?? null,
    importance: params.importance ?? null,
    pinned: params.pinned ?? null,
    memoryType: params.memory_type ?? null,
  });
}

/** Remember into a room (POST /room/remember). Returns created memory as JSON. */
export async function roomRemember(params: RoomRememberParams): Promise<Record<string, unknown>> {
  return call<Record<string, unknown>>('room_remember', {
    roomId: params.room_id,
    content: params.content,
    tags: params.tags,
    namespace: params.namespace ?? null,
    memoryType: params.memory_type ?? null,
    author: params.author ?? null,
  });
}

/** Import JSONL data (POST /import). Returns import/skip counts. */
export async function utekeImport(jsonlContent: string, namespace?: string): Promise<ImportResult> {
  return call<ImportResult>('uteke_import', {
    jsonlContent,
    namespace: namespace ?? null,
  });
}

/** Export memories as JSONL (GET /export). Returns JSONL string. */
export async function utekeExport(namespace?: string): Promise<string> {
  return call<string>('uteke_export', { namespace: namespace ?? null });
}

/** Build context summary (POST /context). Returns context text. */
export async function utekeContext(namespace?: string): Promise<string> {
  return call<string>('uteke_context', { namespace: namespace ?? null });
}

// #231 — Room-Document linking

/** List documents linked to a room (POST /room/document/list). */
export async function roomDocList(roomId: string): Promise<string[]> {
  return call<string[]>('room_doc_list', { roomId });
}

/** Link a document to a room (PUT /room/document/add). */
export async function roomDocAdd(roomId: string, docSlug: string): Promise<void> {
  await call('room_doc_add', { roomId, docSlug });
}

/** Unlink a document from a room (DELETE /room/document/remove). */
export async function roomDocRemove(roomId: string, docSlug: string): Promise<void> {
  await call('room_doc_remove', { roomId, docSlug });
}

/** List rooms linked to a document (POST /doc/room/list). */
export async function docRoomList(docSlug: string): Promise<string[]> {
  return call<string[]>('doc_room_list', { docSlug });
}
