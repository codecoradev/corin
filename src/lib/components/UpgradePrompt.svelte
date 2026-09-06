<script lang="ts">
  import { onMount } from 'svelte';
  import { X } from 'lucide-svelte';
  import { gatedFeatures, minVersion, type Feature } from '../ts/compat';

  const FEATURE_LABEL: Record<Feature, string> = {
    graphEdgeWrite: 'graph linking',
    namespaceMove: 'namespace move',
    namespaceManage: 'namespace management',
  };

  let gated = $state<Feature[] | null>(null);
  let dismissed = $state(false);

  const DISMISS_LS = 'corin.upgrade.dismissed';

  function dismiss() {
    dismissed = true;
    try {
      localStorage.setItem(DISMISS_LS, new Date().toISOString().slice(0, 10));
    } catch { /* storage unavailable */ }
  }

  onMount(async () => {
    // Re-show after a day so users aren't nagged forever.
    try {
      const d = localStorage.getItem(DISMISS_LS);
      if (d && (Date.now() - new Date(d).getTime()) < 86_400_000) {
        dismissed = true;
        return;
      }
    } catch { /* ignore */ }
    gated = await gatedFeatures().catch(() => null);
    if (gated && gated.length === 0) dismissed = true;
  });
</script>

{#if !dismissed && gated && gated.length > 0}
  <div class="upgrade-banner" role="status">
    <span class="upgrade-text">
      ⬆ Some features ({gated.map((f) => FEATURE_LABEL[f] ?? f).join(', ')}) need
      uteke ≥ {minVersion(gated[0])} — installed server is older. Run
      <code>uteke upgrade</code> to unlock.
    </span>
    <button class="upgrade-close" onclick={dismiss} aria-label="Dismiss">
      <X size={12} strokeWidth={2} />
    </button>
  </div>
{/if}

<style>
  .upgrade-banner {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 6px 40px 6px 14px;
    background: color-mix(in srgb, var(--accent, #5eead4) 12%, var(--bg-surface, #11151d));
    border-bottom: 1px solid color-mix(in srgb, var(--accent, #5eead4) 30%, transparent);
    color: var(--text-primary, #e6e9ef);
    font-size: 0.75rem;
  }
  .upgrade-text { line-height: 1.4; }
  .upgrade-text code {
    font-family: var(--font-mono, monospace);
    font-size: 0.7rem;
    background: var(--bg-hover, #1a2130);
    padding: 1px 5px;
    border-radius: 4px;
  }
  .upgrade-close {
    position: absolute;
    right: 12px;
    display: flex;
    align-items: center;
    background: none;
    border: none;
    color: var(--text-muted, #8b93a5);
    cursor: pointer;
    padding: 4px;
  }
</style>
