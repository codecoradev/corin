<script lang="ts">
  /**
   * Type backfill (#293 follow-up) — finds memories whose content_type is
   * empty (they render with no TYPE in the detail panel) and stamps them
   * to 'text' via PUT /memory. memory_type is server-defaulted to 'fact',
   * so content_type is the only gap users actually see.
   */
  import { memoryUpdate, uteke } from '../../ts/ipc';
  import { Button, Spinner, toastStore } from '../../ui';

  let scanning = $state(false);
  let applying = $state(false);
  let scanned = $state(false);
  let untyped = $state<{ id: string; namespace: string | null }[]>([]);

  async function scan() {
    scanning = true;
    try {
      const items = await uteke.list({ limit: 1000 }).catch(() => []);
      untyped = items
        .filter((m) => !m.content_type || m.content_type.trim() === '')
        .map((m) => ({ id: m.id, namespace: m.namespace }));
      scanned = true;
    } finally {
      scanning = false;
    }
  }

  async function apply() {
    applying = true;
    let ok = 0;
    let fail = 0;
    try {
      for (const item of untyped) {
        try {
          await memoryUpdate({ id: item.id, content_type: 'text' });
          ok++;
        } catch {
          fail++;
        }
      }
      toastStore.success(
        `Backfilled ${ok} memor${ok === 1 ? 'y' : 'ies'} to 'text'${fail ? ` — ${fail} failed` : ''}`,
      );
      await scan(); // re-scan to confirm the queue is empty
    } finally {
      applying = false;
    }
  }
</script>

<section class="content-section">
  <h3>Type Backfill</h3>
  <div class="bf-card">
    <p class="bf-desc">
      Older imports and agent writes can leave memories without a
      <code>content_type</code> — they show no TYPE in the detail panel. This
      stamps them to <code>'text'</code>.
    </p>

    {#if scanning}
      <p class="bf-status"><Spinner size={14} /> Scanning memories…</p>
    {:else if !scanned}
      <p class="bf-status">Scan the first 1000 memories across all namespaces to find untyped ones.</p>
      <Button variant="secondary" size="sm" onclick={scan}>Scan now</Button>
    {:else if untyped.length === 0}
      <p class="bf-status ok">✓ All scanned memories have a type.</p>
      <Button variant="secondary" size="sm" onclick={scan}>Re-scan</Button>
    {:else}
      <p class="bf-status">
        <b>{untyped.length}</b> memor{untyped.length === 1 ? 'y' : 'ies'} without a type found.
      </p>
      <Button variant="primary" size="sm" onclick={apply} disabled={applying}>
        {applying ? 'Backfilling…' : `Backfill ${untyped.length} to 'text'`}
      </Button>
    {/if}
  </div>
</section>

<style>
  .bf-card {
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 12px 14px;
  }
  .bf-desc {
    font-size: 0.8rem;
    color: var(--text-secondary);
    margin: 0 0 10px;
    line-height: 1.5;
  }
  .bf-desc code,
  .bf-status code {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    background: var(--bg-hover);
    padding: 1px 5px;
    border-radius: var(--radius-sm);
  }
  .bf-status {
    font-size: 0.82rem;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 10px;
  }
  .bf-status.ok {
    color: var(--green);
  }
  .bf-status b {
    color: var(--text-primary);
  }
</style>
