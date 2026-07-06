// View type for navigation
export type View = 'dashboard' | 'memories' | 'namespaces' | 'graph' | 'rooms' | 'settings';

// Ecosystem product definitions
export interface ProductCard {
  id: string;
  name: string;
  icon: string;
  description: string;
  color: string;
  defaultUrl: string;
  healthPath: string;
}

// Product health check result
export interface ProductHealth {
  id: string;
  available: boolean;
  latency_ms: number;
  version: string | null;
  error: string | null;
}

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

// Stats response
export interface StatsResponse {
  total_memories: number;
  total_namespaces: number;
  total_tags: number;
  total_edges: number;
  db_size_bytes: number;
}
