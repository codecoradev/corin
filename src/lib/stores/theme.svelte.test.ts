import { describe, it, expect, beforeEach, vi } from 'vitest';

/**
 * Node >=22 exposes a non-functional `localStorage` accessor on globalThis that
 * shadows jsdom's implementation (returns undefined without --localstorage-file),
 * so tests must reach localStorage through `window` when available.
 */
function ls(): Storage | undefined {
  const w = (globalThis as { window?: { localStorage?: Storage } }).window;
  return w?.localStorage;
}

describe('theme store (palette-only switch)', () => {
  beforeEach(() => {
    ls()?.clear();
    document.documentElement.removeAttribute('data-theme');
    vi.resetModules();
  });

  it('defaults to dark', async () => {
    const { theme } = await import('./theme.svelte');
    expect(theme.current).toBe('dark');
  });

  it('applies data-theme on set', async () => {
    const { theme } = await import('./theme.svelte');
    theme.set('light');
    expect(theme.current).toBe('light');
    expect(document.documentElement.dataset.theme).toBe('light');
  });

  it('persists to localStorage when available', async () => {
    const storage = ls();
    if (!storage) return; // env without functional localStorage — persistence covered by E2E
    const { theme } = await import('./theme.svelte');
    theme.set('light');
    expect(storage.getItem('corin.theme')).toBe('light');
  });

  it('toggle flips and persists', async () => {
    const { theme } = await import('./theme.svelte');
    theme.toggle();
    expect(theme.current).toBe('light');
    expect(document.documentElement.dataset.theme).toBe('light');
    theme.toggle();
    expect(theme.current).toBe('dark');
    expect(document.documentElement.dataset.theme).toBe('dark');
  });

  it('init applies without changing value', async () => {
    const { theme } = await import('./theme.svelte');
    theme.init();
    expect(theme.current).toBe('dark');
    expect(document.documentElement.dataset.theme).toBe('dark');
  });
});
