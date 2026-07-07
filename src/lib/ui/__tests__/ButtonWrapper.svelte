<script lang="ts">
  import Button from '../Button.svelte';
  import type { Component } from 'svelte';

  let {
    variant = 'secondary',
    size = 'md',
    disabled = false,
    onclick,
    title,
    text = 'Test',
    icon,
  }: {
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
    size?: 'sm' | 'md' | 'lg';
    disabled?: boolean;
    onclick?: (e: MouseEvent) => void;
    title?: string;
    text?: string;
    icon?: Component;
  } = $props();
</script>

<Button {variant} {size} {disabled} {onclick} {title} {icon}>{text}</Button>
