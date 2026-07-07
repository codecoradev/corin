// View type for navigation
export type View = 'dashboard' | 'memories' | 'namespaces' | 'graph' | 'rooms' | 'documents' | 'settings';

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

// Document entry from uteke-serve /doc/* API
export interface DocEntry {
  id: string;
  slug: string;
  title: string;
  content?: string;
  namespace?: string | null;
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

// Stats response
export interface StatsResponse {
  total_memories: number;
  total_namespaces: number;
  total_tags: number;
  total_edges: number;
  db_size_bytes: number;
}
