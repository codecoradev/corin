<script lang="ts">
  import { onMount } from 'svelte';
  import { TriangleAlert } from 'lucide-svelte';
  import { gatedFeatures, minVersion, serverVersion, type Feature } from '../ts/compat';
  import { Modal } from '../ui';

  const FEATURE_LABEL: Record<Feature, string> = {
    graphEdgeWrite: 'graph linking',
    namespaceMove: 'namespace move',
    namespaceManage: 'namespace management',
  };

  let gated = $state<Feature[] | null>(null);
  let serverVer = $state<string | null>(null);
  let open = $state(false);

  /**
   * First-launch notice: shows once per required-minimum version (acknow-
   * ledgement persisted), instead of a thin banner that other content
   * buries. Once the server is upgraded the gated list empties and this
   * never appears again.
   */
  onMount(async () => {
    gated = await gatedFeatures().catch(() => null);
    if (!gated || gated.length === 0) return;
    serverVer = await serverVersion().catch(() => null);
    const key = `corin.upgrade.prompted.${minVersion(gated[0])}`;
    try {
      if (localStorage.getItem(key) === '1') return; // already acknowledged
    } catch { /* storage unavailable */ }
    open = true;
  });

  function acknowledge() {
    try {
      const key = `corin.upgrade.prompted.${gated ? minVersion(gated[0]) : ''}`;
      localStorage.setItem(key, '1');
    } catch { /* storage unavailable */ }
    open = false;
  }
</script>

<Modal open={open} title="Server update needed" onclose={acknowledge} width="480px">
  <div class="upgrade-dialog">
    <div class="upgrade-icon"><TriangleAlert size={22} strokeWidth={1.75} /></div>
    <p class="lead">
      The connected uteke server{serverVer ? ` (v${serverVer})` : ''} is older than some
      CorIn features require. Those features stay hidden until the server is updated:
    </p>
    <ul class="feature-list">
      {#each gated ?? [] as f (f)}
        <li><b>{FEATURE_LABEL[f] ?? f}</b><span class="req">needs uteke ≥ {minVersion(f)}</span></li>
      {/each}
    </ul>
    <p class="how">
      Update uteke on the machine running the server (<code>uteke upgrade</code>), then
      reopen CorIn. Everything else keeps working in the meantime.
    </p>
    <button class="ack-btn" onclick={acknowledge}>Continue anyway</button>
  </div>
</Modal>

<style>
  .upgrade-dialog {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }
  .upgrade-icon {
    width: 40px;
    height: 40px;
    margin: 0 auto 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-lg);
    background: var(--color-yellow-bg);
    color: var(--yellow);
  }
  .lead {
    margin: 0 0 12px;
    line-height: 1.55;
    text-align: center;
  }
  .feature-list {
    list-style: none;
    margin: 0 0 12px;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .feature-list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 12px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
  }
  .feature-list b {
    color: var(--text-primary);
    font-weight: 600;
  }
  .req {
    font-size: 0.72rem;
    color: var(--yellow);
    font-family: var(--font-mono);
    white-space: nowrap;
  }
  .how {
    margin: 0 0 14px;
    font-size: 0.78rem;
    line-height: 1.5;
    color: var(--text-muted);
  }
  .how code {
    font-family: var(--font-mono);
    font-size: 0.75rem;
    background: var(--bg-hover);
    padding: 1px 6px;
    border-radius: var(--radius-sm);
    color: var(--accent);
  }
  .ack-btn {
    width: 100%;
    padding: 9px;
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    font-size: 0.85rem;
    cursor: pointer;
  }
  .ack-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
