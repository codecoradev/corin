import { describe, expect, it } from 'vitest';
import { isApple, kbdComboFor, modLabelFor } from './platform';

describe('platform helpers', () => {
  it('detects Apple platforms', () => {
    expect(isApple('MacIntel')).toBe(true);
    expect(isApple('iPhone')).toBe(true);
    expect(isApple('iPad')).toBe(true);
    expect(isApple('Macintosh; Intel Mac OS X 10_15_7')).toBe(true);
  });

  it('treats Windows/Linux as non-Apple', () => {
    expect(isApple('Win32')).toBe(false);
    expect(isApple('Linux x86_64')).toBe(false);
    expect(isApple('')).toBe(false);
  });

  it('labels the modifier per platform', () => {
    expect(modLabelFor('MacIntel')).toBe('⌘');
    expect(modLabelFor('Win32')).toBe('Ctrl');
    expect(modLabelFor('Linux x86_64')).toBe('Ctrl');
  });

  it('formats combos per platform convention', () => {
    // macOS drops the separator: ⌘N, not ⌘+N.
    expect(kbdComboFor('MacIntel', 'N')).toBe('⌘N');
    expect(kbdComboFor('Win32', 'N')).toBe('Ctrl+N');
    expect(kbdComboFor('Linux x86_64', 'K')).toBe('Ctrl+K');
  });
});
