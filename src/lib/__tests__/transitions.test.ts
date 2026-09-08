import { describe, it, expect, afterEach } from 'vitest';
import {
  fadeQuick,
  modalScale,
  overlayFade,
  overlayFlyUp,
  slideDown,
  expandSlide,
  motionSuppressed,
} from '../transitions';

// Regression lock for #269: in web mode (and whenever the user asks for
// reduced motion) every shared preset must resolve to a zero-duration
// transition. Svelte's out-transition completion never fires in background
// panes, so a real transition there stacked stale views forever.

const node = document.createElement('div');

afterEach(() => {
  delete (window as any).__TAURI_INTERNALS__;
});

describe('motion suppression (web mode + reduced motion)', () => {
  it('is always on in a plain browser', () => {
    // vitest runs without __TAURI_INTERNALS__ → web mode → suppressed.
    expect(motionSuppressed()).toBe(true);
  });

  it('is on for desktop when prefers-reduced-motion is set', () => {
    (window as any).__TAURI_INTERNALS__ = {};
    (window as any).matchMedia = (query: string) => ({
      matches: query.includes('prefers-reduced-motion'),
      media: query,
    });
    expect(motionSuppressed()).toBe(true);
  });

  it('is off for desktop when the user allows motion', () => {
    (window as any).__TAURI_INTERNALS__ = {};
    (window as any).matchMedia = (query: string) => ({
      matches: false,
      media: query,
    });
    expect(motionSuppressed()).toBe(false);
  });
});

describe('presets collapse to zero duration when suppressed', () => {
  it('every shared preset returns duration 0 in web mode', () => {
    const presets = [fadeQuick, modalScale, overlayFade, overlayFlyUp, slideDown, expandSlide];
    for (const preset of presets) {
      const config = preset(node);
      expect(config.duration).toBe(0);
    }
  });
});
