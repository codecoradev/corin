/// Shared Svelte transition presets — import where needed.
/// Usage: `import { fadeQuick, modalScale } from '$lib/transitions'`

import { fade, fly, scale, slide } from 'svelte/transition';
import type { TransitionConfig } from 'svelte/transition';

type EasingFunc = (t: number) => number;

// Easing — cubic-bezier approximations for smooth UI motion
export const easeOut: EasingFunc = (t: number) => 1 - Math.pow(1 - t, 3);
export const easeInOut: EasingFunc = (t: number) =>
  t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
export const springOut: EasingFunc = (t: number) => {
  const c = 1.70158;
  return 1 + (c + 1) * Math.pow(t - 1, 3) + c * Math.pow(t - 1, 2);
};

// ── View & content transitions ──────────────────────────────────

/** Quick fade for view switches (150ms) */
export const fadeQuick = (node: Element): TransitionConfig =>
  fade(node, { duration: 150 });

/** Fade + subtle upward slide for list items / content (200ms) */
export const fadeUp = (node: Element, delay = 0): TransitionConfig =>
  fly(node, { duration: 200, delay, y: 8, easing: easeOut });

/** Slide down from top for notifications / bars (200ms) */
export const slideDown = (node: Element): TransitionConfig =>
  fly(node, { duration: 200, y: -12, easing: easeOut });

// ── Modal & overlay transitions ─────────────────────────────────

/** Scale + fade for modal dialogs (200ms, starts at 96%) */
export const modalScale = (node: Element): TransitionConfig =>
  scale(node, { duration: 200, start: 0.96, opacity: 0, easing: easeOut });

/** Scale + fade for confirm dialogs (slightly snappier) */
export const dialogPop = (node: Element): TransitionConfig =>
  scale(node, { duration: 150, start: 0.92, opacity: 0, easing: springOut });

/** Backdrop fade (120ms) */
export const backdropFade = (node: Element): TransitionConfig =>
  fade(node, { duration: 120 });

// ── Expand / collapse ───────────────────────────────────────────

/** Slide for tree expand / accordion (smooth height) */
export const expandSlide = (node: Element): TransitionConfig =>
  slide(node, { duration: 180, easing: easeOut });

/** Fly from left for sidebar / panel slide-in */
export const slideInRight = (node: Element): TransitionConfig =>
  fly(node, { duration: 200, x: -20, easing: easeOut, opacity: 0 });
