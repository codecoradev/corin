<script lang="ts">
  import { Spinner, toastStore } from '../ui';
  import { X } from 'lucide-svelte';
  import {
    memoryUpdate, roomRemember, utekeImport, utekeExport, utekeContext,
    roomDocList, roomDocAdd, roomDocRemove, docRoomList,
  } from '../ts/ipc';

  // Active tab
  let activeTab = $state<'memory' | 'room-remember' | 'import-export' | 'context' | 'room-docs'>('memory');

  // ── Memory Update state ──
  let memId = $state('');
  let memContent = $state('');
  let memTags = $state('');
  let memImportance = $state('');
  let memPinned = $state(false);
  let memType = $state('');
  let updatingMem = $state(false);
  let memResult = $state<Record<string, unknown> | null>(null);

  async function handleMemoryUpdate() {
    if (!memId.trim()) { toastStore.error('Memory ID is required'); return; }
    updatingMem = true;
    memResult = null;
    try {
      const tags = memTags.trim() ? memTags.split(',').map(t => t.trim()).filter(Boolean) : undefined;
      const imp = memImportance.trim() ? parseFloat(memImportance) : undefined;
      const result = await memoryUpdate({
        id: memId.trim(),
        content: memContent.trim() || undefined,
        tags,
        importance: imp,
        pinned: memPinned || undefined,
        memory_type: memType.trim() || undefined,
      });
      memResult = result;
      toastStore.success('Memory updated');
    } catch (e) {
      toastStore.error(`Update failed: ${e}`);
    } finally {
      updatingMem = false;
    }
  }

  // ── Room Remember state ──
  let rrRoomId = $state('');
  let rrContent = $state('');
  let rrTags = $state('');
  let rrNamespace = $state('');
  let rrType = $state('');
  let rrAuthor = $state('');
  let remembering = $state(false);
  let rrResult = $state<Record<string, unknown> | null>(null);

  async function handleRoomRemember() {
    if (!rrRoomId.trim() || !rrContent.trim()) { toastStore.error('Room ID and content are required'); return; }
    remembering = true;
    rrResult = null;
    try {
      const tags = rrTags.trim() ? rrTags.split(',').map(t => t.trim()).filter(Boolean) : [];
      const result = await roomRemember({
        room_id: rrRoomId.trim(),
        content: rrContent.trim(),
        tags,
        namespace: rrNamespace.trim() || undefined,
        memory_type: rrType.trim() || undefined,
        author: rrAuthor.trim() || undefined,
      });
      rrResult = result;
      toastStore.success('Memory stored in room');
      rrContent = '';
      rrTags = '';
    } catch (e) {
      toastStore.error(`Remember failed: ${e}`);
    } finally {
      remembering = false;
    }
  }

  // ── Import/Export state ──
  let exportNs = $state('');
  let exportData = $state('');
  let exporting = $state(false);
  let importData = $state('');
  let importNs = $state('');
  let importing = $state(false);
  let importResult = $state<{ imported: number; skipped: number } | null>(null);

  async function handleExport() {
    exporting = true;
    exportData = '';
    try {
      const result = await utekeExport(exportNs.trim() || undefined);
      exportData = result;
      toastStore.success(`Exported ${result.split('\n').filter(Boolean).length} entries`);
    } catch (e) {
      toastStore.error(`Export failed: ${e}`);
    } finally {
      exporting = false;
    }
  }

  async function handleImport() {
    if (!importData.trim()) { toastStore.error('Paste JSONL data to import'); return; }
    importing = true;
    importResult = null;
    try {
      const result = await utekeImport(importData.trim(), importNs.trim() || undefined);
      importResult = result;
      toastStore.success(`Imported ${result.imported} entries (${result.skipped} skipped)`);
    } catch (e) {
      toastStore.error(`Import failed: ${e}`);
    } finally {
      importing = false;
    }
  }

  // ── Context state ──
  let ctxNs = $state('');
  let ctxResult = $state('');
  let buildingCtx = $state(false);

  async function handleContext() {
    buildingCtx = true;
    ctxResult = '';
    try {
      const result = await utekeContext(ctxNs.trim() || undefined);
      ctxResult = result;
      if (!result) toastStore.info('Context is empty');
    } catch (e) {
      toastStore.error(`Context failed: ${e}`);
    } finally {
      buildingCtx = false;
    }
  }

  // ── Room-Doc linking state ──
  let rdRoomId = $state('');
  let rdDocSlug = $state('');
  let rdLinkedDocs = $state<string[]>([]);
  let rdLinkedRooms = $state<string[]>([]);
  let loadingLinks = $state(false);

  async function handleListRoomDocs() {
    if (!rdRoomId.trim()) { toastStore.error('Room ID is required'); return; }
    loadingLinks = true;
    try {
      rdLinkedDocs = await roomDocList(rdRoomId.trim());
    } catch (e) {
      toastStore.error(`List failed: ${e}`);
    } finally {
      loadingLinks = false;
    }
  }

  async function handleListDocRooms() {
    if (!rdDocSlug.trim()) { toastStore.error('Document slug is required'); return; }
    loadingLinks = true;
    try {
      rdLinkedRooms = await docRoomList(rdDocSlug.trim());
    } catch (e) {
      toastStore.error(`List failed: ${e}`);
    } finally {
      loadingLinks = false;
    }
  }

  async function handleAddLink() {
    if (!rdRoomId.trim() || !rdDocSlug.trim()) { toastStore.error('Room ID and doc slug are required'); return; }
    try {
      await roomDocAdd(rdRoomId.trim(), rdDocSlug.trim());
      toastStore.success(`Linked "${rdDocSlug}" to room "${rdRoomId}"`);
      await handleListRoomDocs();
    } catch (e) {
      toastStore.error(`Link failed: ${e}`);
    }
  }

  async function handleRemoveLink(slug: string) {
    try {
      await roomDocRemove(rdRoomId.trim(), slug);
      toastStore.success(`Unlinked "${slug}"`);
      await handleListRoomDocs();
    } catch (e) {
      toastStore.error(`Unlink failed: ${e}`);
    }
  }

  const tabs = [
    { id: 'memory' as const, label: 'Update Memory' },
    { id: 'room-remember' as const, label: 'Room Remember' },
    { id: 'import-export' as const, label: 'Import / Export' },
    { id: 'context' as const, label: 'Context' },
    { id: 'room-docs' as const, label: 'Room ↔ Docs' },
  ];
</script>

<div class="tools-view">
  <div class="header">
    <h2>Uteke Tools</h2>
    <p class="subtitle">Direct access to Uteke server endpoints (#216, #231)</p>
  </div>

  <!-- Tab bar -->
  <div class="tab-bar">
    {#each tabs as tab}
      <button
        class="tab"
        class:active={activeTab === tab.id}
        onclick={() => activeTab = tab.id}
      >
        {tab.label}
      </button>
    {/each}
  </div>

  <!-- Memory Update -->
  {#if activeTab === 'memory'}
    <section class="card">
      <h3>Update Memory</h3>
      <p class="hint">PUT /memory — partial update, all fields optional except ID</p>
      <div class="form-grid">
        <label class="field">
          <span>Memory ID *</span>
          <input bind:value={memId} placeholder="mem_xxx" />
        </label>
        <label class="field full">
          <span>Content</span>
          <textarea bind:value={memContent} rows="3" placeholder="New content (optional)"></textarea>
        </label>
        <label class="field">
          <span>Tags (comma-separated)</span>
          <input bind:value={memTags} placeholder="tag1, tag2" />
        </label>
        <label class="field">
          <span>Importance (0.0–1.0)</span>
          <input bind:value={memImportance} placeholder="0.8" type="number" step="0.1" min="0" max="1" />
        </label>
        <label class="field">
          <span>Type</span>
          <input bind:value={memType} placeholder="fact" />
        </label>
        <label class="field checkbox">
          <input type="checkbox" bind:checked={memPinned} />
          <span>Pinned</span>
        </label>
      </div>
      <div class="actions">
        <button class="btn-primary" onclick={handleMemoryUpdate} disabled={updatingMem}>
          {#if updatingMem}<span class="spinner-inline"><Spinner size={14} /></span>{/if}
          Update
        </button>
      </div>
      {#if memResult}
        <pre class="result">{JSON.stringify(memResult, null, 2)}</pre>
      {/if}
    </section>

  <!-- Room Remember -->
  {:else if activeTab === 'room-remember'}
    <section class="card">
      <h3>Room Remember</h3>
      <p class="hint">POST /room/remember — store a memory directly into a room</p>
      <div class="form-grid">
        <label class="field">
          <span>Room ID *</span>
          <input bind:value={rrRoomId} placeholder="room_xxx" />
        </label>
        <label class="field full">
          <span>Content *</span>
          <textarea bind:value={rrContent} rows="3" placeholder="Memory content"></textarea>
        </label>
        <label class="field">
          <span>Tags (comma-separated)</span>
          <input bind:value={rrTags} placeholder="tag1, tag2" />
        </label>
        <label class="field">
          <span>Namespace</span>
          <input bind:value={rrNamespace} placeholder="default" />
        </label>
        <label class="field">
          <span>Type</span>
          <input bind:value={rrType} placeholder="fact" />
        </label>
        <label class="field">
          <span>Author</span>
          <input bind:value={rrAuthor} placeholder="agent-name" />
        </label>
      </div>
      <div class="actions">
        <button class="btn-primary" onclick={handleRoomRemember} disabled={remembering}>
          {#if remembering}<span class="spinner-inline"><Spinner size={14} /></span>{/if}
          Remember
        </button>
      </div>
      {#if rrResult}
        <pre class="result">{JSON.stringify(rrResult, null, 2)}</pre>
      {/if}
    </section>

  <!-- Import / Export -->
  {:else if activeTab === 'import-export'}
    <section class="card">
      <h3>Export</h3>
      <p class="hint">GET /export — export memories as JSONL</p>
      <div class="inline-form">
        <input bind:value={exportNs} placeholder="namespace (optional)" />
        <button class="btn-primary" onclick={handleExport} disabled={exporting}>
          {#if exporting}<span class="spinner-inline"><Spinner size={14} /></span>{/if}
          Export
        </button>
      </div>
      {#if exportData}
        <pre class="result scrollable">{exportData}</pre>
      {/if}
    </section>

    <section class="card">
      <h3>Import</h3>
      <p class="hint">POST /import — import JSONL data (one JSON object per line)</p>
      <div class="inline-form">
        <input bind:value={importNs} placeholder="namespace (optional)" />
      </div>
      <textarea bind:value={importData} rows="6" placeholder="One JSON object per line, e.g. content, tags, namespace fields"></textarea>
      <div class="actions">
        <button class="btn-primary" onclick={handleImport} disabled={importing}>
          {#if importing}<span class="spinner-inline"><Spinner size={14} /></span>{/if}
          Import
        </button>
      </div>
      {#if importResult}
        <div class="result-inline">
          <span class="badge-success">Imported: {importResult.imported}</span>
          <span class="badge-warn">Skipped: {importResult.skipped}</span>
        </div>
      {/if}
    </section>

  <!-- Context -->
  {:else if activeTab === 'context'}
    <section class="card">
      <h3>Build Context</h3>
      <p class="hint">POST /context — generate context summary from stored memories</p>
      <div class="inline-form">
        <input bind:value={ctxNs} placeholder="namespace (optional)" />
        <button class="btn-primary" onclick={handleContext} disabled={buildingCtx}>
          {#if buildingCtx}<span class="spinner-inline"><Spinner size={14} /></span>{/if}
          Build
        </button>
      </div>
      {#if ctxResult}
        <pre class="result">{ctxResult}</pre>
      {/if}
    </section>

  <!-- Room-Doc Linking -->
  {:else if activeTab === 'room-docs'}
    <section class="card">
      <h3>Room ↔ Document Linking</h3>
      <p class="hint">Link, unlink, and list document-room associations</p>
      <div class="form-grid">
        <label class="field">
          <span>Room ID</span>
          <input bind:value={rdRoomId} placeholder="room_xxx" />
        </label>
        <label class="field">
          <span>Doc Slug</span>
          <input bind:value={rdDocSlug} placeholder="my-doc" />
        </label>
      </div>
      <div class="actions">
        <button class="btn-primary" onclick={handleAddLink}>Link</button>
        <button class="btn-secondary" onclick={handleListRoomDocs} disabled={loadingLinks || !rdRoomId}>
          List Docs in Room
        </button>
        <button class="btn-secondary" onclick={handleListDocRooms} disabled={loadingLinks || !rdDocSlug}>
          List Rooms for Doc
        </button>
      </div>
      {#if loadingLinks}
        <div class="center-spinner"><Spinner size={20} /></div>
      {/if}
      {#if rdLinkedDocs.length > 0}
        <div class="link-list">
          <h4>Documents in "{rdRoomId}"</h4>
          <ul>
            {#each rdLinkedDocs as slug}
              <li>
                <span>{slug}</span>
                <button class="btn-icon" onclick={() => handleRemoveLink(slug)} title="Unlink document" aria-label="Unlink {slug}">
                  <X size={13} strokeWidth={2.5} />
                </button>
              </li>
            {/each}
          </ul>
        </div>
      {/if}
      {#if rdLinkedRooms.length > 0}
        <div class="link-list">
          <h4>Rooms linked to "{rdDocSlug}"</h4>
          <ul>
            {#each rdLinkedRooms as rid}
              <li><span>{rid}</span></li>
            {/each}
          </ul>
        </div>
      {/if}
    </section>
  {/if}
</div>

<style>
  .tools-view { padding: 1.5rem; max-width: 900px; margin: 0 auto; }
  .header { margin-bottom: 1.5rem; }
  .header h2 { margin: 0; font-size: 1.5rem; }
  .subtitle { color: var(--text-muted); font-size: 0.875rem; margin: 0.25rem 0 0; }

  .tab-bar { display: flex; gap: 0; border-bottom: 1px solid var(--border); margin-bottom: 1.5rem; overflow-x: auto; }
  .tab { padding: 0.5rem 1rem; background: none; border: none; border-bottom: 2px solid transparent; color: var(--text-muted); cursor: pointer; white-space: nowrap; font-size: 0.875rem; }
  .tab:hover { color: var(--text-primary); }
  .tab.active { color: var(--accent); border-bottom-color: var(--accent); }

  .card { background: var(--bg-tertiary); border-radius: var(--radius-lg); padding: 1.25rem; margin-bottom: 1rem; }
  .card h3 { margin: 0 0 0.25rem; font-size: 1.1rem; }
  .hint { color: var(--text-muted); font-size: 0.75rem; margin: 0 0 1rem; }

  .form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 0.75rem; }
  .field { display: flex; flex-direction: column; gap: 0.25rem; }
  .field.full { grid-column: 1 / -1; }
  .field.checkbox { flex-direction: row; align-items: center; gap: 0.5rem; }
  .field span { font-size: 0.75rem; color: var(--text-muted); }
  .field input, .field textarea {
    background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius-sm);
    padding: 0.5rem; color: var(--text-primary); font-size: 0.875rem;
  }
  .field input:focus, .field textarea:focus { outline: none; border-color: var(--accent); }

  .inline-form { display: flex; gap: 0.5rem; margin-bottom: 0.75rem; }
  .inline-form input { flex: 1; }

  .actions { margin-top: 0.75rem; display: flex; gap: 0.5rem; }

  .btn-primary, .btn-secondary {
    padding: 0.5rem 1rem; border-radius: var(--radius-sm); font-size: 0.875rem; cursor: pointer;
    border: 1px solid var(--border); display: flex; align-items: center; gap: 0.4rem;
  }
  .btn-primary { background: var(--accent); color: var(--bg-primary); border: none; }
  .btn-primary:disabled { opacity: 0.5; cursor: default; }
  .btn-secondary { background: var(--bg-tertiary); color: var(--text-primary); }
  .btn-secondary:disabled { opacity: 0.5; cursor: default; }

  .btn-icon { background: none; border: none; color: var(--red); cursor: pointer; padding: 0 0.25rem; font-size: 0.875rem; }

  .spinner-inline { display: inline-flex; align-items: center; }
  .center-spinner { display: flex; justify-content: center; padding: 1rem; }

  .result {
    margin-top: 0.75rem; background: var(--bg-secondary); border: 1px solid var(--border);
    border-radius: var(--radius-sm); padding: 0.75rem; font-size: 0.75rem; font-family: monospace;
    white-space: pre-wrap; word-break: break-word; max-height: 300px; overflow-y: auto;
  }
  .result.scrollable { max-height: 400px; }

  .result-inline { margin-top: 0.75rem; display: flex; gap: 0.5rem; }
  .badge-success { background: #2a4; color: #fff; padding: 0.25rem 0.5rem; border-radius: var(--radius-sm); font-size: 0.75rem; }
  .badge-warn { background: #a72; color: #fff; padding: 0.25rem 0.5rem; border-radius: var(--radius-sm); font-size: 0.75rem; }

  .link-list { margin-top: 1rem; }
  .link-list h4 { font-size: 0.875rem; margin: 0 0 0.5rem; }
  .link-list ul { list-style: none; padding: 0; margin: 0; }
  .link-list li { display: flex; justify-content: space-between; align-items: center; padding: 0.4rem 0.5rem; border-bottom: 1px solid var(--border); font-size: 0.875rem; }

  textarea { background: var(--bg-primary); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 0.5rem; color: var(--text-primary); font-size: 0.875rem; font-family: monospace; width: 100%; resize: vertical; }
</style>
