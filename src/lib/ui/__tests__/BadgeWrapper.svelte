<script lang="ts">
  import Badge from '../Badge.svelte';
  let { color = 'default', text = 'tag' }: { color?: 'default' | 'accent' | 'green' | 'red' | 'yellow' | 'mauve' | 'teal'; text?: string } = $props();
</script>
<Badge {color}>{text}</Badge>
