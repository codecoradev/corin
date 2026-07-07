<script lang="ts">
  import Badge from '../Badge.svelte';
  let { color = 'default', text = 'tag' }: { color?: string; text?: string } = $props();
</script>
<Badge {color}>{text}</Badge>
