<script lang="ts">
  let expanded = $state<Set<string>>(new Set(['a']));
  let kids: Record<string, string[]> = { a: ['a1', 'a2'] };

  export function toggle(id: string) {
    const next = new Set(expanded);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    expanded = next;
  }

  export function isOpen(id: string) {
    return expanded.has(id);
  }
</script>

{#snippet node(id: string)}
  {@const open = expanded.has(id)}
  <button data-testid="toggle-{id}" onclick={() => toggle(id)}>tog</button>
  {#if open}
    <ul data-testid="kids-{id}">
      {#each kids[id] ?? [] as c}{@render node(c)}{/each}
    </ul>
  {/if}
{/snippet}

{#each Object.keys(kids) as id}{@render node(id)}{/each}
