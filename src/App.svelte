<script lang="ts">
  import './app.css';
  import { onMount } from 'svelte';
  import { getUiStore } from './lib/stores/ui.svelte';
  import { system } from './lib/ts/ipc';

  const ui = getUiStore();

  onMount(async () => {
    // Check if we have a persisted data dir
    // Phase 1: user must pick a data directory on first launch
  });
</script>

<div class="app">
  {#if !ui.dataDirInitialized}
    <div class="welcome-screen">
      <div class="welcome-content">
        <h1>Codecora Hub</h1>
        <p>Desktop knowledge workstation powered by Uteke</p>
        <button class="primary-btn" onclick={async () => {
          try {
            const dir = await system.openDataDir();
            ui.dataDir = dir;
            ui.dataDirInitialized = true;
          } catch (e) {
            console.error('Failed to open data dir:', e);
          }
        }}>
          Open Data Directory
        </button>
        <p class="hint">Select or create a folder for your knowledge base.</p>
      </div>
    </div>
  {:else}
    <!-- Main app layout will go here -->
    <div class="main-layout">
      <div class="placeholder">
        <p>📊 Dashboard coming soon</p>
      </div>
    </div>
  {/if}
</div>

<style>
  .app {
    width: 100%;
    height: 100vh;
    display: flex;
    background: var(--bg-primary, #0e0e12);
    color: var(--text-primary, #cdd6f4);
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
  }

  .welcome-screen {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .welcome-content {
    text-align: center;
    max-width: 400px;
  }

  .welcome-content h1 {
    font-size: 2rem;
    font-weight: 700;
    margin-bottom: 0.5rem;
    color: var(--accent, #89b4fa);
  }

  .welcome-content p {
    color: var(--text-muted, #6c7086);
    margin-bottom: 1.5rem;
  }

  .primary-btn {
    padding: 10px 24px;
    background: var(--accent, #89b4fa);
    color: var(--bg-primary, #0e0e12);
    border: none;
    border-radius: 6px;
    font-size: 0.95rem;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .primary-btn:hover {
    opacity: 0.85;
  }

  .hint {
    font-size: 0.8rem;
    margin-top: 0.75rem !important;
  }

  .main-layout {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .placeholder {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted, #6c7086);
    font-size: 1.2rem;
  }
</style>
