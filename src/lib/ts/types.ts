// View type for navigation
export type View = 'dashboard' | 'memories' | 'namespaces' | 'graph' | 'rooms' | 'kanban' | 'documents' | 'settings';

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

// Document summary (list view — no content)
export interface DocSummary {
  id: string;
  slug: string;
  title: string;
  namespace: string;
  version: number;
  updated_at: string;
  parent_id: string | null;
  depth: number;
  has_children: boolean;
  sort_order: number;
}

// Full document (with content)
export interface DocEntry {
  id: string;
  slug: string;
  title: string;
  content: string;
  namespace: string;
  tags: string[];
  metadata: Record<string, unknown> | null;
  version: number;
  content_type: string;
  created_at: string;
  updated_at: string;
  parent_id: string | null;
  path: string;
  depth: number;
  sort_order: number;
  has_children: boolean;
}

// Document search result
export interface DocSearchResult {
  document: DocSummary;
  chunk_heading: string;
  chunk_snippet: string;
  score: number;
  mode: string;
}

// ─── Kanban types (Hermes dashboard plugin API) ───────────────────────────

export interface KanbanTaskAge {
  created_age_seconds: number | null;
  started_age_seconds: number | null;
  time_to_complete_seconds: number | null;
}

export interface KanbanLinkCounts {
  parents: number;
  children: number;
}

export interface KanbanTaskProgress {
  done: number;
  total: number;
}

export interface KanbanTask {
  id: string;
  title: string;
  body: string | null;
  assignee: string | null;
  status: string;
  priority: number;
  tenant: string | null;
  workspace_kind: string | null;
  created_at: number;
  started_at: number | null;
  completed_at: number | null;
  result: string | null;
  latest_summary: string | null;
  age: KanbanTaskAge | null;
  link_counts: KanbanLinkCounts | null;
  comment_count: number | null;
  progress: KanbanTaskProgress | null;
  diagnostics: unknown[] | null;
  warnings: { count: number } | null;
}

export interface KanbanColumn {
  name: string;
  tasks: KanbanTask[];
}

export interface KanbanBoard {
  columns: KanbanColumn[];
  tenants: string[];
  assignees: string[];
  latest_event_id: number;
  now: number;
}

export interface KanbanComment {
  id: number;
  task_id: string;
  author: string;
  body: string;
  created_at: number;
}

export interface KanbanEvent {
  id: number;
  task_id: string;
  kind: string;
  payload: unknown;
  created_at: number;
  run_id: number | null;
}

export interface KanbanLinks {
  parents: string[];
  children: string[];
}

export interface KanbanRun {
  id: number;
  profile: string;
  status: string;
  outcome: string | null;
  summary: string | null;
  error: string | null;
  worker_pid: number | null;
  started_at: number | null;
  ended_at: number | null;
}

export interface KanbanTaskDetail {
  task: KanbanTask;
  comments: KanbanComment[];
  events: KanbanEvent[];
  links: KanbanLinks;
  runs: KanbanRun[];
}

export interface KanbanStats {
  [key: string]: unknown;
}

export interface CreateTaskResponse {
  task: KanbanTask | null;
  warning: string | null;
}
