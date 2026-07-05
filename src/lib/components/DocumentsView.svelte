<script lang="ts">
  import { docs } from '../ts/ipc';
  import type { DocSummary, DocEntry, DocSearchResult } from '../ts/types';
  import { onMount } from 'svelte';
  import DOMPurify from 'dompurify';
  import { marked } from 'marked';

  interface Props {
    namespace: string | null;
  }

  let { namespace }: Props = $props();

  // ── State ────────────────────────────────────────────────────────────
  let documents = $state<DocSummary[]>([]);
  let loading = $state(true);
  let selectedDocId = $state<string | null>(null);
  let currentDoc = $state<DocEntry | null>(null);
  let docLoading = $state(false);

  // Tree state
  let expandedNodes = $state<Set<string>>(new Set());
  let childrenCache = $state<Record<string, DocSummary[]>>({});
  let childrenLoading = $state<Record<string, boolean>>({});

  // Search state
  let searchQuery = $state('');
  let searchResults = $state<DocSearchResult[]>([]);
  let searchActive = $state(false);
  let searchLoading = $state(false);

  // Editor state
  let editMode = $state(false);
  let editContent = $state('');
  let editTitle = $state('');
  let editTags = $state('');
  let editSaving = $state(false);
  let editSlug = $state('');
  let previewMode = $state<'edit' | 'preview' | 'split'>('edit');

  // Create state
  let showCreateForm = $state(false);
  let newSlug = $state('');
  let newTitle = $state('');
  let newContent = $state('');
  let newTags = $state('');
  let creating = $state(false);

  // Delete state
  let showDeleteConfirm = $state(false);

  // Active tab
  let activeTab = $state<'browse' | 'search'>('browse');

  // Error state
  let lastError = $state<string | null>(null);

  // CodeMirror refs
  let editorContainer: HTMLElement | undefined = $state();
  let editorView: import('@codemirror/view').EditorView | undefined = $state();
  let cmLoaded = $state(false);

  function reportError(op: string, e: unknown) {
    const msg = typeof e === 'string' ? e : (e instanceof Error ? e.message : JSON.stringify(e));
    lastError = `${op}: ${msg}`;
    console.error(lastError, e);
    setTimeout(() => { lastError = null; }, 5000);
  }

  // ── Breadcrumb ─────────────────────────────────────────────────────
  let breadcrumb = $state<Array<{ id: string; title: string }>>([]);

  function buildBreadcrumb(doc: DocEntry | null): Array<{ id: string; title: string }> {
    if (!doc || !doc.parent_id) return doc ? [{ id: doc.id, title: doc.title }] : [];
    const parts: Array<{ id: string; title: string }> = [{ id: doc.id, title: doc.title }];
    let parentId = doc.parent_id;
    let depth = 0;
    while (parentId && depth < 10) {
      const parent = documents.find(d => d.id === parentId);
      if (parent) {
        parts.unshift({ id: parent.id, title: parent.title });
        parentId = parent.parent_id ?? null;
      } else {
        // Check children cache
        for (const children of Object.values(childrenCache)) {
          const p = children.find(d => d.id === parentId);
          if (p) {
            parts.unshift({ id: p.id, title: p.title });
            parentId = p.parent_id ?? null;
            break;
          }
        }
        if (!parent) break;
      }
      depth++;
    }
    return parts;
  }

  // ── Load documents ───────────────────────────────────────────────
  async function loadDocuments() {
    loading = true;
    try {
      documents = await docs.listRoots({ namespace: namespace ?? undefined });
    } catch (e) {
      documents = [];
      reportError('Load documents', e);
    }
    loading = false;
  }

  $effect(() => {
    namespace;
    expandedNodes = new Set();
    childrenCache = {};
    selectedDocId = null;
    currentDoc = null;
    loadDocuments();
  });

  // ── Tree operations ───────────────────────────────────────────────
  function toggleExpand(docId: string) {
    const next = new Set(expandedNodes);
    if (next.has(docId)) {
      next.delete(docId);
    } else {
      next.add(docId);
      if (!childrenCache[docId]) {
        loadChildren(docId);
      }
    }
    expandedNodes = next;
  }

  async function loadChildren(parentId: string) {
    childrenLoading[parentId] = true;
    childrenLoading = childrenLoading;
    try {
      const children = await docs.children(parentId, { namespace: namespace ?? undefined });
      childrenCache[parentId] = children;
      childrenCache = childrenCache;
    } catch (e) {
      reportError('Load children', e);
    } finally {
      childrenLoading[parentId] = false;
      childrenLoading = childrenLoading;
    }
  }

  // ── Select document ───────────────────────────────────────────────
  async function selectDocument(docId: string) {
    selectedDocId = docId;
    editMode = false;
    docLoading = true;
    try {
      currentDoc = await docs.get(docId, { namespace: namespace ?? undefined });
      breadcrumb = buildBreadcrumb(currentDoc);
    } catch (e) {
      currentDoc = null;
      reportError('Load document', e);
    }
    docLoading = false;
  }

  // ── Search ───────────────────────────────────────────────────────
  let searchDebounce: ReturnType<typeof setTimeout> | null = null;

  function handleSearchInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    searchQuery = value;

    if (searchDebounce) clearTimeout(searchDebounce);
    if (!value.trim()) {
      searchActive = false;
      searchResults = [];
      return;
    }

    searchDebounce = setTimeout(() => {
      performSearch(value.trim());
    }, 400);
  }

  async function performSearch(query: string) {
    searchLoading = true;
    searchActive = true;
    activeTab = 'search';
    try {
      searchResults = await docs.search(query, { namespace: namespace ?? undefined, limit: 20 });
    } catch (e) {
      searchResults = [];
      reportError('Search', e);
    }
    searchLoading = false;
  }

  // ── Create document ───────────────────────────────────────────────
  function toggleCreateForm() {
    showCreateForm = !showCreateForm;
    newSlug = '';
    newTitle = '';
    newContent = '';
    newTags = '';
  }

  async function createDocument() {
    if (!newTitle.trim() || !newSlug.trim()) return;
    creating = true;
    try {
      const tags = newTags.split(',').map(t => t.trim()).filter(Boolean);
      await docs.create(newSlug.trim(), newTitle.trim(), newContent, {
        tags,
        namespace: namespace ?? undefined,
        parent: selectedDocId ?? undefined,
      });
      showCreateForm = false;
      await loadDocuments();
      if (selectedDocId) {
        await loadChildren(selectedDocId);
      }
    } catch (e) {
      reportError('Create', e);
    }
    creating = false;
  }

  // ── Edit / Save document ─────────────────────────────────────────
  async function startEdit() {
    if (!currentDoc) return;
    editMode = true;
    editContent = currentDoc.content;
    editTitle = currentDoc.title;
    editTags = currentDoc.tags.join(', ');
    editSlug = currentDoc.slug;
    previewMode = 'edit';
    await initCodeMirror();
  }

  function cancelEdit() {
    editMode = false;
    destroyCodeMirror();
  }

  async function saveDocument() {
    if (!currentDoc) return;
    editSaving = true;
    try {
      const tags = editTags.split(',').map(t => t.trim()).filter(Boolean);
      await docs.create(editSlug, editTitle.trim(), editContent, {
        tags,
        namespace: namespace ?? undefined,
        parent: currentDoc.parent_id ?? undefined,
      });
      currentDoc = await docs.get(currentDoc.slug, { namespace: namespace ?? undefined });
      editMode = false;
      await loadDocuments();
      destroyCodeMirror();
    } catch (e) {
      reportError('Save', e);
    }
    editSaving = false;
  }

  // ── Delete document ───────────────────────────────────────────────
  async function deleteDocument() {
    if (!currentDoc) return;
    try {
      await docs.delete(currentDoc.slug, { namespace: namespace ?? undefined });
      showDeleteConfirm = false;
      selectedDocId = null;
      currentDoc = null;
      await loadDocuments();
    } catch (e) {
      reportError('Delete', e);
      showDeleteConfirm = false;
    }
  }

  // ── Export document ──────────────────────────────────────────────
  function exportMarkdown() {
    if (!currentDoc) return;
    const content = `# ${currentDoc.title}\n\n${currentDoc.content}`;
    const blob = new Blob([content], { type: 'text/markdown' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${currentDoc.slug}.md`;
    a.click();
    URL.revokeObjectURL(url);
  }

  // ── Markdown rendering ──────────────────────────────────────────
  function renderMarkdown(text: string): string {
    try {
      const html = marked.parse(text, { async: false }) as string;
      return DOMPurify.sanitize(html);
    } catch {
      return text;
    }
  }

  // ── CodeMirror editor ────────────────────────────────────────────
  async function initCodeMirror() {
    if (cmLoaded) {
      updateEditorContent(editContent);
      return;
    }
    try {
      const { EditorView, keymap, lineNumbers, highlightActiveLineGutter, highlightSpecialChars, drawSelection, highlightActiveLine } = await import('@codemirror/view');
      const { EditorState } = await import('@codemirror/state');
      const { defaultKeymap, history, historyKeymap, indentWithTab } = await import('@codemirror/commands');
      const { markdown, markdownLanguage } = await import('@codemirror/lang-markdown');
      const { languages } = await import('@codemirror/language-data');
      const { syntaxHighlighting, defaultHighlightStyle, foldGutter, indentOnInput, bracketMatching, foldKeymap } = await import('@codemirror/language');
      const { oneDark } = await import('@codemirror/theme-one-dark');
      const { tags } = await import('@lezer/highlight');

      const mdLang = markdown({ base: markdownLanguage, codeLanguages: languages });

      // Catppuccin Mocha theme override for code blocks
      const catppuccinMocha = EditorView.theme({
        '&': { background: 'var(--bg-primary)', color: 'var(--text-primary)' },
        '.cm-content': { caretColor: 'var(--accent)', fontFamily: 'var(--font-mono)', fontSize: '0.85rem', lineHeight: '1.6', padding: '8px 0' },
        '.cm-line': { padding: '0 4px' },
        '.cm-gutters': { background: 'var(--bg-secondary)', borderRight: '1px solid var(--border)', color: 'var(--text-muted)', fontSize: '0.75rem' },
        '.cm-activeLineGutter': { backgroundColor: 'var(--bg-hover)', color: 'var(--text-secondary)' },
        '.cm-activeLine': { backgroundColor: 'rgba(49, 50, 68, 0.5)' },
        '.cm-selectionBackground': { backgroundColor: 'var(--accent-dim) !important' },
        '.cm-focused .cm-cursor': { borderLeftColor: 'var(--accent)' },
        '.cm-focused .cm-selectionBackground': { backgroundColor: 'var(--accent-dim)' },
        '&.cm-focused .cm-scroller': { outline: 'none' },
        '.cm-foldPlaceholder': { background: 'var(--bg-tertiary)', border: '1px solid var(--border)', color: 'var(--text-muted)' },
      }, { dark: true });

      const state = EditorState.create({
        doc: editContent,
        extensions: [
          lineNumbers(),
          highlightActiveLineGutter(),
          highlightSpecialChars(),
          history(),
          foldGutter(),
          drawSelection(),
          indentOnInput(),
          bracketMatching(),
          highlightActiveLine(),
          syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
          syntaxHighlighting(highlightActiveLine),
          keymap.of([
            ...defaultKeymap,
            ...historyKeymap,
            ...foldKeymap,
            indentWithTab,
          ]),
          mdLang,
          oneDark,
          catppuccinMocha,
          EditorView.updateListener.of((update) => {
            if (update.docChanged) {
              editContent = update.state.doc.toString();
            }
          }),
          EditorView.lineWrapping,
        ],
      });

      if (editorContainer) {
        editorView = new EditorView({ state, parent: editorContainer });
        cmLoaded = true;
      }
    } catch (e) {
      console.error('CodeMirror init failed:', e);
      cmLoaded = false;
    }
  }

  function updateEditorContent(content: string) {
    if (!editorView) return;
    const currentContent = editorView.state.doc.toString();
    if (currentContent !== content) {
      editorView.dispatch({
        changes: { from: 0, to: editorView.state.doc.length, insert: content },
      });
    }
  }

  function destroyCodeMirror() {
    if (editorView) {
      editorView.destroy();
      editorView = undefined;
      cmLoaded = false;
    }
  }

  // ── Helpers ───────────────────────────────────────────────────────
  function relativeTime(dateStr: string): string {
    if (!dateStr) return '';
    const now = Date.now();
    const then = new Date(dateStr).getTime();
    const diffMs = now - then;
    const diffSec = Math.floor(diffMs / 1000);
    if (diffSec < 60) return 'just now';
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHr = Math.floor(diffMin / 60);
    if (diffHr < 24) return `${diffHr}h ago`;
    const diffDay = Math.floor(diffHr / 24);
    if (diffDay < 30) return `${diffDay}d ago`;
    const diffMo = Math.floor(diffDay / 30);
    if (diffMo < 12) return `${diffMo}mo ago`;
    const diffYr = Math.floor(diffMo / 12);
    return `${diffYr}y ago`;
  }

  // Cleanup CodeMirror on unmount
  $effect(() => {
    return () => { destroyCodeMirror(); };
  });
</script>

<div class="docs-view">
  {#if lastError}
    <div class="error-banner">
      <span>{lastError}</span>
      <button class="error-dismiss" onclick={() => lastError = null} class:onclick>×</button>
    </div>
  {/if}

  <!-- Header -->
  <div class="docs-header">
    <div class="header-left">
      <h2>Documents</h2>
      <span class="count">{documents.length} documents</span>
    </div>
    <div class="header-actions">
      <button class="btn-new" onclick={toggleCreateForm}>
        {showCreateForm ? '✕ Cancel' : '+ New Document'}
      </button>
    </div>
  </div>

  <!-- Tabs -->
  <div class="tabs">
    <button class="tab" class:active={activeTab === 'browse'} onclick={() => { activeTab = 'browse'; searchQuery = ''; searchActive = false; searchResults = []; }}>Browse</button>
    <button class="tab" class:active={activeTab === 'search'} onclick={() => activeTab = 'search'}>Search</button>
  </div>

  <!-- Tab: Browse -->
  {#if activeTab === 'browse'}
    <div class="layout">
      <div class="doc-tree-panel">
        <!-- Create form -->
        {#if showCreateForm}
          <div class="create-form">
            <input type="text" class="input" placeholder="slug (url-safe identifier)" bind:value={newSlug} autofocus />
            <input type="text" class="input" placeholder="Title" bind:value={newTitle} />
            <textarea class="textarea" placeholder="Content (markdown)" bind:value={newContent} rows="4"></textarea>
            <input type="text" class="input" placeholder="Tags (comma-separated)" bind:value={newTags} />
            <div class="create-actions">
              <button class="btn-create" onclick={createDocument} disabled={!newTitle.trim() || !newSlug.trim() || creating}>
                {creating ? 'Creating...' : 'Create'}
              </button>
              <button class="btn-cancel" onclick={toggleCreateForm}>Cancel</button>
            </div>
          </div>
        {/if}

        <!-- Document tree (recursive via snippet) -->
        {#if loading}
          <div class="msg">Loading documents...</div>
        {:else if documents.length === 0}
          <div class="msg">
            <p>No documents yet.</p>
            <p class="sub">Create one via the button above or CLI:<br>
            <code>uteke doc create my-doc --title "My Doc"</code></p>
          </div>
        {:else}
          {#each documents as doc (doc.id)}
            {@const children = childrenCache[doc.id]}
            {@const isExpanded = expandedNodes.has(doc.id)}
            {@render treeNode(doc, isExpanded, children, childrenLoading[doc.id] ?? false)}
          {/each}
        {/if}
      </div>

      <!-- Document detail panel -->
      <div class="doc-detail-panel">
        {#if !selectedDocId}
          <div class="msg"><p>Select a document to view its content</p></div>
        {:else if docLoading}
          <div class="msg">Loading document...</div>
        {:else if !currentDoc}
          <div class="msg">Document not found.</div>
        {:else}
          <!-- Breadcrumb -->
          {#if breadcrumb.length > 1}
            <div class="breadcrumb">
              {#each breadcrumb as crumb, i}
                {#if i > 0}<span class="breadcrumb-sep">/</span>{/if}
                <button
                  class="breadcrumb-item"
                  class:current={i === breadcrumb.length - 1}
                  onclick={() => selectDocument(crumb.id)}
                >
                  {crumb.title}
                </button>
              {/each}
            </div>
          {/if}

          <!-- Detail header -->
          <div class="detail-header">
            <div class="detail-info">
              <h3>{currentDoc.title}</h3>
              <div class="detail-meta">
                <span class="slug">{currentDoc.slug}</span>
                <span>·</span>
                <span>v{currentDoc.version}</span>
                <span>·</span>
                <span>{relativeTime(currentDoc.updated_at)}</span>
                <span>·</span>
                <span>depth {currentDoc.depth}</span>
              </div>
              {#if currentDoc.tags.length > 0}
                <div class="tags">
                  {#each currentDoc.tags as tag}
                    <span class="tag">{tag}</span>
                  {/each}
                </div>
              {/if}
            </div>
            <div class="detail-actions">
              {#if !editMode}
                <button class="btn-action" onclick={startEdit}>✏ Edit</button>
                <button class="btn-action" onclick={exportMarkdown}>📥 Export</button>
                {#if !showDeleteConfirm}
                  <button class="btn-delete" onclick={() => showDeleteConfirm = true}>🗑 Delete</button>
                {:else}
                  <span class="delete-label">Delete "{currentDoc.title}"?</span>
                  <button class="btn-confirm-delete" onclick={deleteDocument}>Confirm</button>
                  <button class="btn-cancel-del" onclick={() => showDeleteConfirm = false}>Cancel</button>
                {/if}
              {/if}
            </div>
          </div>

          <!-- Editor mode -->
          {#if editMode}
            <div class="editor-area">
              <div class="editor-top-row">
                <input type="text" class="input title-input" placeholder="Title" bind:value={editTitle} />
                <input type="text" class="input slug-input" placeholder="Slug" bind:value={editSlug} />
                <div class="view-toggle">
                  <button class="view-btn" class:active={previewMode === 'edit'} onclick={() => previewMode = 'edit'}>Edit</button>
                  <button class="view-btn" class:active={previewMode === 'split'} onclick={() => previewMode = 'split'}>Split</button>
                  <button class="view-btn" class:active={previewMode === 'preview'} onclick={() => previewMode = 'preview'}>Preview</button>
                </div>
              </div>
              <div class="editor-content-area">
                {#if previewMode === 'edit' || previewMode === 'split'}
                  <div class="editor-pane" class:narrow={previewMode === 'split'}>
                    <div bind:this={editorContainer} class="cm-editor-wrap"></div>
                  </div>
                {/if}
                {#if previewMode === 'preview' || previewMode === 'split'}
                  <div class="preview-pane" class:narrow={previewMode === 'split'}>
                    <div class="doc-content">
                      {@html renderMarkdown(editContent)}
                    </div>
                  </div>
                {/if}
              </div>
              <div class="editor-footer">
                <input type="text" class="input tags-input" placeholder="Tags (comma-separated)" bind:value={editTags} />
                <div class="editor-actions">
                  <button class="btn-create" onclick={saveDocument} disabled={editSaving}>
                    {editSaving ? 'Saving...' : '💾 Save'}
                  </button>
                  <button class="btn-cancel" onclick={cancelEdit}>Cancel</button>
                </div>
              </div>
            </div>
          {:else}
            <!-- Read mode — rendered markdown -->
            <div class="doc-content">
              {@html renderMarkdown(currentDoc.content)}
            </div>
          {/if}
        {/if}
      </div>
    </div>
  {/if}

  <!-- Tab: Search -->
  {#if activeTab === 'search'}
    <div class="search-panel">
      <div class="search-bar">
        <input
          type="text"
          class="input search-input"
          placeholder="Search documents (semantic + keyword)..."
          value={searchQuery}
          oninput={handleSearchInput}
          autofocus
        />
      </div>

      {#if searchLoading}
        <div class="msg">Searching...</div>
      {:else if searchActive && searchResults.length === 0}
        <div class="msg">
          <p>No results for "{searchQuery}"</p>
          <p class="sub">Try different keywords or broader terms.</p>
        </div>
      {:else if searchActive}
        <div class="search-results">
          {#each searchResults as result, i (result.document.id)}
            <button
              class="search-result-item"
              onclick={() => {
                selectDocument(result.document.id);
                activeTab = 'browse';
              }}
            >
              <div class="search-result-header">
                <span class="search-result-title">{result.document.title}</span>
                <span class="search-result-score">{(result.score * 100).toFixed(0)}%</span>
              </div>
              {#if result.chunk_heading}
                <div class="search-result-heading">{result.chunk_heading}</div>
              {/if}
              <div class="search-result-snippet">{result.chunk_snippet}</div>
              <div class="search-result-meta">
                <span class="mode-badge">{result.mode}</span>
                <span>{result.document.namespace}</span>
                <span>·</span>
                <span>{relativeTime(result.document.updated_at)}</span>
              </div>
            </button>
          {/each}
        </div>
      {:else}
        <div class="msg">
          <p>Type to search documents</p>
          <p class="sub">Uses hybrid search (semantic + keyword) via Uteke FTS5 embeddings.</p>
        </div>
      {/if}
    </div>
  {/if}
</div>

{#snippet treeNode(doc: DocSummary, isExpanded: boolean, children: DocSummary[] | undefined, isLoading: boolean)}
  <button
    class="doc-item"
    class:selected={selectedDocId === doc.id}
    style="padding-left: calc(12px + {doc.depth * 16}px)"
    onclick={() => selectDocument(doc.id)}
  >
    {#if doc.has_children}
      <span
        class="expand-icon"
        class:expanded={isExpanded}
        onclick={(e) => { e.stopPropagation(); toggleExpand(doc.id); }}
      >▶</span>
    {:else}
      <span class="expand-icon leaf"></span>
    {/if}
    <span class="doc-title">{doc.title}</span>
    <span class="doc-meta">{relativeTime(doc.updated_at)}</span>
  </button>

  {#if isExpanded && isLoading}
    <div class="loading-children" style="padding-left: calc(12px + {(doc.depth + 1) * 16}px)">
      Loading...
    </div>
  {:else if isExpanded && children}
    {#each children as child (child.id)}
      {@const grandChildren = childrenCache[child.id]}
      {@const isChildExpanded = expandedNodes.has(child.id)}
      {@render treeNode(child, isChildExpanded, grandChildren, childrenLoading[child.id] ?? false)}
    {/each}
  {/if}
{/snippet}

<style>
  .docs-view { height: 100%; display: flex; flex-direction: column; }

  .error-banner { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 8px 16px; background: rgba(230,69,83,0.12); color: #e64553; font-size: 0.8rem; border-bottom: 1px solid rgba(230,69,83,0.3); }
  .error-dismiss { background: none; border: none; color: inherit; font-size: 1.1rem; cursor: pointer; padding: 0 4px; line-height: 1; }

  .docs-header { padding: 16px 24px 8px; display: flex; align-items: baseline; justify-content: space-between; gap: 12px; border-bottom: 1px solid var(--border); }
  .header-left { display: flex; align-items: baseline; gap: 12px; }
  h2 { font-size: 1.1rem; }
  .count { font-size: 0.8rem; color: var(--text-muted); }
  .header-actions { display: flex; gap: 8px; }
  .btn-new { font-size: 0.8rem; padding: 4px 12px; background: var(--accent); color: var(--bg-primary); border: none; border-radius: 6px; cursor: pointer; font-weight: 500; }
  .btn-new:hover { opacity: 0.85; }

  /* Tabs */
  .tabs { display: flex; gap: 0; border-bottom: 1px solid var(--border); padding: 0 24px; flex-shrink: 0; }
  .tab { font-size: 0.85rem; padding: 8px 16px; background: transparent; color: var(--text-muted); border: none; border-bottom: 2px solid transparent; cursor: pointer; transition: color 0.15s, border-color 0.15s; }
  .tab:hover { color: var(--text-secondary); }
  .tab.active { color: var(--accent); border-bottom-color: var(--accent); }

  /* Layout */
  .layout { flex: 1; display: flex; overflow: hidden; }

  /* Tree panel */
  .doc-tree-panel { width: 320px; overflow-y: auto; padding: 8px 0; border-right: 1px solid var(--border); display: flex; flex-direction: column; flex-shrink: 0; }

  /* Create form */
  .create-form { padding: 10px 12px; background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 8px; margin: 4px 12px 8px; display: flex; flex-direction: column; gap: 6px; }
  .input { font-size: 0.85rem; padding: 6px 10px; background: var(--bg-primary); color: var(--text-primary); border: 1px solid var(--border); border-radius: 4px; outline: none; font-family: inherit; }
  .input:focus { border-color: var(--accent); }
  .input::placeholder { color: var(--text-muted); opacity: 0.6; }
  .textarea { font-size: 0.85rem; padding: 6px 10px; background: var(--bg-primary); color: var(--text-primary); border: 1px solid var(--border); border-radius: 4px; outline: none; font-family: inherit; resize: vertical; min-height: 60px; }
  .textarea:focus { border-color: var(--accent); }
  .textarea::placeholder { color: var(--text-muted); opacity: 0.6; }
  .create-actions { display: flex; gap: 8px; }
  .btn-create { font-size: 0.8rem; padding: 5px 14px; background: var(--accent); color: var(--bg-primary); border: none; border-radius: 4px; cursor: pointer; font-weight: 500; }
  .btn-create:disabled { opacity: 0.4; cursor: not-allowed; }
  .btn-cancel { font-size: 0.8rem; padding: 5px 14px; background: transparent; color: var(--text-secondary); border: 1px solid var(--border); border-radius: 4px; cursor: pointer; }
  .btn-cancel:hover { border-color: var(--text-muted); }

  /* Document items in tree */
  .doc-item { display: flex; align-items: center; gap: 6px; padding: 6px 12px; background: transparent; border: 1px solid transparent; border-radius: 4px; cursor: pointer; text-align: left; width: 100%; font-size: 0.85rem; }
  .doc-item:hover { background: var(--bg-hover); }
  .doc-item.selected { background: var(--bg-hover); border-color: var(--accent); }
  .expand-icon { font-size: 0.6rem; width: 12px; text-align: center; flex-shrink: 0; transition: transform 0.15s; color: var(--text-muted); display: inline-block; }
  .expand-icon.expanded { transform: rotate(90deg); }
  .expand-icon.leaf { visibility: hidden; }
  .doc-title { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text-primary); }
  .doc-meta { font-size: 0.65rem; color: var(--text-muted); flex-shrink: 0; }
  .loading-children { font-size: 0.75rem; color: var(--text-muted); padding: 4px 12px; }

  /* Detail panel */
  .doc-detail-panel { flex: 1; overflow-y: auto; padding: 16px 24px; display: flex; flex-direction: column; }

  /* Breadcrumb */
  .breadcrumb { display: flex; align-items: center; gap: 4px; margin-bottom: 12px; font-size: 0.75rem; flex-wrap: wrap; }
  .breadcrumb-item { background: none; border: none; color: var(--text-muted); cursor: pointer; padding: 2px 4px; font-size: 0.75rem; border-radius: 3px; }
  .breadcrumb-item:hover { color: var(--accent); background: var(--bg-hover); }
  .breadcrumb-item.current { color: var(--accent); cursor: default; }
  .breadcrumb-sep { color: var(--text-muted); opacity: 0.5; }

  /* Detail header */
  .detail-header { display: flex; justify-content: space-between; align-items: flex-start; gap: 12px; margin-bottom: 16px; flex-wrap: wrap; }
  .detail-info { flex: 1; min-width: 200px; }
  .detail-header h3 { font-size: 1.1rem; color: var(--accent); margin-bottom: 4px; }
  .detail-meta { font-size: 0.75rem; color: var(--text-muted); display: flex; gap: 4px; align-items: center; margin-bottom: 4px; }
  .slug { font-family: var(--font-mono); color: var(--text-secondary); }
  .tags { display: flex; gap: 4px; flex-wrap: wrap; }
  .tag { font-size: 0.7rem; padding: 2px 6px; background: var(--bg-hover); color: var(--text-secondary); border-radius: 3px; }
  .detail-actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; flex-wrap: wrap; }

  .btn-action { font-size: 0.75rem; padding: 3px 10px; background: transparent; color: var(--text-secondary); border: 1px solid var(--border); border-radius: 4px; cursor: pointer; }
  .btn-action:hover { border-color: var(--accent); color: var(--accent); }
  .btn-delete { font-size: 0.75rem; padding: 3px 10px; background: transparent; color: var(--text-muted); border: 1px solid var(--border); border-radius: 4px; cursor: pointer; }
  .btn-delete:hover { color: #e64553; border-color: #e64553; }
  .delete-label { font-size: 0.75rem; color: var(--text-secondary); }
  .btn-confirm-delete { font-size: 0.75rem; padding: 3px 10px; background: #e64553; color: #fff; border: none; border-radius: 4px; cursor: pointer; }
  .btn-confirm-delete:hover { background: #c7374a; }
  .btn-cancel-del { font-size: 0.75rem; padding: 3px 10px; background: transparent; color: var(--text-secondary); border: 1px solid var(--border); border-radius: 4px; cursor: pointer; }

  /* Editor */
  .editor-area { flex: 1; display: flex; flex-direction: column; gap: 8px; min-height: 0; }
  .editor-top-row { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .title-input { font-size: 1.05rem; font-weight: 600; padding: 8px 10px; flex: 1; min-width: 150px; }
  .slug-input { font-family: var(--font-mono); font-size: 0.85rem; flex: 1; min-width: 150px; }
  .view-toggle { display: flex; gap: 0; border: 1px solid var(--border); border-radius: 4px; overflow: hidden; flex-shrink: 0; }
  .view-btn { font-size: 0.75rem; padding: 4px 10px; background: transparent; color: var(--text-muted); border: none; border-right: 1px solid var(--border); cursor: pointer; }
  .view-btn:last-child { border-right: none; }
  .view-btn.active { background: var(--accent); color: var(--bg-primary); }
  .view-btn:hover:not(.active) { background: var(--bg-hover); color: var(--text-secondary); }

  .editor-content-area { flex: 1; display: flex; gap: 0; min-height: 0; border: 1px solid var(--border); border-radius: 6px; overflow: hidden; }
  .editor-pane { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .editor-pane.narrow { flex: 1; }
  .preview-pane { flex: 1; overflow-y: auto; border-left: 1px solid var(--border); padding: 16px; background: var(--bg-secondary); }
  .preview-pane.narrow { flex: 1; }
  .cm-editor-wrap { flex: 1; overflow: auto; }
  .cm-editor-wrap :global(.cm-editor) { height: 100%; }

  .editor-footer { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .tags-input { flex: 1; min-width: 200px; }
  .editor-actions { display: flex; gap: 8px; }

  /* Document content (read mode + preview) */
  .doc-content { font-size: 0.85rem; color: var(--text-primary); line-height: 1.7; flex: 1; }
  .doc-content :global(h1) { font-size: 1.3rem; color: var(--accent); margin: 20px 0 12px; padding-bottom: 6px; border-bottom: 1px solid var(--border); }
  .doc-content :global(h2) { font-size: 1.1rem; color: var(--accent); margin: 16px 0 8px; }
  .doc-content :global(h3) { font-size: 1rem; color: var(--accent); margin: 12px 0 6px; }
  .doc-content :global(h4) { font-size: 0.9rem; color: var(--text-primary); margin: 10px 0 4px; font-weight: 600; }
  .doc-content :global(p) { margin: 8px 0; }
  .doc-content :global(ul), .doc-content :global(ol) { margin: 8px 0; padding-left: 24px; }
  .doc-content :global(li) { margin: 4px 0; }
  .doc-content :global(blockquote) { border-left: 3px solid var(--accent); padding: 4px 12px; margin: 8px 0; color: var(--text-secondary); background: var(--bg-tertiary); border-radius: 0 4px 4px 0; }
  .doc-content :global(code) { font-family: var(--font-mono); font-size: 0.8rem; padding: 1px 5px; background: var(--bg-tertiary); border-radius: 3px; color: var(--text-secondary); }
  .doc-content :global(pre) { background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 6px; padding: 12px; overflow-x: auto; margin: 12px 0; }
  .doc-content :global(pre code) { background: none; padding: 0; border-radius: 0; }
  .doc-content :global(a) { color: var(--accent); text-decoration: underline; }
  .doc-content :global(table) { border-collapse: collapse; width: 100%; margin: 12px 0; }
  .doc-content :global(th), .doc-content :global(td) { border: 1px solid var(--border); padding: 6px 10px; text-align: left; font-size: 0.8rem; }
  .doc-content :global(th) { background: var(--bg-tertiary); color: var(--text-secondary); }
  .doc-content :global(hr) { border: none; border-top: 1px solid var(--border); margin: 16px 0; }
  .doc-content :global(img) { max-width: 100%; border-radius: 6px; }

  /* Search panel */
  .search-panel { flex: 1; display: flex; flex-direction: column; padding: 16px 24px; overflow-y: auto; }
  .search-bar { margin-bottom: 16px; }
  .search-input { width: 100%; font-size: 0.9rem; padding: 10px 14px; }

  .search-results { display: flex; flex-direction: column; gap: 8px; }
  .search-result-item { display: block; padding: 12px 16px; background: var(--bg-tertiary); border: 1px solid var(--border); border-radius: 6px; cursor: pointer; text-align: left; width: 100%; }
  .search-result-item:hover { border-color: var(--accent); }
  .search-result-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px; }
  .search-result-title { font-size: 0.9rem; color: var(--text-primary); font-weight: 500; }
  .search-result-score { font-size: 0.75rem; color: var(--green); font-family: var(--font-mono); }
  .search-result-heading { font-size: 0.8rem; color: var(--accent); margin-bottom: 4px; }
  .search-result-snippet { font-size: 0.8rem; color: var(--text-secondary); line-height: 1.4; margin-bottom: 6px; }
  .search-result-meta { display: flex; gap: 4px; align-items: center; font-size: 0.7rem; color: var(--text-muted); }
  .mode-badge { font-size: 0.65rem; padding: 1px 6px; background: rgba(137,180,250,0.15); color: var(--accent); border-radius: 3px; }

  /* General */
  .msg { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; color: var(--text-muted); text-align: center; gap: 8px; padding: 32px; }
  .msg .sub { font-size: 0.85rem; opacity: 0.7; line-height: 1.6; }
  .msg code { font-family: var(--font-mono); font-size: 0.8rem; padding: 2px 6px; background: var(--bg-tertiary); border-radius: 3px; color: var(--text-secondary); }
</style>
