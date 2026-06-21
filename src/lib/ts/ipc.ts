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
  openDataDir: () => invoke<string>('open_data_dir'),
};

// Tauri event listener
export { listen } from '@tauri-apps/api/event';
export type { UnlistenFn } from '@tauri-apps/api/event';
