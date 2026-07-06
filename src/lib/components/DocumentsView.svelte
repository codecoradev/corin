<script lang="ts">
  import { docs } from '../ts/ipc';
  import type { DocEntry, DocSearchResult } from '../ts/types';
  import { EditorView, keymap, lineNumbers, highlightActiveLineGutter, highlightSpecialChars, drawSelection, dropCursor, rectangularSelection, crosshairCursor, highlightActiveLine } from '@codemirror/view';
  import { EditorState } from '@codemirror/state';
  import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
  import { bracketMatching, foldGutter, indentOnInput, foldKeymap, defaultHighlightStyle, HighlightStyle, syntaxHighlighting } from '@codemirror/language';
  import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
  import { languages } from '@codemirror/language-data';
  import { searchKeymap, highlightSelectionMatches } from '@codemirror/search';
  import { autocompletion, completionKeymap, closeBrackets, closeBracketsKeymap } from '@codemirror/autocomplete';
  import { tags } from '@lezer/highlight';

  // Catppuccin Mocha theme
  const catppuccinDarkTheme = EditorView.theme({
    '&': { color: '#cdd6f4', backgroundColor: '#1e1e2e', height: '100%' },
    '.cm-content': { caretColor: '#f5e0dc' },
    '.cm-cursor, .cm-dropCursor': { borderLeftColor: '#f5e0dc' },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': { backgroundColor: '#45475a !important' },
    '.cm-panels': { backgroundColor: '#181825', color: '#cdd6f4' },
    '.cm-panels.cm-panels-top': { borderBottom: '2px solid #313244' },
    '.cm-searchMatch': { backgroundColor: 'rgba(249,226,175,0.2)', outline: '1px solid rgba(249,226,175,0.4)' },
    '&.cm-focused .cm-matchingBracket, &.cm-focused .cm-nonmatchingBracket': { backgroundColor: 'rgba(137,180,250,0.3)', outline: '1px solid #89b4fa' },
    '.cm-activeLine': { backgroundColor: 'rgba(69,71,90,0.3)' },
    '.cm-selectionMatch': { backgroundColor: 'rgba(137,180,250,0.15)' },
  }, { dark: true });

  const catppuccinDarkHighlighting = HighlightStyle.define([
    { tag: tags.heading1, color: '#cba6f7', fontWeight: 'bold', fontSize: '1.2em' },
    { tag: tags.heading2, color: '#cba6f7', fontWeight: 'bold', fontSize: '1.1em' },
    { tag: tags.heading3, color: '#cba6f7', fontWeight: 'bold' },
    { tag: tags.heading4, color: '#cba6f7' },
    { tag: tags.heading5, color: '#cba6f7' },
    { tag: tags.heading6, color: '#cba6f7' },
    { tag: tags.emphasis, color: '#f9e2af', fontStyle: 'italic' },
    { tag: tags.strong, color: '#fab387', fontWeight: 'bold' },
    { tag: tags.strikethrough, textDecoration: 'line-through' },
    { tag: tags.link, color: '#89b4fa' },
    { tag: tags.url, color: '#89b4fa', textDecoration: 'underline' },
    { tag: tags.monospace, color: '#a6e3a1', fontFamily: 'var(--font-mono)' },
    { tag: tags.quote, color: '#a6adc8', fontStyle: 'italic' },
    { tag: tags.meta, color: '#9399b2' },
    { tag: tags.processingInstruction, color: '#f38ba8' },
    { tag: tags.comment, color: '#6c7086', fontStyle: 'italic' },
    { tag: tags.keyword, color: '#cba6f7' },
    { tag: tags.string, color: '#a6e3a1' },
    { tag: tags.number, color: '#fab387' },
    { tag: tags.bool, color: '#fab387' },
    { tag: tags.null, color: '#9399b2' },
    { tag: tags.propertyName, color: '#89b4fa' },
    { tag: tags.variableName, color: '#cdd6f4' },
    { tag: tags.operator, color: '#89dceb' },
    { tag: tags.punctuation, color: '#9399b2' },
    { tag: tags.bracket, color: '#9399b2' },
    { tag: tags.atom, color: '#fab387' },
    { tag: tags.content, color: '#cdd6f4' },
    { tag: tags.contentSeparator, color: '#585b70' },
    { tag: tags.list, color: '#89b4fa' },
  ]);

  interface Props {
    namespace: string | null;
  }

  let { namespace }: Props = $props();

  // ─── State ──────────────────────────────────────────────────────────
  let rootDocs = $state<DocEntry[]>([]);
  let expandedIds = $state<Set<string>>(new Set());
  let childrenCache = $state<Map<string, DocEntry[]>>(new Map());
  let selectedDoc = $state<DocEntry | null>(null);
  let editorContent = $state('');
  let editorTitle = $state('');
  let editorSlug = $state('');
  let editorTags = $state('');
  let searchResults = $state<DocSearchResult[]>([]);
  let searchQuery = $state('');
  let showSearchResults = $state(false);
  let loading = $state(true);
  let searching = $state(false);
  let saving = $state(false);
  let showNewDoc = $state(false);
  let showDeleteConfirm = $state(false);
  let deleteTarget = $state<DocEntry | null>(null);
  let error = $state('');
  let editorView = $state<EditorView | null>(null);

  // ─── Load root documents ──────────────────────────────────────────
  async function loadRootDocs() {
    loading = true;
    error = '';
    try {
      rootDocs = await docs.list({ roots_only: true, namespace: namespace ?? undefined });
    } catch (e: any) {
      error = e.toString();
    } finally {
      loading = false;
    }
  }

  // ─── Load children for a node ────────────────────────────────────
  async function loadChildren(docId: string) {
    if (childrenCache.has(docId)) return;
    try {
      const children = await docs.list({ parent: docId, namespace: namespace ?? undefined });
      childrenCache.set(docId, children);
      childrenCache = childrenCache; // trigger reactivity
    } catch { /* ignore */ }
  }

  // ─── Toggle tree node ─────────────────────────────────────────────
  async function toggleNode(doc: DocEntry) {
    if (expandedIds.has(doc.id)) {
      expandedIds.delete(doc.id);
      expandedIds = expandedIds;
    } else {
      expandedIds.add(doc.id);
      expandedIds = expandedIds;
      await loadChildren(doc.id);
    }
  }

  // ─── Select doc ──────────────────────────────────────────────────
  async function selectDoc(doc: DocEntry) {
    selectedDoc = doc;
    loading = true;
    try {
      const full = await docs.get({ id: doc.id });
      selectedDoc = full;
      editorTitle = full.title;
      editorSlug = full.slug;
      editorContent = full.content ?? '';
      editorTags = (full.tags ?? []).join(', ');
      showSearchResults = false;
      showNewDoc = false;
    } catch (e: any) {
      error = e.toString();
    } finally {
      loading = false;
    }
  }

  // ─── Navigate breadcrumb ──────────────────────────────────────────
  async function navigateToPath(docId: string) {
    const doc = rootDocs.find(d => d.id === docId) ||
      [...childrenCache.values()].flat().find(d => d.id === docId);
    if (doc) await selectDoc(doc);
  }

  // ─── Search ────────────────────────────────────────────────────────
  let searchTimeout: ReturnType<typeof setTimeout> | null = null;

  function onSearchInput(e: Event) {
    searchQuery = (e.target as HTMLInputElement).value;
    if (searchTimeout) clearTimeout(searchTimeout);
    if (!searchQuery.trim()) {
      showSearchResults = false;
      searchResults = [];
      return;
    }
    searchTimeout = setTimeout(() => handleSearch(), 300);
  }

  async function handleSearch() {
    if (!searchQuery.trim()) return;
    searching = true;
    showSearchResults = true;
    try {
      searchResults = await docs.search(searchQuery, { namespace: namespace ?? undefined, limit: 20 });
    } catch (e: any) {
      error = e.toString();
    } finally {
      searching = false;
    }
  }

  // ─── New doc ────────────────────────────────────────────────────────
  function newDoc() {
    showNewDoc = true;
    selectedDoc = null;
    editorTitle = '';
    editorSlug = '';
    editorContent = '# New Document\n\n';
    editorTags = '';
    showSearchResults = false;
  }

  // ─── Save ──────────────────────────────────────────────────────────
  async function saveDoc() {
    if (!editorSlug.trim()) {
      error = 'Slug is required';
      return;
    }
    saving = true;
    error = '';
    try {
      const tags = editorTags.split(',').map(t => t.trim()).filter(Boolean);
      const parent = selectedDoc?.parent_id ?? undefined;
      const created = await docs.create(editorSlug, editorTitle || editorSlug, editorContent, {
        namespace: namespace ?? undefined,
        tags,
        parent,
      });
      selectedDoc = created;
      showNewDoc = false;
      await loadRootDocs();
    } catch (e: any) {
      error = e.toString();
    } finally {
      saving = false;
    }
  }

  // ─── Delete ───────────────────────────────────────────────────────
  function confirmDelete(doc: DocEntry) {
    deleteTarget = doc;
    showDeleteConfirm = true;
  }

  async function executeDelete() {
    if (!deleteTarget) return;
    try {
      await docs.delete({ id: deleteTarget.id });
      if (selectedDoc?.id === deleteTarget.id) {
        selectedDoc = null;
        showNewDoc = false;
      }
      await loadRootDocs();
    } catch (e: any) {
      error = e.toString();
    }
    showDeleteConfirm = false;
    deleteTarget = null;
  }

  // ─── Export ───────────────────────────────────────────────────────
  function exportDoc() {
    if (selectedDoc) docs.exportFile(selectedDoc);
  }

  // ─── CodeMirror ──────────────────────────────────────────────────
  function createEditor(container: HTMLElement, content: string) {
    const state = EditorState.create({
      doc: content,
      extensions: [
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightSpecialChars(),
        history(),
        foldGutter(),
        drawSelection(),
        dropCursor(),
        EditorState.allowMultipleSelections.of(true),
        indentOnInput(),
        bracketMatching(),
        closeBrackets(),
        autocompletion(),
        rectangularSelection(),
        crosshairCursor(),
        highlightActiveLine(),
        highlightSelectionMatches(),
        syntaxHighlighting(catppuccinDarkHighlighting),
        catppuccinDarkTheme,
        markdown({ base: markdownLanguage, codeLanguages: languages }),
        keymap.of([
          ...closeBracketsKeymap,
          ...defaultKeymap,
          ...searchKeymap,
          ...historyKeymap,
          ...foldKeymap,
          ...completionKeymap,
          indentWithTab,
        ]),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            editorContent = update.state.doc.toString();
          }
        }),
      ],
    });

    const view = new EditorView({ state, parent: container });
    editorView = view;
  }

  function updateEditor(content: string) {
    if (editorView) {
      editorView.dispatch({
        changes: { from: 0, to: editorView.state.doc.length, insert: content },
      });
    }
  }

  function editorMount(node: HTMLElement) {
    if (node) {
      createEditor(node, editorContent);
    } else if (editorView) {
      editorView.destroy();
      editorView = null;
    }
  }

  // ─── Formatted date ───────────────────────────────────────────────
  function formatDate(iso: string | null | undefined): string {
    if (!iso) return '';
    try {
      return new Date(iso).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
    } catch {
      return iso ?? '';
    }
  }

  // Watch editorContent changes from outside (e.g. load new doc)
  $effect(() => {
    if (editorView && (selectedDoc || showNewDoc)) {
      const current = editorView.state.doc.toString();
      if (current !== editorContent) {
        updateEditor(editorContent);
      }
    }
  });

  // ─── Build breadcrumb path ────────────────────────────────────────
  function getBreadcrumb(): string[] {
    if (!selectedDoc) return [];
    const parts: string[] = [];
    if (selectedDoc.path && selectedDoc.path.length > 0) {
      parts.push(...selectedDoc.path);
    }
    return parts;
  }

  // ─── Lifecycle ───────────────────────────────────────────────────
  $effect(() => {
    loadRootDocs();
  });

  // Cleanup on destroy
  $effect(() => {
    return () => {
      if (editorView) editorView.destroy();
    };
  });
</script>

<div class="docs-view">
  <div class="docs-header">
    <h2>Documents</h2>
    <span class="count">{rootDocs.length} root docs</span>
    <div class="header-actions">
      <button class="action-btn new-btn" onclick={newDoc}>+ New</button>
    </div>
  </div>

  {#if error}
    <div class="error-bar">
      {error}
      <button class="dismiss-btn" onclick={() => (error = '')}>✕</button>
    </div>
  {/if}

  <div class="layout">
    <!-- ─── Left Panel: Tree + Search ─────────────────────────── -->
    <div class="doc-tree-panel">
      <div class="search-bar">
        <input
          type="text"
          placeholder="Search documents..."
          value={searchQuery}
          oninput={onSearchInput}
        />
      </div>

      {#if searching}
        <div class="msg">Searching...</div>
      {:else if showSearchResults && searchResults.length > 0}
        <div class="search-results">
          {#each searchResults as result (result.document.id)}
            <button
              class="doc-card search-hit"
              class:active={selectedDoc?.id === result.document.id}
              onclick={() => selectDoc(result.document)}
            >
              <div class="doc-title">{result.document.title}</div>
              <div class="doc-slug">/{result.document.slug}</div>
              {#if result.chunk_snippet}
                <div class="doc-snippet">{result.chunk_snippet.slice(0, 120)}</div>
              {/if}
              <div class="doc-score">{(result.score * 100).toFixed(0)}%</div>
            </button>
          {/each}
        </div>
      {:else if showSearchResults}
        <div class="msg">No results for "{searchQuery}"</div>
      {:else if loading}
        <div class="msg">Loading...</div>
      {:else if rootDocs.length === 0}
        <div class="msg">
          <p>No documents yet.</p>
          <p class="sub">Create your first document with the + New button.</p>
        </div>
      {:else}
        <div class="doc-tree">
          {#each rootDocs as doc (doc.id)}
            {@render treeNode(doc)}
          {/each}
        </div>
      {/if}
    </div>

    <!-- ─── Right Panel: Editor ─────────────────────────────────── -->
    <div class="editor-panel">
      {#if selectedDoc || showNewDoc}
        <!-- Breadcrumb -->
        {#if selectedDoc && !showNewDoc}
          <div class="breadcrumb">
            <button class="crumb-link" onclick={() => { selectedDoc = null; showNewDoc = false; }}>Documents</button>
            {#each getBreadcrumb() as part, i}
              <span class="crumb-sep">/</span>
              <button class="crumb-link">{part}</button>
            {/each}
            <span class="crumb-sep">/</span>
            <span class="crumb-current">{selectedDoc.title}</span>
          </div>
        {/if}

        <div class="editor-toolbar">
          <div class="editor-meta">
            <input
              type="text"
              class="title-input"
              placeholder="Document title..."
              bind:value={editorTitle}
            />
            <input
              type="text"
              class="slug-input"
              placeholder="slug-name"
              bind:value={editorSlug}
            />
            <input
              type="text"
              class="tags-input"
              placeholder="tag1, tag2"
              bind:value={editorTags}
            />
          </div>
          <div class="editor-actions">
            <button class="action-btn" onclick={saveDoc} disabled={saving}>
              {saving ? 'Saving...' : '💾 Save'}
            </button>
            <button class="action-btn" onclick={exportDoc}>📥 Export</button>
            {#if selectedDoc}
              <button class="action-btn danger" onclick={() => selectedDoc && confirmDelete(selectedDoc)}>🗑</button>
            {/if}
          </div>
        </div>

        {#if selectedDoc && !showNewDoc}
          <div class="doc-meta-bar">
            <span class="meta-item">v{selectedDoc.version ?? 1}</span>
            {#if selectedDoc.updated_at}
              <span class="meta-item">{formatDate(selectedDoc.updated_at)}</span>
            {/if}
            {#if selectedDoc.namespace}
              <span class="meta-item ns-badge">{selectedDoc.namespace}</span>
            {/if}
          </div>
        {/if}

        <div class="editor-container" use:editorMount></div>
      {:else}
        <div class="empty-state">
          <div class="empty-icon">📄</div>
          <p>Select a document to view</p>
          <p class="sub">or create a new one with + New</p>
        </div>
      {/if}
    </div>
  </div>
</div>

<!-- Delete Confirmation Dialog -->
{#if showDeleteConfirm}
  <div class="overlay" onclick={() => { showDeleteConfirm = false; deleteTarget = null; }}>
    <div class="confirm-dialog" onclick={(e) => e.stopPropagation()}>
      <h3>Delete Document</h3>
      <p>Are you sure you want to delete <strong>{deleteTarget?.title}</strong>?</p>
      <p class="sub">This action cannot be undone.</p>
      <div class="confirm-actions">
        <button class="action-btn" onclick={() => { showDeleteConfirm = false; deleteTarget = null; }}>Cancel</button>
        <button class="action-btn danger" onclick={executeDelete}>Delete</button>
      </div>
    </div>
  </div>
{/if}

{#snippet treeNode(doc: DocEntry)}
  <div class="tree-node">
    <button
      class="doc-card"
      class:active={selectedDoc?.id === doc.id}
      onclick={() => selectDoc(doc)}
    >
      <span class="tree-toggle" class:has-children={doc.has_children} class:expanded={expandedIds.has(doc.id)}
        onclick={(e: MouseEvent) => { e.stopPropagation(); if (doc.has_children) toggleNode(doc); }}
        role="button"
      >
        {doc.has_children ? (expandedIds.has(doc.id) ? '▼' : '▶') : '•'}
      </span>
      <span class="doc-info">
        <div class="doc-title">{doc.title}</div>
        <div class="doc-slug">/{doc.slug}</div>
      </span>
    </button>
    {#if expandedIds.has(doc.id) && childrenCache.has(doc.id)}
      <div class="tree-children">
        {#each childrenCache.get(doc.id) ?? [] as child (child.id)}
          {@render treeNode(child)}
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

<style>
  .docs-view { height: 100%; display: flex; flex-direction: column; }
  .docs-header { padding: 16px 24px 8px; display: flex; align-items: baseline; gap: 12px; border-bottom: 1px solid var(--border); }
  h2 { font-size: 1.1rem; }
  .count { font-size: 0.8rem; color: var(--text-muted); }
  .header-actions { margin-left: auto; }

  .error-bar {
    padding: 8px 24px;
    background: rgba(243, 139, 168, 0.15);
    color: #f38ba8;
    font-size: 0.8rem;
    display: flex;
    align-items: center;
    gap: 8px;
    border-bottom: 1px solid rgba(243, 139, 168, 0.3);
  }
  .dismiss-btn { background: none; border: none; color: #f38ba8; cursor: pointer; padding: 0 4px; }

  .layout { flex: 1; display: flex; overflow: hidden; }

  /* ─── Left Panel (Tree) ─── */
  .doc-tree-panel {
    width: 300px;
    min-width: 200px;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border);
    overflow: hidden;
  }

  .search-bar { padding: 8px 12px; border-bottom: 1px solid var(--border); }
  .search-bar input {
    width: 100%;
    padding: 6px 10px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 0.85rem;
  }
  .search-bar input:focus { outline: none; border-color: var(--accent); }

  .doc-tree { flex: 1; overflow-y: auto; padding: 4px 0; }

  .tree-node { user-select: none; }
  .tree-children { padding-left: 16px; }

  .tree-toggle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    font-size: 0.65rem;
    color: var(--text-muted);
    flex-shrink: 0;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
  }
  .tree-toggle.has-children { cursor: pointer; }
  .tree-toggle:hover { color: var(--text-primary); }

  .doc-card {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    padding: 6px 12px;
    width: 100%;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 0.85rem;
    cursor: pointer;
    transition: background 0.1s;
    text-align: left;
  }
  .doc-card:hover { background: var(--bg-hover); }
  .doc-card.active { background: var(--bg-hover); color: var(--accent); border-left: 2px solid var(--accent); }
  .doc-card.search-hit { padding: 10px 12px; border-left: 3px solid transparent; }
  .doc-card.search-hit.active { border-left-color: var(--accent); }
  .doc-info { min-width: 0; }
  .doc-title { font-size: 0.85rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .doc-slug { font-size: 0.7rem; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .doc-snippet { font-size: 0.75rem; color: var(--text-muted); margin-top: 2px; line-height: 1.3; }
  .doc-score { font-size: 0.65rem; color: var(--accent); font-weight: 600; margin-top: 2px; }

  .doc-meta { display: flex; gap: 8px; font-size: 0.7rem; color: var(--text-muted); margin-top: 2px; }
  .doc-meta .ns { color: var(--accent); }

  .search-results { overflow-y: auto; }

  .msg { padding: 20px 12px; text-align: center; color: var(--text-muted); font-size: 0.85rem; }
  .msg .sub { font-size: 0.75rem; margin-top: 4px; }

  /* ─── Right Panel (Editor) ─── */
  .editor-panel { flex: 1; display: flex; flex-direction: column; overflow: hidden; }

  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 6px 16px;
    font-size: 0.75rem;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
    white-space: nowrap;
  }
  .crumb-link { background: none; border: none; color: var(--text-muted); cursor: pointer; padding: 2px 4px; font-size: 0.75rem; }
  .crumb-link:hover { color: var(--accent); }
  .crumb-sep { color: var(--text-muted); opacity: 0.5; }
  .crumb-current { color: var(--text-primary); font-weight: 600; padding: 2px 4px; }

  .editor-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
    flex-wrap: wrap;
  }
  .editor-meta { display: flex; gap: 8px; flex: 1; min-width: 0; }
  .title-input {
    flex: 1;
    min-width: 120px;
    padding: 4px 8px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 0.85rem;
    font-weight: 600;
  }
  .slug-input {
    width: 160px;
    padding: 4px 8px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 0.8rem;
    font-family: var(--font-mono);
  }
  .tags-input {
    width: 140px;
    padding: 4px 8px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 0.8rem;
  }
  .title-input:focus, .slug-input:focus, .tags-input:focus { outline: none; border-color: var(--accent); }

  .editor-actions { display: flex; gap: 6px; }

  .doc-meta-bar {
    display: flex;
    gap: 12px;
    padding: 4px 16px;
    font-size: 0.7rem;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
  }
  .ns-badge {
    padding: 1px 6px;
    background: var(--bg-tertiary);
    border-radius: 3px;
    color: var(--accent);
    font-size: 0.65rem;
    font-family: var(--font-mono);
  }

  .editor-container { flex: 1; overflow: hidden; }
  .editor-container :global(.cm-editor) { height: 100%; }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    gap: 8px;
  }
  .empty-icon { font-size: 3rem; opacity: 0.3; }
  .empty-state p { margin: 0; }
  .empty-state .sub { font-size: 0.8rem; }

  /* ─── Shared Buttons ─── */
  .action-btn {
    padding: 6px 12px;
    background: var(--bg-hover);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 0.8rem;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }
  .action-btn:hover { background: var(--bg-tertiary); border-color: var(--text-muted); }
  .action-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .action-btn.danger { color: #f38ba8; border-color: rgba(243,139,168,0.3); }
  .action-btn.danger:hover { background: rgba(243,139,168,0.15); }
  .new-btn { background: var(--accent); color: var(--bg-primary); border-color: var(--accent); font-weight: 600; }
  .new-btn:hover { opacity: 0.85; }

  /* ─── Dialog ─── */
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .confirm-dialog {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 24px;
    max-width: 400px;
    width: 90%;
  }
  .confirm-dialog h3 { margin: 0 0 12px; }
  .confirm-dialog p { margin: 0; font-size: 0.9rem; }
  .confirm-dialog .sub { font-size: 0.8rem; color: var(--text-muted); margin-top: 4px; }
  .confirm-actions { display: flex; gap: 8px; margin-top: 16px; justify-content: flex-end; }
</style>
