import { writable } from 'svelte/store';

/**
 * A document slug requested from elsewhere in the UI (e.g. a unified-search
 * document hit in MemoryList). `DocumentsView` consumes + clears this on mount
 * via a `$effect`, opening the document by slug.
 */
export const pendingDocSlug = writable<string | null>(null);
