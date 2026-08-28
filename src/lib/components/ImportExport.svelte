<script lang="ts">
  import { system, utekeExport, utekeImport } from '../ts/ipc';
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';
  import { isWebMode } from '../ts/transport';
  import { parseJsonl, type JsonlPreview } from '../utils/jsonl';

  interface Props {
    namespaces: string[];
    onimported?: () => void;
  }

  let { namespaces = [], onimported }: Props = $props();

  // Mode
  let mode = $state<'export' | 'import'>('export');

  // Export state
  let exportFormat = $state<'json' | 'jsonl' | 'markdown' | 'csv'>('json');
  let exportNamespace = $state<string | null>(null);
  let exporting = $state(false);

  // Import state
  let importStep = $state<'pick' | 'preview' | 'done'>('pick');
  let importFileName = $state<string | null>(null);
  let importFileData = $state<string | null>(null);
  let importFormat = $state<'json' | 'markdown' | 'jsonl'>('json');
  let jsonlPreview = $state<JsonlPreview | null>(null);
  let jsonlTargetNs = $state<string>('');
  let importResult = $state<{ imported: number; skipped: number } | null>(null);
  let importPreview = $state<{
    format: string;
    memories: number;
    edges: number;
    rooms: number;
    namespaces: string[];
    tags?: string[];
  } | null>(null);
  let importing = $state(false);
  let importCount = $state<number | null>(null);
  let errorMsg = $state<string | null>(null);

  function reset() {
    mode = 'export';
    exportFormat = 'json';
    exportNamespace = null;
    exporting = false;
    importStep = 'pick';
    importFileName = null;
    importFileData = null;
    importFormat = 'json';
    jsonlPreview = null;
    jsonlTargetNs = '';
    importResult = null;
    importing = false;
    importCount = null;
    errorMsg = null;
  }

  // ─── Export ──────────────────────────────────────────────────────

  async function handleExport() {
    exporting = true;
    errorMsg = null;
    try {
      const ext =
        exportFormat === 'json' ? 'json'
        : exportFormat === 'csv' ? 'csv'
        : exportFormat === 'jsonl' ? 'jsonl'
        : 'md';
      const name = exportNamespace
        ? `corin-export-${exportNamespace}.${ext}`
        : `corin-export.${ext}`;

      // Desktop: native save dialog first — the chosen path is the write
      // target. Web mode: no dialog, straight to a Blob download.
      let filePath: string | null = null;
      if (!isWebMode) {
        filePath = await save({
          defaultPath: name,
          filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
        });

        if (!filePath) {
          exporting = false;
          return;
        }
      }

      // JSONL is the server-native format (GET /export); the other formats
      // are CorIn's own export engines (system.export_data).
      const data = exportFormat === 'jsonl'
        ? await utekeExport(exportNamespace ?? undefined)
        : await system.exportData(exportFormat, exportNamespace);

      if (filePath) {
        await writeTextFile(filePath, data);
      } else {
        const mime =
          exportFormat === 'json' ? 'application/json'
          : exportFormat === 'csv' ? 'text/csv'
          : exportFormat === 'jsonl' ? 'application/x-ndjson'
          : 'text/markdown';
        const blob = new Blob([data], { type: mime });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = name;
        a.click();
        URL.revokeObjectURL(url);
      }
    } catch (e: any) {
      errorMsg = e.toString();
    } finally {
      exporting = false;
    }
  }

  // ─── Import ──────────────────────────────────────────────────────

  async function handlePickFile() {
    errorMsg = null;
    // Web mode: hidden <input type="file"> instead of the native dialog.
    if (isWebMode) {
      fileInput?.click();
      return;
    }
    try {
      const filePath = await open({
        multiple: false,
        filters: [
          { name: 'CorIn Export', extensions: ['json'] },
          { name: 'JSONL', extensions: ['jsonl'] },
          { name: 'Markdown', extensions: ['md'] },
        ],
      });

      if (!filePath) return;

      importFileName = filePath.split('/').pop() || filePath.split('\\').pop() || 'unknown';
      importFileData = await readTextFile(filePath);

      // Detect format from extension
      importFormat = importFileName.endsWith('.md') ? 'markdown'
        : importFileName.endsWith('.jsonl') ? 'jsonl'
        : 'json';

      if (importFormat === 'jsonl') {
        jsonlPreview = parseJsonl(importFileData);
      } else {
        importPreview = await system.importPreview(importFormat, importFileData);
      }
      importStep = 'preview';
    } catch (e: any) {
      errorMsg = e.toString();
    }
  }

  /** Web-mode file input change handler. */
  async function handleWebFile(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    input.value = ''; // allow re-picking the same file
    if (!file) return;
    errorMsg = null;
    try {
      importFileName = file.name;
      importFileData = await file.text();
      importFormat = importFileName.endsWith('.md') ? 'markdown'
        : importFileName.endsWith('.jsonl') ? 'jsonl'
        : 'json';
      if (importFormat === 'jsonl') {
        jsonlPreview = parseJsonl(importFileData);
      } else {
        importPreview = await system.importPreview(importFormat, importFileData);
      }
      importStep = 'preview';
    } catch (e: any) {
      errorMsg = e.toString();
    }
  }

  async function handleImport() {
    if (!importFileData) return;
    importing = true;
    errorMsg = null;
    try {
      if (importFormat === 'jsonl') {
        importResult = await utekeImport(importFileData, jsonlTargetNs.trim() || undefined);
        importStep = 'done';
        onimported?.();
      } else {
        importCount = await system.importData(importFormat, importFileData);
        importStep = 'done';
        onimported?.();
      }
    } catch (e: any) {
      errorMsg = e.toString();
    } finally {
      importing = false;
    }
  }

  const formatInfo: Record<string, string> = {
    json: 'Full bundle: memories, edges, rooms. Best for backups and migration.',
    jsonl: 'Server-native JSONL (GET /export). Round-trips through POST /import — duplicates are skipped.',
    markdown: 'Per-memory .md files with Obsidian-compatible YAML frontmatter.',
    csv: 'Flat table export. Compatible with spreadsheets and data tools.',
  };

  // Web-mode hidden file input
  let fileInput = $state<HTMLInputElement | null>(null);
</script>

<input
  type="file"
  accept=".json,.md"
  bind:this={fileInput}
  onchange={handleWebFile}
  hidden
/>

<div class="import-export">
  <div class="mode-tabs">
    <button class:active={mode === 'export'} onclick={() => { reset(); mode = 'export'; }}>Export</button>
    <button class:active={mode === 'import'} onclick={() => { reset(); mode = 'import'; }}>Import</button>
  </div>

  {#if errorMsg}
    <div class="error-msg">{errorMsg}</div>
  {/if}

  <!-- Export -->
  {#if mode === 'export'}
    <div class="section">
      <h3>Format</h3>
      <div class="format-grid">
        {#each [
          { key: 'json' as const, label: 'JSON', desc: formatInfo.json },
          { key: 'jsonl' as const, label: 'JSONL (server)', desc: formatInfo.jsonl },
          { key: 'markdown' as const, label: 'Markdown', desc: formatInfo.markdown },
          { key: 'csv' as const, label: 'CSV', desc: formatInfo.csv },
        ] as fmt}
          <button
            class="format-card"
            class:active={exportFormat === fmt.key}
            onclick={() => exportFormat = fmt.key}
          >
            <span class="format-label">{fmt.label}</span>
            <span class="format-desc">{fmt.desc}</span>
          </button>
        {/each}
      </div>
    </div>

    <div class="section">
      <h3>Namespace</h3>
      <select bind:value={exportNamespace}>
        <option value="">All namespaces</option>
        {#each namespaces as ns}
          <option value={ns}>{ns}</option>
        {/each}
      </select>
    </div>

    <button class="primary-btn" onclick={handleExport} disabled={exporting}>
      {exporting ? 'Exporting...' : '↓ Export'}
    </button>
  {/if}

  <!-- Import -->
  {#if mode === 'import'}
    {#if importStep === 'pick'}
      <div class="section">
        <h3>Import from file</h3>
        <p class="hint">Supported formats: CorIn JSON export (.json), server JSONL (.jsonl), Obsidian-compatible Markdown (.md)</p>
        <button class="primary-btn" onclick={handlePickFile}>
          Pick File...
        </button>
      </div>
    {:else if importStep === 'preview' && importFormat === 'jsonl' && jsonlPreview}
      <div class="section">
        <h3>Preview: {importFileName}</h3>
        <div class="preview-grid">
          <div class="preview-item">
            <span class="preview-val">{jsonlPreview.count}</span>
            <span class="preview-label">Entries</span>
          </div>
          <div class="preview-item">
            <span class="preview-val">{jsonlPreview.malformed}</span>
            <span class="preview-label">Malformed</span>
          </div>
        </div>
        {#if jsonlPreview.first5.length > 0}
          <div class="jsonl-samples">
            {#each jsonlPreview.first5 as entry}
              <div class="jsonl-sample">{entry.content.slice(0, 90)}{entry.content.length > 90 ? '…' : ''}</div>
            {/each}
          </div>
        {/if}
        <div class="section">
          <h3>Target namespace (optional)</h3>
          <select bind:value={jsonlTargetNs}>
            <option value="">Keep each entry's own namespace</option>
            {#each namespaces as ns}
              <option value={ns}>Override → {ns}</option>
            {/each}
          </select>
        </div>
        <div class="preview-actions">
          <button class="secondary-btn" onclick={() => importStep = 'pick'}>
            Back
          </button>
          <button class="primary-btn" onclick={handleImport} disabled={importing || jsonlPreview.count === 0}>
            {importing ? 'Importing...' : `Import ${jsonlPreview.count} entries`}
          </button>
        </div>
      </div>
    {:else if importStep === 'preview' && importPreview}
      <div class="section">
        <h3>Preview: {importFileName}</h3>
        <div class="preview-grid">
          <div class="preview-item">
            <span class="preview-val">{importPreview.memories}</span>
            <span class="preview-label">Memories</span>
          </div>
          <div class="preview-item">
            <span class="preview-val">{importPreview.edges}</span>
            <span class="preview-label">Edges</span>
          </div>
          <div class="preview-item">
            <span class="preview-val">{importPreview.rooms}</span>
            <span class="preview-label">Rooms</span>
          </div>
        </div>
        {#if importPreview.namespaces.length > 0}
          <div class="preview-meta">
            <span class="meta-label">Namespaces:</span>
            <span class="meta-tags">{importPreview.namespaces.join(', ')}</span>
          </div>
        {/if}
        {#if importPreview.tags && importPreview.tags.length > 0}
          <div class="preview-meta">
            <span class="meta-label">Tags:</span>
            <span class="meta-tags">{importPreview.tags.join(', ')}</span>
          </div>
        {/if}
        <div class="preview-actions">
          <button class="secondary-btn" onclick={() => importStep = 'pick'}>
            Back
          </button>
          <button class="primary-btn" onclick={handleImport} disabled={importing}>
            {importing ? 'Importing...' : 'Import'}
          </button>
        </div>
      </div>
    {:else if importStep === 'done'}
      <div class="section">
        <h3>Import Complete</h3>
        {#if importResult}
          <p class="success-msg">
            Imported {importResult.imported} new memor{importResult.imported === 1 ? 'y' : 'ies'}
            {#if importResult.skipped > 0}— {importResult.skipped} malformed line{importResult.skipped === 1 ? '' : 's'} skipped{/if}.
            Exact duplicates are silently deduplicated by the server.
          </p>
        {:else}
          <p class="success-msg">
            Successfully imported {importCount} memories.
          </p>
        {/if}
        <button class="secondary-btn" onclick={reset}>
          Done
        </button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .import-export {
    padding: 4px 0;
  }

  .mode-tabs {
    display: flex;
    gap: 0;
    margin-bottom: 16px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .mode-tabs button {
    flex: 1;
    padding: 8px 16px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 0.85rem;
    cursor: pointer;
    transition: background 0.1s;
  }

  .mode-tabs button.active {
    background: var(--accent);
    color: var(--bg-primary);
    font-weight: 600;
  }

  .mode-tabs button:not(.active):hover {
    background: var(--bg-hover);
  }

  .section {
    margin-bottom: 16px;
  }

  .section h3 {
    font-size: 0.85rem;
    color: var(--text-primary);
    margin-bottom: 8px;
  }

  .format-grid {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .format-card {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 12px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
    transition: border-color 0.15s;
  }

  .format-card:hover {
    border-color: var(--text-muted);
  }

  .format-card.active {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }

  .format-label {
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .format-desc {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  select {
    width: 100%;
    padding: 8px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-primary);
    font-size: 0.85rem;
  }

  .hint {
    font-size: 0.78rem;
    color: var(--text-muted);
    margin-bottom: 12px;
  }

  .primary-btn {
    width: 100%;
    padding: 10px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .primary-btn:hover:not(:disabled) {
    opacity: 0.85;
  }

  .primary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .secondary-btn {
    padding: 8px 16px;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    cursor: pointer;
    transition: background 0.15s;
  }

  .secondary-btn:hover {
    background: var(--bg-hover);
  }

  .preview-grid {
    display: flex;
    gap: 12px;
    margin-bottom: 12px;
  }

  .preview-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    flex: 1;
    padding: 12px 8px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }

  .preview-val {
    font-size: 1.2rem;
    font-weight: 700;
    color: var(--accent);
  }

  .preview-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .preview-meta {
    display: flex;
    gap: 8px;
    font-size: 0.78rem;
    margin-bottom: 4px;
  }

  .meta-label {
    color: var(--text-muted);
  }

  .meta-tags {
    color: var(--text-secondary);
  }

  .jsonl-samples {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 12px;
    padding: 8px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }

  .jsonl-sample {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .preview-actions {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }

  .preview-actions .secondary-btn {
    flex: 1;
  }

  .preview-actions .primary-btn {
    flex: 2;
  }

  .error-msg {
    padding: 8px 12px;
    background: var(--color-red-bg);
    color: var(--red);
    border: 1px solid var(--color-red-line);
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    margin-bottom: 12px;
  }

  .success-msg {
    font-size: 0.85rem;
    color: var(--green);
    margin-bottom: 12px;
  }
</style>
