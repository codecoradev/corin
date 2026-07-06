import { invoke } from '@tauri-apps/api/core';
import type {
  MemoryEntry, SearchResult, GraphData, GraphEdge,
  RoomEntry, StatsResponse,
} from './types';

export const memory = {
  remember: (content: string, opts?: {
    tags?: string[];
    namespace?: string;
    content_type?: string;
    importance?: number;
  }) => invoke<string>('remember', {
    content,
    tags: opts?.tags ?? [],
    namespace: opts?.namespace ?? null,
    content_type: opts?.content_type ?? null,
    importance: opts?.importance ?? null,
  }),
  recall: (query: string, opts?: { namespace?: string; limit?: number }) =>
    invoke<SearchResult[]>('recall', { query, namespace: opts?.namespace ?? null, limit: opts?.limit ?? null }),
  search: (query: string, opts?: { namespace?: string; limit?: number }) =>
    invoke<SearchResult[]>('search', { query, namespace: opts?.namespace ?? null, limit: opts?.limit ?? null }),
  list: (opts?: { namespace?: string; tag?: string; limit?: number; offset?: number }) =>
    invoke<MemoryEntry[]>('list', {
      namespace: opts?.namespace ?? null,
      tag: opts?.tag ?? null,
      limit: opts?.limit ?? null,
      offset: opts?.offset ?? null,
    }),
  forget: (id: string) => invoke<void>('forget', { id }),
  get: (id: string) => invoke<MemoryEntry>('get_memory', { id }),
};

export const graph = {
  getData: (opts?: { namespace?: string; limit?: number }) =>
    invoke<GraphData>('get_graph_data', { namespace: opts?.namespace ?? null, limit: opts?.limit ?? null }),
  getNeighbors: (id: string, depth?: number) =>
    invoke<MemoryEntry[]>('get_neighbors', { id, depth: depth ?? null }),
  addEdge: (source: string, target: string, opts?: { edgeType?: string; weight?: number }) =>
    invoke<number>('add_edge', {
      source,
      target,
      edge_type: opts?.edgeType ?? null,
      weight: opts?.weight ?? null,
    }),
  removeEdge: (id: number) => invoke<void>('remove_edge', { id }),
};

export const room = {
  list: () => invoke<RoomEntry[]>('list_rooms'),
  getSummary: (id: string) => invoke<string>('get_room_summary', { roomId: id }),
  getDocument: (id: string) => invoke<string>('get_room_document', { roomId: id }),
  create: (name: string, opts?: { namespace?: string; tags?: string[] }) =>
    invoke<string>('create_room', {
      name,
      namespace: opts?.namespace ?? null,
      tags: opts?.tags ?? null,
    }),
  delete: (id: string) => invoke<void>('delete_room', { roomId: id }),
};

export const system = {
  stats: () => invoke<StatsResponse>('stats'),
  listNamespaces: () => invoke<string[]>('list_namespaces'),
  listTags: (namespace?: string) => invoke<Record<string, number>>('list_tags', { namespace: namespace ?? null }),
  getSettings: () => invoke<Record<string, string>>('get_settings'),
  setSettings: (settings: Record<string, string>) => invoke<void>('set_settings', { settings }),
  exportData: (format: 'json' | 'markdown' | 'csv', namespace?: string | null) => invoke<string>('export_data', { format, namespace: namespace ?? null }),
  importPreview: (format: 'json' | 'markdown', data: string) => invoke<{ format: string; memories: number; edges: number; rooms: number; namespaces: string[]; tags?: string[] }>('import_preview', { format, data }),
  importData: (format: 'json' | 'markdown', data: string) => invoke<number>('import_data', { format, data }),
  openDataDir: () => invoke<string>('init_data_dir'), // returns path, doesn't open file manager
};

// Tauri event listener
export { listen } from '@tauri-apps/api/event';
export type { UnlistenFn } from '@tauri-apps/api/event';

// Updater
import { check } from '@tauri-apps/plugin-updater';
export const updater = {
  check: () => check(),
};

// Uteke Integration (read-only)
export const uteke = {
  available: () => invoke<boolean>('uteke_available'),
  get: (id: string) => invoke<MemoryEntry>('uteke_get', { id }),
  graph: (opts?: { namespace?: string; limit?: number }) =>
    invoke<GraphData>('uteke_graph', {
      namespace: opts?.namespace ?? null,
      limit: opts?.limit ?? null,
    }),
  namespaces: () => invoke<string[]>('uteke_namespaces'),
  namespacesWithCounts: () => invoke<Array<{ name: string; count: number }>>('uteke_namespaces_with_counts'),
  rooms: (namespace?: string) =>
    invoke<{ id: string; title: string | null; namespace: string; memory_count: number; participant_count: number; created_at: string; updated_at: string }[]>('uteke_rooms', {
      namespace: namespace ?? null,
    }),
  roomRecall: (roomId: string, limit?: number) =>
    invoke<MemoryEntry[]>('uteke_room_recall', { roomId, limit: limit ?? null }),
  roomMemories: (roomId: string, opts?: { limit?: number; author?: string }) =>
    invoke<MemoryEntry[]>('uteke_room_memories', {
      roomId,
      limit: opts?.limit ?? null,
      author: opts?.author ?? null,
    }),
  roomStats: (roomId: string) =>
    invoke<{ memory_count: number; participant_count: number; participant_namespaces?: string[] }>('uteke_room_stats', { roomId }),
  list: (opts?: { namespace?: string; namespaces?: string[]; tag?: string; limit?: number; offset?: number }) =>
    invoke<MemoryEntry[]>('uteke_list', {
      namespace: opts?.namespace ?? null,
      namespaces: opts?.namespaces ?? null,
      tag: opts?.tag ?? null,
      limit: opts?.limit ?? null,
      offset: opts?.offset ?? null,
    }),
  search: (query: string, opts?: { namespace?: string; limit?: number }) =>
    invoke<SearchResult[]>('uteke_search', {
      query,
      namespace: opts?.namespace ?? null,
      limit: opts?.limit ?? null,
    }),
  stats: () => invoke<StatsResponse>('uteke_stats'),
  neighbors: (id: string, limit?: number) =>
    invoke<{ id: string; content: string; tags: string[]; namespace: string | null; importance: number | null; content_type: string | null; created_at: string | null; relationship: string; score: number | null; shared_tags: string[] }[]>('uteke_neighbors', { id, limit: limit ?? null }),
};

// Uteke Server Integration (HTTP — semantic search, auto-linking)
export const utekeServer = {
  status: () => invoke<{
    available: boolean;
    url?: string;
    hint?: string;
    stats?: { total_memories: number; unique_tags: number; db_size_bytes: number; hot: number; warm: number; cold: number };
  }>('uteke_server_status'),

  recall: (query: string, opts?: { namespace?: string; limit?: number }) =>
    invoke<Array<MemoryEntry & { score: number }>>('uteke_recall', {
      query,
      namespace: opts?.namespace ?? null,
      limit: opts?.limit ?? null,
    }),

  remember: (content: string, opts?: { tags?: string[]; namespace?: string }) =>
    invoke<{ id?: string; duplicate: boolean; existing_id?: string; existing_content?: string; score?: number; hint?: string }>('uteke_remember', {
      content,
      tags: opts?.tags ?? null,
      namespace: opts?.namespace ?? null,
    }),

  forget: (id: string) => invoke<void>('uteke_forget', { id }),

  graph: (namespace?: string, namespaces?: string[]) =>
    invoke<{
      nodes: Array<{ id: string; label: string; entity_type: string | null }>;
      edges: Array<{ source: string; target: string; relation: string; weight: number }>;
      stats: { node_count: number; edge_count: number; relation_types: string[] };
      hint?: string;
    }>('uteke_server_graph', { namespace: namespace ?? null, namespaces: namespaces ?? null }),

  stats: () => invoke<{
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
    invoke<MemoryEntry[]>('uteke_recent', {
      namespace: opts?.namespace ?? null,
      limit: opts?.limit ?? null,
    }),
};

// AI Agent Integration (#55)
export const agents = {
  detect: () => invoke<Array<{ name: string; config_path: string; found: boolean }>>('detect_agents'),
  generateAgentMd: (projectDir?: string) => invoke<string>('generate_agent_md', { projectDir: projectDir ?? null }),
  runDream: (opts?: { namespace?: string; dryRun?: boolean }) =>
    invoke<{
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
    invoke<Array<{
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

// Connection Manager (#37)
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

export const connection = {
  list: () => invoke<ConnectionInfo[]>('list_connections'),
  add: (opts: {
    name: string;
    productType: string;
    url: string;
    authToken?: string;
    authType?: string;
  }) => invoke<string>('add_connection', opts),
  update: (opts: {
    id: string;
    name?: string;
    url?: string;
    authToken?: string;
    authType?: string;
  }) => invoke<void>('update_connection', opts),
  delete: (id: string) => invoke<void>('delete_connection', { id }),
  test: (id: string) => invoke<HealthInfo>('test_connection', { id }),
  setPrimary: (id: string) => invoke<void>('set_primary_connection', { id }),
  reconnect: (id: string) => invoke<HealthInfo>('reconnect_connection', { id }),
  disconnect: () => invoke<void>('disconnect_connection'),
};
