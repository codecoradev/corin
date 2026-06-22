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
  getSummary: (id: string) => invoke<string>('get_room_summary', { room_id: id }),
  getDocument: (id: string) => invoke<string>('get_room_document', { room_id: id }),
  create: (name: string, opts?: { namespace?: string; tags?: string[] }) =>
    invoke<string>('create_room', {
      name,
      namespace: opts?.namespace ?? null,
      tags: opts?.tags ?? null,
    }),
};

export const system = {
  stats: () => invoke<StatsResponse>('stats'),
  listNamespaces: () => invoke<string[]>('list_namespaces'),
  listTags: (namespace?: string) => invoke<Record<string, number>>('list_tags', { namespace: namespace ?? null }),
  getSettings: () => invoke<Record<string, string>>('get_settings'),
  setSettings: (settings: Record<string, string>) => invoke<void>('set_settings', { settings }),
  exportData: (format: 'json' | 'markdown') => invoke<string>('export_data', { format }),
  importData: (data: string) => invoke<number>('import_data', { data }),
  openDataDir: () => invoke<string>('init_data_dir'),
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
  rooms: (namespace?: string) =>
    invoke<{ id: string; title: string | null; namespace: string; memory_count: number; participant_count: number; created_at: string; updated_at: string }[]>('uteke_rooms', {
      namespace: namespace ?? null,
    }),
  roomRecall: (roomId: string, limit?: number) =>
    invoke<MemoryEntry[]>('uteke_room_recall', { roomId, limit: limit ?? null }),
  list: (opts?: { namespace?: string; tag?: string; limit?: number; offset?: number }) =>
    invoke<MemoryEntry[]>('uteke_list', {
      namespace: opts?.namespace ?? null,
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
