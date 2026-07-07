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
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';
  import {
    Search,
    SquarePen,
    Columns2,
    Eye,
    ChevronDown,
    Save,
    Download,
    Trash2,
    Plus,
  } from 'lucide-svelte';
  import { open as shellOpen } from '@tauri-apps/plugin-shell';

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
  interface Props {
    namespace: string | null;
  }

  let { namespace }: Props = $props();

  // ─── Editor / Preview mode ─────────────────────────────────────────
  type ViewMode = 'edit' | 'preview' | 'split';
  let viewMode = $state<ViewMode>('edit');

  // ─── State ─────────────────────────────────────────────────────────
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
  let errorTimeout: ReturnType<typeof setTimeout> | null = null;
  let editorView = $state<EditorView | null>(null);
  let showProps = $state(false);

  // ─── Bound DOM elements for scroll reset ───────────────────────────
  let editorContainer: HTMLElement | null = null;
  let previewContainer: HTMLElement | null = null;

  // ─── Markdown rendering ────────────────────────────────────────────
  let renderedHtml = $state('');

  // ─── Render markdown: GFM + breaks enabled (single \n → <br>) ───────
  // External links get target="_blank" for system browser handling.
  const renderer = new marked.Renderer();
  renderer.link = ({ href, title, text }: { href: string; title?: string | null; text: string }) => {
    const url = href ?? '';
    if (url.startsWith('http://') || url.startsWith('https://')) {
      return `<a href="${url}" target="_blank" rel="noopener noreferrer">${text}</a>`;
    }
    return `<a href="${url}">${text}</a>`;
  };

  function renderMarkdown(md: string) {
    try {
      // Pass options directly to parse() so they can't be reset by HMR
      const html = marked.parse(md, {
        renderer,
        gfm: true,
        breaks: true,
        async: false,
      }) as string;
      renderedHtml = DOMPurify.sanitize(html, {
        ADD_ATTR: ['target', 'rel'],
      });
    } catch {
      renderedHtml = DOMPurify.sanitize(md);
    }
  }

  // Re-render when content changes
  $effect(() => {
    if (viewMode !== 'edit') {
      renderMarkdown(editorContent);
    }
  });

  // ─── Show error with auto-dismiss ───────────────────────────────────
  function showError(msg: string) {
    error = msg;
    if (errorTimeout) clearTimeout(errorTimeout);
    errorTimeout = setTimeout(() => { error = ''; }, 8000);
  }

  // ─── Load root documents ──────────────────────────────────────────
  async function loadRootDocs() {
    loading = true;
    try {
      rootDocs = await docs.list({ roots_only: true, namespace: namespace ?? undefined });
      // Auto-expand root nodes that have children
      for (const doc of rootDocs) {
        if (doc.has_children) {
          expandedIds.add(doc.id);
          loadChildren(doc.id);
        }
      }
      if (rootDocs.some(d => d.has_children)) {
        expandedIds = expandedIds;
      }
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
      const children = await docs.list({ parent: docId, namespace: namespace ?? undefined });
      childrenCache.set(docId, children);
      childrenCache = childrenCache;
    } catch (e: any) {
      showError(`Failed to load children: ${e}`);
    }
  }

  // ─── Load all descendants (for selected doc's tree path) ─────────
  async function expandPathToDoc(docId: string) {
    // Load root children first
    for (const root of rootDocs) {
      if (root.has_children && !childrenCache.has(root.id)) {
        await loadChildren(root.id);
      }
    }
    // Recursively expand down the path to find and expand the target
    function searchLevel(entries: DocEntry[]): boolean {
      for (const entry of entries) {
        if (entry.id === docId) return true;
        if (entry.has_children) {
          if (childrenCache.has(entry.id)) {
            const kids = childrenCache.get(entry.id) ?? [];
            if (!expandedIds.has(entry.id)) {
              expandedIds.add(entry.id);
              expandedIds = expandedIds;
            }
            if (searchLevel(kids)) return true;
          }
        }
      }
      return false;
    }
    searchLevel(rootDocs);
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
      // Default to preview mode when opening an existing document
      viewMode = 'preview';
      // Auto-load and expand children of this document in the tree
      if (full.has_children) {
        expandedIds.add(full.id);
        expandedIds = expandedIds;
        await loadChildren(full.id);
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
      searchResults = await docs.search(searchQuery, { namespace: namespace ?? undefined, limit: 20 });
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
        // Existing document → update via /doc/update
        const updated = await docs.update({
          id: selectedDoc.id,
          title: editorTitle || editorSlug,
          content: editorContent,
          tags,
          namespace: selectedDoc.namespace ?? namespace ?? undefined,
        });
        selectedDoc = updated;
      } else {
        // New document → create via /doc/create
        // Parent is the currently-selected doc (if any), not its parent_id.
        const parent = selectedDoc?.id ?? undefined;
        const created = await docs.create(editorSlug, editorTitle || editorSlug, editorContent, {
          namespace: namespace ?? undefined,
          tags,
          parent,
        });
        selectedDoc = created;
        showNewDoc = false;
      }
      await loadRootDocs();
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
  function exportDoc() {
    if (selectedDoc) docs.exportFile(selectedDoc);
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

  // ─── Formatted date ──────────────────────────────────────────────
  function formatDate(iso: string | null | undefined): string {
    if (!iso) return '';
    try {
      return new Date(iso).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
    } catch {
      return iso ?? '';
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
  function getBreadcrumb(): string[] {
    if (!selectedDoc) return [];
    if (selectedDoc.path && selectedDoc.path.length > 0) {
      const parts = selectedDoc.path.split('/').filter(Boolean);
      return parts.length > 0 ? parts : [];
    }
    return [];
  }

  // ─── Word count & reading time ────────────────────────────────────
  function getWordCount(text: string): number {
    return text.trim() ? text.trim().split(/\s+/).length : 0;
  }

  function getReadingTime(words: number): string {
    const mins = Math.max(1, Math.ceil(words / 200));
    return words === 0 ? '' : `${mins} min read`;
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

  {#if error}
    <div class="error-bar">
      <span class="error-text">{error}</span>
      <button class="dismiss-btn" onclick={() => (error = '')}>✕</button>
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
              {#each getBreadcrumb() as part, i}
                <span class="crumb-sep">/</span>
                <button class="crumb-link">{part.slice(0, 8)}</button>
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

        <!-- Meta bar: version, date, namespace + actions -->
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
              {#if selectedDoc.namespace}
                <span class="ns-badge">{selectedDoc.namespace}</span>
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
        {doc.has_children ? (expandedIds.has(doc.id) ? '▾' : '▸') : '·'}
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
  .dismiss-btn {
    background: none;
    border: none;
    color: #f38ba8;
    cursor: pointer;
    padding: 0 2px;
    flex-shrink: 0;
  }

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

  .doc-tree { flex: 1; overflow-y: auto; padding: 2px 0; }

  .tree-node { user-select: none; }
  .tree-children { padding-left: 12px; }

  .tree-toggle {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    font-size: 0.6rem;
    color: var(--text-muted);
    flex-shrink: 0;
    background: none;
    border: none;
    cursor: default;
    padding: 0;
    line-height: 1;
  }
  .tree-toggle.has-children { cursor: pointer; }
  .tree-toggle:hover { color: var(--text-primary); }

  .doc-card {
    display: flex;
    align-items: flex-start;
    gap: 4px;
    padding: 5px 10px;
    width: 100%;
    background: transparent;
    border: none;
    border-left: 2px solid transparent;
    color: var(--text-secondary);
    font-size: 0.8rem;
    cursor: pointer;
    transition: background 0.1s, border-color 0.1s;
    text-align: left;
    line-height: 1.4;
  }
  .doc-card:hover { background: var(--bg-hover); }
  .doc-card.active { background: var(--bg-hover); color: var(--accent); border-left-color: var(--accent); }
  .doc-card.search-hit { padding: 8px 10px; }
  .doc-info { min-width: 0; }
  .doc-title { font-size: 0.8rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .doc-slug { font-size: 0.65rem; color: var(--text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .doc-snippet { font-size: 0.7rem; color: var(--text-muted); margin-top: 2px; line-height: 1.3; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .doc-score { font-size: 0.6rem; color: var(--accent); font-weight: 600; margin-top: 2px; }

  .search-results { overflow-y: auto; }

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
