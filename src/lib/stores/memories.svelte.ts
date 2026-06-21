// Memory data store — loaded memories, search results, filters
import type { MemoryEntry, SearchResult } from '../ts/types';
import { memory as memoryApi } from '../ts/ipc';

export function getMemoryStore() {
  let memories: MemoryEntry[] = $state([]);
  let searchResults: SearchResult[] = $state([]);
  let activeMemory: MemoryEntry | null = $state(null);
  let loading = $state(false);
  let error: string | null = $state(null);
  let currentNamespace: string | null = $state(null);
  let currentTag: string | null = $state(null);

  async function loadMemories(opts?: { namespace?: string; tag?: string; limit?: number }) {
    loading = true;
    error = null;
    try {
      memories = await memoryApi.list(opts);
    } catch (e: any) {
      error = e.toString();
    } finally {
      loading = false;
    }
  }

  async function searchMemories(query: string, opts?: { namespace?: string; limit?: number }) {
    loading = true;
    error = null;
    try {
      searchResults = await memoryApi.search(query, opts);
    } catch (e: any) {
      error = e.toString();
    } finally {
      loading = false;
    }
  }

  async function selectMemory(id: string) {
    try {
      activeMemory = await memoryApi.get(id);
    } catch {
      activeMemory = null;
    }
  }

  async function createMemory(content: string, opts?: { tags?: string[]; namespace?: string }) {
    const id = await memoryApi.remember(content, opts);
    await loadMemories({ namespace: currentNamespace ?? undefined, tag: currentTag ?? undefined });
    return id;
  }

  async function deleteMemory(id: string) {
    await memoryApi.forget(id);
    if (activeMemory?.id === id) activeMemory = null;
    await loadMemories({ namespace: currentNamespace ?? undefined, tag: currentTag ?? undefined });
  }

  function setNamespace(ns: string | null) { currentNamespace = ns; }
  function setTag(tag: string | null) { currentTag = tag; }
  function clearSearch() { searchResults = []; error = null; }

  return {
    get memories() { return memories; },
    get searchResults() { return searchResults; },
    get activeMemory() { return activeMemory; },
    get loading() { return loading; },
    get error() { return error; },
    get currentNamespace() { return currentNamespace; },
    get currentTag() { return currentTag; },
    loadMemories,
    searchMemories,
    selectMemory,
    createMemory,
    deleteMemory,
    setNamespace,
    setTag,
    clearSearch,
  };
}
