/**
 * Theme store — palette-only dark/light switch (issue #291, DESIGN.md).
 * Dark is the default identity; light is the warm-paper palette.
 * Applies `data-theme` on <html> so token overrides cascade to everything.
 * Dark maps to `data-theme="corin"` (the @codecora/tokens theme name); light
 * stays `data-theme="light"` (Corin's own palette in app.css).
 *
 * `$state` is a compiler rune — needs no import (Svelte 5, `.svelte.ts`).
 * Wrapped in a class instance so runes work at module scope and tests can
 * read live state from the exported object.
 */

export type Theme = 'dark' | 'light';
const LS_KEY = 'corin.theme';
const ATTR: Record<Theme, string> = { dark: 'corin', light: 'light' };

function initial(): Theme {
  if (typeof localStorage !== 'undefined') {
    const saved = localStorage.getItem(LS_KEY);
    if (saved === 'dark' || saved === 'light') return saved;
  }
  return 'dark';
}

class ThemeStore {
  current: Theme = $state(initial());

  private apply(t: Theme) {
    if (typeof document !== 'undefined') {
      document.documentElement.dataset.theme = ATTR[t];
    }
  }

  /** Idempotent apply at app boot (App.svelte onMount). */
  init(): void {
    this.apply(this.current);
  }

  set(t: Theme): void {
    this.current = t;
    if (typeof localStorage !== 'undefined') localStorage.setItem(LS_KEY, t);
    this.apply(t);
  }

  toggle(): void {
    this.set(this.current === 'dark' ? 'light' : 'dark');
  }
}

export const theme = new ThemeStore();
