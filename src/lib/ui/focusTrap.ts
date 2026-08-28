/**
 * Svelte action: trap Tab focus inside a dialog container.
 *
 * - focuses the first focusable element on mount (restores the previously
 *   focused element on destroy)
 * - keeps Tab / Shift+Tab cycling inside the container
 * - pure DOM, no dependencies — use with `use:focusTrap` on the dialog node
 */

const FOCUSABLE = [
  'a[href]',
  'button:not([disabled])',
  'textarea:not([disabled])',
  'input:not([type="hidden"]):not([disabled])',
  'select:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(', ');

export function focusTrap(node: HTMLElement) {
  const previous = document.activeElement as HTMLElement | null;

  function focusables(): HTMLElement[] {
    return Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
      // checkVisibility() is native in browsers; jsdom (tests) has no layout,
      // so fall back to trusting the DOM.
      (el) => (typeof el.checkVisibility === 'function' ? el.checkVisibility() : true),
    );
  }

  // Move focus into the dialog on mount.
  const initial = focusables()[0] ?? node;
  initial.focus();

  function onKeydown(e: KeyboardEvent) {
    if (e.key !== 'Tab') return;
    const items = focusables();
    if (items.length === 0) {
      e.preventDefault();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement as HTMLElement | null;

    if (!active || !node.contains(active)) {
      // Focus escaped (e.g. click on chrome) — pull it back in.
      e.preventDefault();
      first.focus();
    } else if (e.shiftKey && active === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && active === last) {
      e.preventDefault();
      first.focus();
    }
  }

  node.addEventListener('keydown', onKeydown);
  return {
    destroy() {
      node.removeEventListener('keydown', onKeydown);
      previous?.focus?.();
    },
  };
}
