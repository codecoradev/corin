import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { focusTrap } from '../focusTrap';

describe('focusTrap action', () => {
  let container: HTMLElement;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    container.remove();
  });

  function button(label: string): HTMLButtonElement {
    const b = document.createElement('button');
    b.textContent = label;
    return b;
  }

  function tab(active: HTMLElement, shift = false): KeyboardEvent {
    const e = new KeyboardEvent('keydown', {
      key: 'Tab',
      shiftKey: shift,
      bubbles: true,
      cancelable: true,
    });
    active.dispatchEvent(e);
    return e;
  }

  it('focuses the first focusable element on mount', () => {
    container.appendChild(button('first'));
    container.appendChild(button('second'));

    const { destroy } = focusTrap(container);
    expect(document.activeElement).toBe(container.querySelector('button'));

    destroy();
  });

  it('cycles Tab from the last element back to the first', () => {
    const first = button('first');
    const last = button('last');
    container.append(first, last);

    const { destroy } = focusTrap(container);
    last.focus();

    const e = tab(last);
    expect(e.defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(first);

    destroy();
  });

  it('cycles Shift+Tab from the first element back to the last', () => {
    const first = button('first');
    const last = button('last');
    container.append(first, last);

    const { destroy } = focusTrap(container);
    first.focus();

    const e = tab(first, true);
    expect(e.defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(last);

    destroy();
  });

  it('skips disabled buttons when cycling', () => {
    const first = button('first');
    const disabled = button('mid');
    disabled.disabled = true;
    const last = button('last');
    container.append(first, disabled, last);

    const { destroy } = focusTrap(container);
    last.focus();

    tab(last);
    expect(document.activeElement).toBe(first);

    destroy();
  });

  it('restores focus to the previously focused element on destroy', () => {
    const outside = button('outside');
    document.body.appendChild(outside);
    outside.focus();

    const inner = button('inner');
    container.appendChild(inner);

    const { destroy } = focusTrap(container);
    expect(document.activeElement).toBe(inner);

    destroy();
    expect(document.activeElement).toBe(outside);

    outside.remove();
  });
});
