import { describe, it, expect, vi, beforeEach } from 'vitest';
import { flushSync } from 'svelte';
import { toastStore } from './toast.svelte.ts';

// Mock timers to test auto-dismiss
vi.useFakeTimers();

describe('toastStore', () => {
  beforeEach(() => {
    toastStore.clear();
    vi.clearAllTimers();
  });

  it('starts with empty list', () => {
    expect(toastStore.list).toHaveLength(0);
  });

  it('adds a success toast', () => {
    toastStore.success('Saved!');
    flushSync();
    expect(toastStore.list).toHaveLength(1);
    expect(toastStore.list[0].type).toBe('success');
    expect(toastStore.list[0].message).toBe('Saved!');
  });

  it('adds an error toast', () => {
    toastStore.error('Something went wrong');
    flushSync();
    expect(toastStore.list[0].type).toBe('error');
  });

  it('adds an info toast', () => {
    toastStore.info('Heads up');
    flushSync();
    expect(toastStore.list[0].type).toBe('info');
  });

  it('auto-dismisses after default duration', () => {
    toastStore.success('Temp');
    flushSync();
    expect(toastStore.list).toHaveLength(1);

    vi.advanceTimersByTime(3000);
    flushSync();
    expect(toastStore.list).toHaveLength(0);
  });

  it('error toasts persist longer by default (6s)', () => {
    toastStore.error('Fail');
    flushSync();

    vi.advanceTimersByTime(3000);
    flushSync();
    expect(toastStore.list).toHaveLength(1); // still there

    vi.advanceTimersByTime(3000);
    flushSync();
    expect(toastStore.list).toHaveLength(0); // gone after 6s
  });

  it('dismiss removes specific toast by id', () => {
    toastStore.success('A');
    toastStore.success('B');
    flushSync();
    const firstId = toastStore.list[0].id;

    toastStore.dismiss(firstId);
    flushSync();
    expect(toastStore.list).toHaveLength(1);
    expect(toastStore.list[0].message).toBe('B');
  });

  it('clear removes all toasts', () => {
    toastStore.success('A');
    toastStore.error('B');
    toastStore.info('C');
    flushSync();

    toastStore.clear();
    expect(toastStore.list).toHaveLength(0);
  });

  it('increments unique ids', () => {
    toastStore.success('A');
    flushSync();
    const id1 = toastStore.list[0].id;

    toastStore.clear();
    flushSync();

    toastStore.success('B');
    flushSync();
    const id2 = toastStore.list[0].id;

    expect(id2).toBeGreaterThan(id1);
  });
});
