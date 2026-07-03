// Cursor/offset pagination helper for uteke list endpoints.
//
// Per-namespace offset pagination is verified working on uteke-serve.
// (Cross-namespace list is a server-side gap — see uteke #526 — so we
// still page within a single namespace here.)
//
// Usage:
//   const pager = createPager({ namespace: 'corin', pageSize: 20 });
//   await pager.loadInitial();
//   await pager.loadMore();

import { uteke, memory as memoryApi } from '../ts/ipc';
import type { MemoryEntry } from '../ts/types';

export interface PagerOpts {
  namespace?: string | null;
  namespaces?: string[] | null;
  pageSize?: number;
  /** If false, use the local fallback (memoryApi) instead of uteke HTTP. */
  useUteke?: boolean;
}

export function createPager(opts: PagerOpts = {}) {
  const pageSize = opts.pageSize ?? 20;
  const namespace = opts.namespace ?? null;
  // `null` = all namespaces (backend fans out every namespace).
  // `[]` = none, array = explicit selection.
  const namespaces = opts.namespaces ?? null;
  const useUteke = opts.useUteke ?? true;

  let items = $state<MemoryEntry[]>([]);
  let offset = $state(0);
  let hasMore = $state(true);
  let loading = $state(false);
  let initialLoaded = $state(false);

  async function fetchPage(off: number, limit: number): Promise<MemoryEntry[]> {
    if (useUteke) {
      // When an explicit namespace selection exists, pass `namespaces`.
      // Otherwise (null = all) pass neither so the backend fans out every
      // namespace (legacy /list no-namespace returns only "default").
      const useSelection = namespaces !== null;
      return uteke
        .list({
          namespace: useSelection ? undefined : (namespace ?? undefined),
          namespaces: useSelection ? (namespaces ?? undefined) : undefined,
          limit,
          offset: off,
        })
        .catch(() => []);
    }
    return memoryApi.list({ namespace: namespace ?? undefined, limit }).catch(() => []);
  }

  async function loadInitial() {
    if (initialLoaded) return;
    loading = true;
    try {
      const page = await fetchPage(0, pageSize);
      items = page;
      offset = page.length;
      hasMore = page.length === pageSize;
      initialLoaded = true;
    } finally {
      loading = false;
    }
  }

  async function loadMore() {
    if (!hasMore || loading) return;
    loading = true;
    try {
      const page = await fetchPage(offset, pageSize);
      items = [...items, ...page];
      offset += page.length;
      hasMore = page.length === pageSize;
    } finally {
      loading = false;
    }
  }

  function reset() {
    items = [];
    offset = 0;
    hasMore = true;
    initialLoaded = false;
  }

  return {
    get items() { return items; },
    get hasMore() { return hasMore; },
    get loading() { return loading; },
    get initialLoaded() { return initialLoaded; },
    get pageSize() { return pageSize; },
    loadInitial,
    loadMore,
    reset,
  };
}
