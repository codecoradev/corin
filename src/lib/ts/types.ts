// View type for navigation
export type View = 'dashboard' | 'memories' | 'graph' | 'rooms' | 'settings';

// Memory entry from uteke-core
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

// Stats response
export interface StatsResponse {
  total_memories: number;
  total_namespaces: number;
  total_tags: number;
  total_edges: number;
  db_size_bytes: number;
}
