<script lang="ts">
  import { updater } from '../../ts/ipc';
  import type { Update } from '@tauri-apps/plugin-updater';

  let checkingUpdates = $state(false);
  let installingUpdate = $state(false);
  let updateStatus = $state<string | null>(null);
  let pendingUpdate: Update | null = $state(null);

  async function checkForUpdates() {
    checkingUpdates = true;
    updateStatus = null;
    pendingUpdate = null;
    try {
      const update = await updater.check();
      if (update) {
        pendingUpdate = update;
        updateStatus = `Update available: v${update.version}`;
      } else {
        updateStatus = 'Up to date ✅';
      }
    } catch (e: unknown) {
      updateStatus = `Error: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      checkingUpdates = false;
    }
  }

  async function installUpdate() {
    if (!pendingUpdate) return;
    installingUpdate = true;
    updateStatus = 'Downloading and installing...';
    try {
      await pendingUpdate.downloadAndInstall();
      updateStatus = 'Update installed. Restarting...';
      await new Promise((r) => setTimeout(r, 1500));
      await import('@tauri-apps/plugin-process').then((m) => m.relaunch());
    } catch (e: unknown) {
      updateStatus = `Error: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      installingUpdate = false;
    }
  }
</script>

<section class="content-section">
  <h3>Updates</h3>
  <div class="update-section">
    <button class="data-btn" onclick={checkForUpdates} disabled={checkingUpdates}>
      {checkingUpdates ? 'Checking...' : '↻ Check for Updates'}
    </button>
    {#if updateStatus}
      <p class="update-msg" class:ok={updateStatus.startsWith('Up to date')} class:err={updateStatus.startsWith('Error')}>{updateStatus}</p>
    {/if}
    {#if pendingUpdate}
      <button class="install-btn" onclick={installUpdate} disabled={installingUpdate}>
        {installingUpdate ? 'Installing...' : '↓ Install Update'}
      </button>
    {/if}
  </div>
</section>

<style>
  h3 { font-size: 0.9rem; color: var(--text-secondary); margin: 0 0 14px; font-weight: 600; }
  .content-section { margin-bottom: 24px; }
  .update-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
  }
  .data-btn {
    padding: 8px 16px;
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 0.85rem;
  }
  .data-btn:hover { border-color: var(--accent); }
  .update-msg { font-size: 0.85rem; color: var(--text-secondary); }
  .update-msg.ok { color: var(--green); }
  .update-msg.err { color: var(--red); }
  .install-btn {
    padding: 8px 16px;
    background: var(--green);
    color: var(--bg-primary);
    border: none;
    border-radius: 4px;
    font-weight: 600;
    font-size: 0.85rem;
  }
  .install-btn:disabled { opacity: 0.5; }
</style>
