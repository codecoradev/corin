<script lang="ts">
  import { system } from '../ts/ipc';
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { readTextFile } from '@tauri-apps/plugin-fs';

  interface Props {
    namespaces: string[];
    onimported?: () => void;
  }

  let { namespaces = [], onimported }: Props = $props();

  // Mode
  let mode = $state<'export' | 'import'>('export');

  // Export state
  let exportFormat = $state<'json' | 'markdown' | 'csv'>('json');
  let exportNamespace = $state<string | null>(null);
  let exporting = $state(false);

  // Import state
  let importStep = $state<'pick' | 'preview' | 'done'>('pick');
  let importFileName = $state<string | null>(null);
  let importFileData = $state<string | null>(null);
  let importFormat = $state<'json' | 'markdown'>('json');
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
    importPreview = null;
    importing = false;
    importCount = null;
    errorMsg = null;
  }

  // ─── Export ──────────────────────────────────────────────────────

  async function handleExport() {
    exporting = true;
    errorMsg = null;
    try {
      const ext = exportFormat === 'json' ? 'json' : exportFormat === 'csv' ? 'csv' : 'md';
      const name = exportNamespace
        ? `corin-export-${exportNamespace}.${ext}`
        : `corin-export.${ext}`;

      const filePath = await save({
        defaultPath: name,
        filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
      });

      if (!filePath) {
        exporting = false;
        return;
      }

      const data = await system.exportData(exportFormat, exportNamespace);

      // Write via Tauri fs (use the shell plugin to write)
      // For simplicity we use the browser download approach as fallback
      const blob = new Blob([data], {
        type: exportFormat === 'json' ? 'application/json' : exportFormat === 'csv' ? 'text/csv' : 'text/markdown',
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = name;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e: any) {
      errorMsg = e.toString();
    } finally {
      exporting = false;
    }
  }

  // ─── Import ──────────────────────────────────────────────────────

  async function handlePickFile() {
    errorMsg = null;
    try {
      const filePath = await open({
        multiple: false,
        filters: [
          { name: 'CorIn Export', extensions: ['json'] },
          { name: 'Markdown', extensions: ['md'] },
        ],
      });

      if (!filePath) return;

      importFileName = filePath.split('/').pop() || filePath.split('\\').pop() || 'unknown';
      importFileData = await readTextFile(filePath);

      // Detect format from extension
      importFormat = importFileName.endsWith('.md') ? 'markdown' : 'json';

      // Preview
      importPreview = await system.importPreview(importFormat, importFileData);
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
      importCount = await system.importData(importFormat, importFileData);
      importStep = 'done';
      onimported?.();
    } catch (e: any) {
      errorMsg = e.toString();
    } finally {
      importing = false;
    }
  }

  const formatInfo: Record<string, string> = {
    json: 'Full bundle: memories, edges, rooms. Best for backups and migration.',
    markdown: 'Per-memory .md files with Obsidian-compatible YAML frontmatter.',
    csv: 'Flat table export. Compatible with spreadsheets and data tools.',
  };
</script>

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
          { key: 'json', label: 'JSON', desc: formatInfo.json },
          { key: 'markdown', label: 'Markdown', desc: formatInfo.markdown },
          { key: 'csv', label: 'CSV', desc: formatInfo.csv },
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
        <p class="hint">Supported formats: CorIn JSON export (.json), Obsidian-compatible Markdown (.md)</p>
        <button class="primary-btn" onclick={handlePickFile}>
          Pick File...
        </button>
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
        <p class="success-msg">
          Successfully imported {importCount} memories.
        </p>
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
    border-radius: 4px;
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
    border-radius: 4px;
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
    border-radius: 4px;
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
    border-radius: 4px;
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
    border-radius: 4px;
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
    border-radius: 4px;
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
    background: color-mix(in srgb, #f43f5e 10%, transparent);
    color: #f43f5e;
    border: 1px solid color-mix(in srgb, #f43f5e 30%, transparent);
    border-radius: 4px;
    font-size: 0.8rem;
    margin-bottom: 12px;
  }

  .success-msg {
    font-size: 0.85rem;
    color: var(--green);
    margin-bottom: 12px;
  }
</style>
