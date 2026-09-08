/**
 * JSONL (newline-delimited JSON) helpers for the server-native
 * export/import format (GET /export ↔ POST /import).
 */

export interface JsonlEntry {
  content: string;
  tags: string[];
  namespace: string | null;
  raw: Record<string, unknown>;
}

export interface JsonlPreview {
  count: number;
  malformed: number;
  namespaces: string[];
  tags: string[];
  first5: JsonlEntry[];
}

/** Parse exported JSONL text into a preview. Malformed lines are counted, not fatal. */
export function parseJsonl(data: string): JsonlPreview {
  const namespaces = new Set<string>();
  const tags = new Set<string>();
  const first5: JsonlEntry[] = [];
  let count = 0;
  let malformed = 0;

  for (const line of data.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    try {
      const raw = JSON.parse(trimmed) as Record<string, unknown>;
      count++;
      const content = typeof raw.content === 'string' ? raw.content : '';
      const entryTags = Array.isArray(raw.tags) ? raw.tags.filter((t): t is string => typeof t === 'string') : [];
      const namespace = typeof raw.namespace === 'string' ? raw.namespace : null;
      if (namespace) namespaces.add(namespace);
      for (const t of entryTags) tags.add(t);
      if (first5.length < 5) first5.push({ content, tags: entryTags, namespace, raw });
    } catch {
      malformed++;
    }
  }

  return { count, malformed, namespaces: [...namespaces].sort(), tags: [...tags].sort(), first5 };
}
