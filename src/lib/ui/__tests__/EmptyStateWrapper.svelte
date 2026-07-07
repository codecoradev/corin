<script lang="ts">
  import EmptyState from '../EmptyState.svelte';
  import Button from '../Button.svelte';
  let {
    icon = '📭',
    title = 'Nothing here',
    subtitle,
    withAction = false,
  }: { icon?: string; title?: string; subtitle?: string; withAction?: boolean } = $props();
</script>

<EmptyState {icon} {title} {subtitle}>
  {#if withAction}
    <Button variant="primary">Create</Button>
  {/if}
</EmptyState>
