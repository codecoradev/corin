<script lang="ts">
  import { docs } from '../ts/ipc';
  import type { DocEntry, DocSearchResult, VersionStatus } from '../ts/types';
  import { EditorView, keymap, lineNumbers, highlightActiveLineGutter, highlightSpecialChars, drawSelection, dropCursor, rectangularSelection, crosshairCursor, highlightActiveLine } from '@codemirror/view';
  import { EditorState } from '@codemirror/state';
  import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
  import { bracketMatching, foldGutter, indentOnInput, foldKeymap, defaultHighlightStyle, HighlightStyle, syntaxHighlighting } from '@codemirror/language';
  import { markdown, markdownLanguage } from '@codemirror/lang-markdown';
  import { languages } from '@codemirror/language-data';
  import { searchKeymap, highlightSelectionMatches } from '@codemirror/search';
  import { autocompletion, completionKeymap, closeBrackets, closeBracketsKeymap } from '@codemirror/autocomplete';
  import { tags } from '@lezer/highlight';
  import {
    Search,
    SquarePen,
    Columns2,
    Eye,
    ChevronDown,
    FileText,
    Folder,
    FolderOpen,
    Save,
    Download,
    Trash2,
    Plus,
  } from 'lucide-svelte';
  import { open as shellOpen } from '@tauri-apps/plugin-shell';
  import { slide } from 'svelte/transition';
  import { pendingDocSlug } from '../stores/nav';
  import { renderMarkdown as renderMd } from '../utils/markdown';
  import { formatDate, getWordCount, getReadingTime } from '../utils/format';

  // ─── Catppuccin Mocha theme for CodeMirror ───────────────────────
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

  // ─── Props ─────────────────────────────────────────────────────────
  // No props — documents are global since uteke v0.7.0 (#614).

  // ─── Editor / Preview mode ─────────────────────────────────────────
  type ViewMode = 'edit' | 'preview' | 'split';
  let viewMode = $state<ViewMode>('edit');

  // ─── State ─────────────────────────────────────────────────────────
  let rootDocs = $state<DocEntry[]>([]);
  let expandedIds = $state<Set<string>>(new Set());
  let childrenCache = $state<Map<string, DocEntry[]>>(new Map());
  // Flat id → DocEntry lookup for ancestor lookups (breadcrumb navigation).
  let docById = $state<Map<string, DocEntry>>(new Map());
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
  let errorTimeout: ReturnType<typeof setTimeout> | null = null;
  let success = $state('');
  let successTimeout: ReturnType<typeof setTimeout> | null = null;
  let editorView = $state<EditorView | null>(null);
  let showProps = $state(false);

  // Uteke version gate — documents require >= 0.7.0 (#614).
  let versionStatus = $state<VersionStatus | null>(null);
  let updating = $state(false);

  // ─── Bound DOM elements for scroll reset ───────────────────────────
  let editorContainer: HTMLElement | null = null;
  let previewContainer: HTMLElement | null = null;

  // ─── Markdown rendering ────────────────────────────────────────────
  let renderedHtml = $state('');

  // Re-render when content changes
  $effect(() => {
    if (viewMode !== 'edit') {
      renderedHtml = renderMd(editorContent);
    }
  });

  // ─── Show error with auto-dismiss ───────────────────────────────────
  function showError(msg: string) {
    error = msg;
    success = '';
    if (successTimeout) clearTimeout(successTimeout);
    if (errorTimeout) clearTimeout(errorTimeout);
    errorTimeout = setTimeout(() => { error = ''; }, 8000);
  }

  // ─── Show success with auto-dismiss ─────────────────────────────────
  function showSuccess(msg: string) {
    success = msg;
    error = '';
    if (errorTimeout) clearTimeout(errorTimeout);
    if (successTimeout) clearTimeout(successTimeout);
    successTimeout = setTimeout(() => { success = ''; }, 3000);
  }

  /** Whether a doc has children in the loaded tree (authoritative, from cache). */
  function hasKids(doc: DocEntry): boolean {
    return childrenCache.has(doc.id) && (childrenCache.get(doc.id)?.length ?? 0) > 0;
  }

  // ─── Load all documents & build the tree client-side ──────────────
  // Fetches the full flat doc list once and assembles parent→children so the
  // entire hierarchy is visible upfront (Obsidian/Outline-like), rather than
  // only roots with lazy-expanded children.
  async function loadRootDocs() {
    loading = true;
    try {
      const all = await docs.list();
      // Group by parent_id; roots have no parent_id.
      const byParent = new Map<string, DocEntry[]>();
      const byId = new Map<string, DocEntry>();
      const roots: DocEntry[] = [];
      for (const d of all) {
        byId.set(d.id, d);
        const pid = d.parent_id ?? null;
        if (pid) {
          const arr = byParent.get(pid) ?? [];
          arr.push(d);
          byParent.set(pid, arr);
        } else {
          roots.push(d);
        }
      }
      childrenCache = byParent;
      rootDocs = roots;
      docById = byId;
      // Expand every folder by default so the full tree is visible.
      expandedIds = new Set(byParent.keys());
    } catch (e: any) {
      showError(e.toString());
    } finally {
      loading = false;
    }
  }

  // ─── Load children for a node ────────────────────────────────────
  async function loadChildren(docId: string) {
    if (childrenCache.has(docId)) return;
    try {
      const children = await docs.list({ parent: docId });
      // Immutable update — Svelte 5 does not re-render {@const} reads when a
      // $state Map is mutated in place + reassigned to the same ref.
      const next = new Map(childrenCache);
      next.set(docId, children);
      childrenCache = next;
    } catch (e: any) {
      showError(`Failed to load children: ${e}`);
    }
  }

  // ─── Load all descendants (for selected doc's tree path) ─────────
  async function expandPathToDoc(docId: string) {
    // Children are pre-built; walk the cached tree and expand the path.
    // Collect into a new Set and reassign once — Svelte 5 needs a new ref
    // to re-render {@const} reads of a $state Set.
    const next = new Set(expandedIds);
    function searchLevel(entries: DocEntry[]): boolean {
      for (const entry of entries) {
        if (entry.id === docId) return true;
        if (hasKids(entry)) {
          const kids = childrenCache.get(entry.id) ?? [];
          next.add(entry.id);
          if (searchLevel(kids)) return true;
        }
      }
      return false;
    }
    searchLevel(rootDocs);
    expandedIds = next;
  }

  // ─── Toggle tree node ─────────────────────────────────────────────
  async function toggleNode(doc: DocEntry) {
    // Immutable update (new Set) — required for Svelte 5 to re-render the
    // {@const expanded = expandedIds.has(...)} reads in the tree snippet.
    // Mutating in place + reassigning the same ref does NOT trigger updates.
    const next = new Set(expandedIds);
    if (next.has(doc.id)) {
      next.delete(doc.id);
      expandedIds = next;
    } else {
      next.add(doc.id);
      expandedIds = next;
      // Children are pre-built in loadRootDocs(); load lazily only if missing.
      if (!childrenCache.has(doc.id) && doc.has_children) {
        await loadChildren(doc.id);
      }
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
      // Default to preview mode when opening an existing document
      viewMode = 'preview';
      // Auto-expand this document's subtree (children pre-built in loadRootDocs).
      if (hasKids(full)) {
        expandedIds = new Set([...expandedIds, full.id]);
      }
      // Auto-expand tree path to this document
      await expandPathToDoc(full.id);
      // Reset scroll positions to top for the new document
      requestAnimationFrame(() => {
        if (previewContainer) previewContainer.scrollTop = 0;
        if (editorContainer) {
          const scroller = editorContainer.querySelector('.cm-scroller');
          if (scroller) scroller.scrollTop = 0;
        }
      });
    } catch (e: any) {
      showError(e.toString());
    } finally {
      loading = false;
    }
  }

  // ─── Navigate to document by slug (from markdown links) ─────────
  async function navigateToSlug(slug: string) {
    try {
      const doc = await docs.get({ slug });
      await selectDoc(doc);
    } catch (e: any) {
      showError(`Document not found: ${slug}`);
    }
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
      searchResults = await docs.search(searchQuery, { limit: 20 });
    } catch (e: any) {
      showError(e.toString());
    } finally {
      searching = false;
    }
  }

  // ─── New doc ──────────────────────────────────────────────────────
  function newDoc() {
    showNewDoc = true;
    selectedDoc = null;
    editorTitle = '';
    editorSlug = '';
    editorContent = '# New Document\n\n';
    editorTags = '';
    showSearchResults = false;
    viewMode = 'edit';
  }

  // ─── Save (with Ctrl/Cmd+S shortcut) ─────────────────────────────
  async function saveDoc() {
    if (!editorSlug.trim()) {
      showError('Slug is required');
    return;
    }
    saving = true;
    try {
      const tags = editorTags.split(',').map(t => t.trim()).filter(Boolean);
      if (selectedDoc && !showNewDoc) {
        // Existing document → update
        try {
          // Try /doc/update first (available in uteke 0.6.7+)
          const updated = await docs.update({
            id: selectedDoc.id,
            title: editorTitle || editorSlug,
            content: editorContent,
            tags,
          });
          selectedDoc = updated;
        } catch (e: any) {
          if (e.toString().includes('404')) {
            // Fallback: server doesn't have /doc/update route.
            // Use /doc/create (which is an upsert) + re-fetch full doc.
            await docs.create(
              editorSlug,
              editorTitle || editorSlug,
              editorContent,
              { tags },
            );
            // create returns only {id, slug} — re-fetch for full state
            const full = await docs.get({ id: selectedDoc.id });
            selectedDoc = full;
          } else {
            throw e;
          }
        }
      } else {
        // New document → create via /doc/create
        const parent = selectedDoc?.id ?? undefined;
        await docs.create(editorSlug, editorTitle || editorSlug, editorContent, {
          tags,
          parent,
        });
        // create returns only {id, slug} — re-fetch for full state
        const full = await docs.get({ slug: editorSlug });
        selectedDoc = full;
        showNewDoc = false;
      }
      await loadRootDocs();
      showSuccess(selectedDoc && !showNewDoc ? 'Document updated' : 'Document saved');
    } catch (e: any) {
      showError(e.toString());
    } finally {
      saving = false;
    }
  }

  // ─── Delete ──────────────────────────────────────────────────────
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
      showError(e.toString());
    }
    showDeleteConfirm = false;
    deleteTarget = null;
  }

  // ─── Export ──────────────────────────────────────────────────────
  async function exportDoc() {
    if (!selectedDoc) return;
    try {
      await docs.exportFile(selectedDoc);
      showSuccess('Document exported');
    } catch (e: any) {
      showError(`Export failed: ${e}`);
    }
  }

  // ─── CodeMirror ─────────────────────────────────────────────────
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

  // Watch editorContent changes from outside
  $effect(() => {
    if (editorView && (selectedDoc || showNewDoc)) {
      const current = editorView.state.doc.toString();
      if (current !== editorContent) {
        updateEditor(editorContent);
      }
    }
  });

  // ─── Handle clicks in markdown preview ─────────────────────────────
  // Internal document links → navigate in-app.
  // External links → open in system browser via Tauri shell plugin.
  //   (Tauri's webview does NOT auto-open external links on click —
  //    we must explicitly call shell.open() and prevent default nav.)
  function handlePreviewClick(e: MouseEvent) {
    const anchor = (e.target as HTMLElement).closest('a');
    if (!anchor) return;
    const href = anchor.getAttribute('href');
    if (!href) return;

    // Always prevent default — Tauri webview would otherwise try to
    // navigate internally and fail.
    e.preventDefault();
    e.stopPropagation();

    const isExternal = href.startsWith('http://') || href.startsWith('https://')
      || href.startsWith('mailto:') || href.startsWith('tel:');

    if (isExternal) {
      // Open in system default browser
      shellOpen(href).catch((err) => showError(`Cannot open link: ${err}`));
      return;
    }

    // Internal link: extract slug and navigate in-app
    const slug = href.replace(/^\.\/+/, '').replace(/^\/+/, '').split(/[?#]/)[0].trim();
    if (!slug) return;
    navigateToSlug(slug);
  }

  // ─── Build breadcrumb path ────────────────────────────────────────
  // `selectedDoc.path` is a materialized ancestor chain of UUIDs
  // ("/uuid/uuid/"). Map each segment to its DocEntry via the flat lookup so
  // each crumb is clickable and shows a real title.
  function getBreadcrumb(): DocEntry[] {
    if (!selectedDoc?.path) return [];
    return selectedDoc.path
      .split('/')
      .filter(Boolean)
      .map((id) => docById.get(id))
      .filter((d): d is DocEntry => !!d);
  }

  // ─── Word count & reading time ────────────────────────────────────


  // ─── Lifecycle ───────────────────────────────────────────────────
  $effect(() => {
    init();
  });

  // Open a document requested from elsewhere (e.g. a unified-search hit in
  // MemoryList). `pendingDocSlug` is set before navigating here; consume it.
  $effect(() => {
    const slug = $pendingDocSlug;
    if (slug) {
      pendingDocSlug.set(null);
      navigateToSlug(slug);
    }
  });

  /** Probe the installed uteke version; load docs only if supported. */
  async function init() {
    try {
      versionStatus = await docs.versionStatus();
    } catch {
      versionStatus = null;
    }
    if (versionStatus?.supported !== false) {
      await loadRootDocs();
    } else {
      loading = false;
    }
  }

  /** Run `uteke upgrade`, then re-probe and (if supported) load docs. */
  async function updateUteke() {
    updating = true;
    try {
      versionStatus = await docs.selfUpdate();
      if (versionStatus.supported) {
        success = 'Uteke updated — documents enabled.';
        successTimeout = setTimeout(() => { success = ''; }, 3000);
        await loadRootDocs();
      } else {
        showError(`Still on ${versionStatus.current ?? 'unknown'} after update.`);
      }
    } catch (e: any) {
      showError(`Update failed: ${e}`);
    } finally {
      updating = false;
    }
  }

  // Cleanup on destroy
  $effect(() => {
    return () => {
      if (editorView) editorView.destroy();
    };
  });
</script>

<svelte:window
  onkeydown={(e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      e.preventDefault();
      saveDoc();
    }
  }}
/>

<div class="docs-view">
  <div class="docs-header">
    <h2>Documents</h2>
    <span class="count">{rootDocs.length} root docs</span>
    <div class="header-actions">
      <button class="action-btn new-btn" onclick={newDoc}>
        <Plus size={14} strokeWidth={2.5} /> New
      </button>
    </div>
  </div>

  {#if versionStatus && !versionStatus.supported}
    <div class="version-banner">
      <div class="vb-text">
        <strong>Uteke {versionStatus.required}+ required for documents.</strong>
        Detected: {versionStatus.current ?? 'unknown'}.
      </div>
      <button class="vb-btn" onclick={updateUteke} disabled={updating}>
        {updating ? 'Updating…' : 'Update uteke'}
      </button>
    </div>
  {/if}

  {#if error}
    <div class="error-bar">
      <span class="error-text">{error}</span>
      <button class="dismiss-btn" onclick={() => (error = '')}>✕</button>
    </div>
  {/if}
  {#if success}
    <div class="success-bar">
      <span class="success-text">{success}</span>
      <button class="dismiss-btn success-dismiss" onclick={() => (success = '')}>✕</button>
    </div>
  {/if}

  <div class="layout">
    <!-- ─── Left Panel: Tree + Search ─────────────────────────── -->
    <div class="doc-tree-panel">
      <div class="search-bar">
        <Search size={14} strokeWidth={2} class="search-icon" />
        <input
          type="text"
          placeholder="Search documents..."
          value={searchQuery}
          oninput={onSearchInput}
        />
        {#if searchQuery}
          <button class="search-clear" onclick={() => { searchQuery = ''; showSearchResults = false; searchResults = []; }}>✕</button>
        {/if}
      </div>

      {#if searching}
        <div class="msg"><span class="spinner"></span> Searching...</div>
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
        <div class="msg"><span class="spinner"></span> Loading...</div>
      {:else if rootDocs.length === 0}
        <div class="empty-tree">
          <div class="empty-tree-icon">📄</div>
          <p>No documents yet</p>
          <button class="action-btn new-btn small" onclick={newDoc}><Plus size={12} strokeWidth={2.5} /> Create first doc</button>
        </div>
      {:else}
        <div class="doc-tree">
          {#each rootDocs as doc (doc.id)}
            {@render treeNode(doc)}
          {/each}
        </div>
      {/if}
    </div>

    <!-- ─── Right Panel: Editor/Preview ─────────────────────────── -->
    <div class="editor-panel">
      {#if selectedDoc || showNewDoc}
        <!-- Top bar: breadcrumb + mode toggle -->
        <div class="top-bar">
          <div class="breadcrumb">
            {#if selectedDoc && !showNewDoc}
              <button class="crumb-link" onclick={() => { selectedDoc = null; showNewDoc = false; }}>Documents</button>
              {#each getBreadcrumb() as ancestor, i}
                <span class="crumb-sep">/</span>
                <button class="crumb-link" title={ancestor.title || ancestor.slug} onclick={() => selectDoc(ancestor)}>{ancestor.title || ancestor.slug}</button>
              {/each}
              <span class="crumb-sep">/</span>
              <span class="crumb-current">{selectedDoc.title}</span>
            {:else}
              <span class="crumb-current">New Document</span>
            {/if}
          </div>

          <!-- View mode toggle -->
          <div class="mode-toggle">
            <button
              class="mode-btn"
              class:active={viewMode === 'edit'}
              onclick={() => (viewMode = 'edit')}
              title="Edit (E)"
            >
              <SquarePen size={14} strokeWidth={2} />
              Edit
            </button>
            <button
              class="mode-btn"
              class:active={viewMode === 'split'}
              onclick={() => (viewMode = 'split')}
              title="Split view (S)"
            >
              <Columns2 size={14} strokeWidth={2} />
              Split
            </button>
            <button
              class="mode-btn"
              class:active={viewMode === 'preview'}
              onclick={() => (viewMode = 'preview')}
              title="Preview (P)"
            >
              <Eye size={14} strokeWidth={2} />
              Preview
            </button>
          </div>
        </div>

        <!-- Properties row: title, slug, tags -->
        {#if showProps}
          <div class="props-row">
            <input type="text" class="prop-input title-input" placeholder="Document title..." bind:value={editorTitle} />
            <input type="text" class="prop-input slug-input" placeholder="slug-name" bind:value={editorSlug} />
            <input type="text" class="prop-input tags-input" placeholder="tag1, tag2" bind:value={editorTags} />
          </div>
        {/if}

        <!-- Meta bar: version, date + actions -->
        <div class="meta-bar">
          <div class="meta-left">
            <button class="props-toggle" onclick={() => (showProps = !showProps)}>
              <ChevronDown size={12} strokeWidth={2} />
              Properties
            </button>
            {#if selectedDoc && !showNewDoc}
              <span class="meta-item">v{selectedDoc.version ?? 1}</span>
              {#if selectedDoc.updated_at}
                <span class="meta-item">{formatDate(selectedDoc.updated_at)}</span>
              {/if}
            {:else}
              <span class="meta-item">New draft</span>
            {/if}
            <span class="meta-item meta-dim">{getWordCount(editorContent)} words{getReadingTime(getWordCount(editorContent)) ? ` · ${getReadingTime(getWordCount(editorContent))}` : ''}</span>
          </div>
          <div class="meta-actions">
            <button class="icon-btn" onclick={saveDoc} disabled={saving} title="Save (Ctrl+S)">
              {#if saving}
                <span class="spinner small"></span>
              {:else}
                <Save size={15} strokeWidth={2} />
              {/if}
            </button>
            <button class="icon-btn" onclick={exportDoc} title="Export .md">
              <Download size={15} strokeWidth={2} />
            </button>
            {#if selectedDoc}
              <button class="icon-btn danger" onclick={() => selectedDoc && confirmDelete(selectedDoc)} title="Delete">
                <Trash2 size={15} strokeWidth={2} />
              </button>
            {/if}
          </div>
        </div>

        <!-- Editor / Preview area -->
        <div class="content-area" class:split-mode={viewMode === 'split'}>
          {#if viewMode === 'edit' || viewMode === 'split'}
            <div class="editor-pane">
              <div class="editor-container" bind:this={editorContainer} use:editorMount></div>
            </div>
          {/if}
          {#if viewMode === 'preview' || viewMode === 'split'}
            <div class="preview-pane">
              {#if viewMode === 'split'}
                <div class="pane-label">Preview</div>
              {/if}
              <div class="markdown-body" bind:this={previewContainer} onclick={(e) => handlePreviewClick(e)}>
            {#if editorContent.trim()}
              {@html renderedHtml}
            {:else}
              <p class="preview-empty">Nothing to preview</p>
            {/if}
          </div>
            </div>
          {/if}
        </div>
      {:else}
        <div class="empty-state">
          <div class="empty-icon">📄</div>
          <p class="empty-title">Select a document to view</p>
          <p class="empty-sub">or create a new one with <strong>+ New</strong></p>
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
  {@const kids = childrenCache.get(doc.id) ?? []}
  {@const isFolder = kids.length > 0}
  {@const expanded = expandedIds.has(doc.id)}
  <div class="tree-node">
    <div class="tree-row" class:active={selectedDoc?.id === doc.id} class:folder={isFolder}>
      <button
        class="tree-toggle"
        class:has-children={isFolder}
        class:expanded={expanded}
        onclick={(e: MouseEvent) => { e.stopPropagation(); if (isFolder) toggleNode(doc); }}
        tabindex={isFolder ? 0 : -1}
        aria-label={isFolder ? 'Toggle children' : ''}
      >
        {#if isFolder}
          <ChevronDown size={14} strokeWidth={2} class="chevron-icon" />
        {/if}
      </button>
      <button class="tree-label" onclick={() => selectDoc(doc)}>
        {#if isFolder}
          {#if expanded}
            <FolderOpen size={13} strokeWidth={1.75} class="tree-doc-icon" />
          {:else}
            <Folder size={13} strokeWidth={1.75} class="tree-doc-icon" />
          {/if}
        {:else}
          <FileText size={13} strokeWidth={1.75} class="tree-doc-icon" />
        {/if}
        <span class="tree-title" title={doc.title}>{doc.title || doc.slug}</span>
        {#if isFolder}
          <span class="tree-count">{kids.length}</span>
        {/if}
      </button>
    </div>
    {#if expanded && kids.length > 0}
      <div class="tree-children" transition:slide={{ duration: 180 }}>
        {#each kids as child (child.id)}
          {@render treeNode(child)}
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

<style>
  /* ═══════════════════════════════════════════════════════════════════
     Layout
     ═══════════════════════════════════════════════════════════════════ */
  .docs-view {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .layout { flex: 1; display: flex; overflow: hidden; min-height: 0; }

  /* ═══════════════════════════════════════════════════════════════════
     Header
     ═══════════════════════════════════════════════════════════════════ */
  .docs-header {
    padding: 12px 20px;
    display: flex;
    align-items: center;
    gap: 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  h2 { font-size: 1rem; font-weight: 600; margin: 0; }
  .count { font-size: 0.75rem; color: var(--text-muted); }
  .header-actions { margin-left: auto; }

  /* ═══════════════════════════════════════════════════════════════════
     Error Bar
     ═══════════════════════════════════════════════════════════════════ */
  .error-bar {
    padding: 8px 20px;
    background: rgba(243, 139, 168, 0.1);
    border-bottom: 1px solid rgba(243, 139, 168, 0.25);
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
    animation: slideDown 0.15s ease;
  }
  .success-bar {
    padding: 8px 20px;
    background: rgba(166, 227, 161, 0.1);
    border-bottom: 1px solid rgba(166, 227, 161, 0.25);
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
    animation: slideDown 0.15s ease;
  }
  .version-banner {
    padding: 10px 20px;
    background: rgba(245, 208, 135, 0.12);
    border-bottom: 1px solid rgba(245, 208, 135, 0.3);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-shrink: 0;
  }
  .vb-text { font-size: 0.85rem; color: var(--text-secondary); }
  .vb-text strong { color: var(--yellow); }
  .vb-btn {
    padding: 6px 14px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: 6px;
    font-size: 0.82rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }
  .vb-btn:hover:not(:disabled) { opacity: 0.85; }
  .vb-btn:disabled { opacity: 0.6; cursor: not-allowed; }
  @keyframes slideDown { from { opacity: 0; transform: translateY(-4px); } }
  .error-text {
    color: #f38ba8;
    font-size: 0.8rem;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .success-text {
    color: #a6e3a1;
    font-size: 0.8rem;
    font-weight: 500;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dismiss-btn {
    background: none;
    border: none;
    color: #f38ba8;
    cursor: pointer;
    padding: 0 2px;
    flex-shrink: 0;
  }
  .dismiss-btn.success-dismiss { color: #a6e3a1; }

  /* ═══════════════════════════════════════════════════════════════════
     Left Panel — Tree
     ═══════════════════════════════════════════════════════════════════ */
  .doc-tree-panel {
    width: 280px;
    min-width: 200px;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--border);
    overflow: hidden;
    flex-shrink: 0;
  }

  .search-bar {
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 6px;
    position: relative;
  }
  .search-icon {
    color: var(--text-muted);
    flex-shrink: 0;
  }
  .search-bar input {
    flex: 1;
    padding: 5px 8px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font-size: 0.8rem;
    min-width: 0;
  }
  .search-bar input:focus { outline: none; border-color: var(--accent); }
  .search-clear {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0 2px;
    font-size: 0.8rem;
    flex-shrink: 0;
  }
  .search-clear:hover { color: var(--text-primary); }

  .doc-tree { flex: 1; overflow-y: auto; padding: 4px 0; }

  .tree-node { user-select: none; }
  .tree-children {
    padding-left: 16px;
    margin-left: 10px;
    border-left: 1px solid var(--border);
  }

  .tree-row {
    display: flex;
    align-items: center;
    gap: 0;
    padding: 0 6px 0 0;
    border-left: 2px solid transparent;
    transition: background 0.12s, border-color 0.12s;
    min-height: 28px;
  }
  .tree-row:hover { background: var(--bg-hover); }
  .tree-row.active {
    background: var(--bg-hover);
    border-left-color: var(--accent);
  }
  .tree-row.active .tree-title { color: var(--accent); }
  .tree-row.active .tree-doc-icon { color: var(--accent); }
  .tree-row.folder .tree-doc-icon { color: var(--accent); }
  .tree-row.folder.active .tree-doc-icon { color: var(--accent); }
  .tree-count {
    font-size: 0.6rem;
    color: var(--text-muted);
    background: var(--bg-hover);
    padding: 1px 6px;
    border-radius: 8px;
    margin-left: 2px;
    flex-shrink: 0;
    line-height: 1.4;
  }

  .tree-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 28px;
    flex-shrink: 0;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: default;
    padding: 0;
  }
  .tree-toggle .chevron-icon {
    transition: transform 0.18s ease;
  }
  .tree-toggle:not(.expanded) .chevron-icon {
    transform: rotate(-90deg);
  }
  .tree-toggle.has-children { cursor: pointer; }
  .tree-toggle.has-children:hover { color: var(--text-primary); }

  .tree-label {
    display: flex;
    align-items: center;
    gap: 5px;
    flex: 1;
    min-width: 0;
    padding: 4px 4px 4px 0;
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 0.78rem;
    cursor: pointer;
    text-align: left;
    line-height: 1.3;
  }
  .tree-label:hover { color: var(--text-primary); }
  .tree-doc-icon { color: var(--text-muted); flex-shrink: 0; }
  .tree-title {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .doc-snippet { font-size: 0.7rem; color: var(--text-muted); margin-top: 2px; line-height: 1.3; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .doc-score { font-size: 0.6rem; color: var(--accent); font-weight: 600; margin-top: 2px; }

  /* Search result cards */
  .doc-card {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 10px;
    width: 100%;
    background: transparent;
    border: none;
    border-left: 2px solid transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background 0.1s, border-color 0.1s;
    text-align: left;
  }
  .doc-card:hover { background: var(--bg-hover); transform: translateY(-1px); transition: transform 0.12s ease, background 0.12s; }
  .doc-card.active { background: var(--bg-hover); border-left-color: var(--accent); }
  .doc-card .doc-title { font-size: 0.8rem; font-weight: 500; color: var(--text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .doc-card .doc-slug { font-size: 0.65rem; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

  .search-results { overflow-y: auto; padding: 4px 0; }

  .msg {
    padding: 20px 12px;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.8rem;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }

  .empty-tree {
    padding: 32px 16px;
    text-align: center;
    color: var(--text-muted);
  }
  .empty-tree-icon { font-size: 2rem; opacity: 0.25; margin-bottom: 8px; }
  .empty-tree p { margin: 4px 0; font-size: 0.8rem; }

  /* ═══════════════════════════════════════════════════════════════════
     Right Panel — Editor/Preview
     ═══════════════════════════════════════════════════════════════════ */
  .editor-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }

  /* ── Top bar: breadcrumb + mode toggle ── */
  .top-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    min-height: 32px;
    gap: 8px;
  }

  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 2px;
    font-size: 0.72rem;
    color: var(--text-muted);
    overflow-x: auto;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
    padding: 5px 0;
  }
  .crumb-link { background: none; border: none; color: var(--text-muted); cursor: pointer; padding: 2px 3px; font-size: 0.72rem; }
  .crumb-link:hover { color: var(--accent); }
  .crumb-sep { color: var(--text-muted); opacity: 0.4; }
  .crumb-current { color: var(--text-primary); font-weight: 600; padding: 2px 3px; }

  /* ── Mode toggle ── */
  .mode-toggle {
    display: flex;
    gap: 2px;
    background: var(--bg-primary);
    border-radius: var(--radius);
    padding: 2px;
    flex-shrink: 0;
  }
  .mode-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    background: transparent;
    border: none;
    border-radius: calc(var(--radius) - 2px);
    color: var(--text-muted);
    font-size: 0.7rem;
    cursor: pointer;
    transition: all 0.15s;
  }
  .mode-btn:hover { color: var(--text-primary); background: var(--bg-hover); }
  .mode-btn.active { color: var(--accent); background: var(--bg-tertiary); }
  .mode-btn svg { flex-shrink: 0; }

  /* ── Properties row (collapsible) ── */
  .props-row {
    display: flex;
    gap: 8px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    animation: slideDown 0.12s ease;
  }
  .prop-input {
    padding: 4px 8px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text-primary);
    font-size: 0.8rem;
    min-width: 0;
  }
  .prop-input:focus { outline: none; border-color: var(--accent); }
  .prop-input.title-input { flex: 1; font-weight: 600; }
  .prop-input.slug-input { width: 150px; font-family: var(--font-mono); font-size: 0.75rem; }
  .prop-input.tags-input { width: 130px; font-size: 0.75rem; }

  /* ── Meta bar ── */
  .meta-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    gap: 8px;
  }
  .meta-left {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 0.7rem;
    color: var(--text-muted);
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }
  .meta-item { white-space: nowrap; flex-shrink: 0; }
  .meta-dim { opacity: 0.6; }
  .ns-badge {
    padding: 1px 6px;
    background: var(--bg-tertiary);
    border-radius: 3px;
    color: var(--accent);
    font-size: 0.65rem;
    font-family: var(--font-mono);
  }
  .props-toggle {
    display: flex;
    align-items: center;
    gap: 3px;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.7rem;
    padding: 2px 4px;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .props-toggle:hover { color: var(--text-primary); background: var(--bg-hover); }

  .meta-actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  /* ── Icon buttons ── */
  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.12s;
  }
  .icon-btn:hover { background: var(--bg-hover); border-color: var(--border); color: var(--text-primary); }
  .icon-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .icon-btn.danger:hover { color: #f38ba8; background: rgba(243,139,168,0.1); }

  /* ── Content area ── */
  .content-area {
    flex: 1;
    display: flex;
    overflow: hidden;
  }
  .content-area.split-mode .editor-pane,
  .content-area.split-mode .preview-pane {
    flex: 1;
    min-width: 0;
  }
  .content-area.split-mode .editor-pane {
    border-right: 1px solid var(--border);
  }

  .editor-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-width: 0;
  }
  .editor-container { flex: 1; overflow: hidden; }
  .editor-container :global(.cm-editor) { height: 100%; }

  .preview-pane {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .pane-label {
    padding: 4px 16px;
    font-size: 0.65rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  /* ═══════════════════════════════════════════════════════════════════
     Markdown Preview Styles (Catppuccin Mocha)
     ═══════════════════════════════════════════════════════════════════ */
  .markdown-body {
    flex: 1;
    overflow-y: auto;
    padding: 24px 32px;
    color: var(--text-primary);
    line-height: 1.65;
    font-size: 0.9rem;
  }
  .markdown-body :global(h1) { color: #cba6f7; font-size: 1.6em; font-weight: 700; margin: 0 0 12px; padding-bottom: 6px; border-bottom: 1px solid var(--border); }
  .markdown-body :global(h2) { color: #cba6f7; font-size: 1.3em; font-weight: 600; margin: 24px 0 8px; padding-bottom: 4px; border-bottom: 1px solid rgba(49,50,68,0.5); }
  .markdown-body :global(h3) { color: #cba6f7; font-size: 1.1em; font-weight: 600; margin: 20px 0 6px; }
  .markdown-body :global(h4), .markdown-body :global(h5), .markdown-body :global(h6) { color: #cba6f7; font-weight: 600; margin: 16px 0 4px; }
  .markdown-body :global(p) { margin: 0 0 10px; }
  .markdown-body :global(a) { color: #89b4fa; text-decoration: none; }
  .markdown-body :global(a:hover) { text-decoration: underline; }
  .markdown-body :global(a[data-internal]) { color: var(--mauve); cursor: pointer; }
  .markdown-body :global(a[data-internal]:hover) { text-decoration: underline; }
  .markdown-body :global(strong) { color: #fab387; font-weight: 600; }
  .markdown-body :global(em) { color: #f9e2af; }
  .markdown-body :global(code) {
    color: #a6e3a1;
    background: rgba(30,30,46,0.8);
    padding: 2px 5px;
    border-radius: 3px;
    font-family: var(--font-mono);
    font-size: 0.85em;
  }
  .markdown-body :global(pre) {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 18px;
    overflow-x: auto;
    margin: 12px 0;
  }
  .markdown-body :global(pre code) {
    background: transparent;
    padding: 0;
    font-size: 0.82em;
    line-height: 1.5;
  }
  .markdown-body :global(blockquote) {
    border-left: 3px solid var(--accent);
    margin: 12px 0;
    padding: 4px 16px;
    color: var(--text-secondary);
    background: rgba(137,180,250,0.04);
    border-radius: 0 var(--radius) var(--radius) 0;
  }
  .markdown-body :global(ul), .markdown-body :global(ol) { padding-left: 24px; margin: 8px 0; }
  .markdown-body :global(li) { margin: 3px 0; }
  .markdown-body :global(li::marker) { color: var(--text-muted); }
  .markdown-body :global(hr) { border: none; border-top: 1px solid var(--border); margin: 20px 0; }
  .markdown-body :global(table) {
    width: 100%;
    border-collapse: collapse;
    margin: 12px 0;
    font-size: 0.85rem;
  }
  .markdown-body :global(th), .markdown-body :global(td) {
    padding: 6px 12px;
    border: 1px solid var(--border);
    text-align: left;
  }
  .markdown-body :global(th) { background: var(--bg-tertiary); font-weight: 600; color: var(--text-primary); }
  .markdown-body :global(td) { color: var(--text-secondary); }
  .markdown-body :global(img) { max-width: 100%; border-radius: var(--radius); }
  .preview-empty { color: var(--text-muted); font-style: italic; }

  /* ═══════════════════════════════════════════════════════════════════
     Shared
     ═══════════════════════════════════════════════════════════════════ */
  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 5px 12px;
    background: var(--bg-hover);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: 0.78rem;
    cursor: pointer;
    transition: background 0.12s, border-color 0.12s;
    white-space: nowrap;
  }
  .action-btn:hover { background: var(--bg-tertiary); border-color: var(--text-muted); }
  .action-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .action-btn.danger { color: #f38ba8; border-color: rgba(243,139,168,0.3); }
  .action-btn.danger:hover { background: rgba(243,139,168,0.12); }
  .action-btn.small { padding: 4px 10px; font-size: 0.72rem; }
  .new-btn { background: var(--accent); color: var(--bg-primary); border-color: var(--accent); font-weight: 600; }
  .new-btn:hover { opacity: 0.85; border-color: var(--accent); }
  .btn-icon { font-size: 1rem; font-weight: 700; }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    gap: 4px;
  }
  .empty-icon { font-size: 3rem; opacity: 0.2; }
  .empty-title { font-size: 0.9rem; margin: 8px 0 0; }
  .empty-sub { font-size: 0.75rem; opacity: 0.7; }

  /* ── Spinner ── */
  .spinner {
    display: inline-block;
    width: 12px;
    height: 12px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }
  .spinner.small { width: 10px; height: 10px; border-width: 1.5px; }
  @keyframes spin { to { transform: rotate(360deg); } }

  /* ── Dialog ── */
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
  .confirm-dialog p { margin: 0; font-size: 0.85rem; }
  .confirm-dialog .sub { font-size: 0.75rem; color: var(--text-muted); margin-top: 4px; }
  .confirm-actions { display: flex; gap: 8px; margin-top: 16px; justify-content: flex-end; }
</style>
