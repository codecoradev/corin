/// Shared Svelte transition presets — import where needed.
/// Usage: `import { fadeQuick, modalScale } from '$lib/transitions'`

import { fade, fly, scale, slide } from 'svelte/transition';
import type { TransitionConfig } from 'svelte/transition';
import { isWebMode } from './ts/transport';

type EasingFunc = (t: number) => number;

// Web-mode guard: in a background browser pane, Svelte's out-transition
// completion callbacks never fire — keyed swaps and overlay closes leave
// stale DOM stacked forever. A zero-duration no-op keeps the API contract
// (Svelte requires a function) while finishing instantly; desktop keeps
// the real presets untouched.
const noOp = (): TransitionConfig => ({ duration: 0 });
const guarded = <A extends unknown[]>(
  preset: (node: Element, ...args: A) => TransitionConfig,
): ((node: Element, ...args: A) => TransitionConfig) => {
  if (!isWebMode) return preset;
  return (node, ..._args) => ({ duration: 0 });
};

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
export const fadeQuick = guarded((node: Element): TransitionConfig =>
  fade(node, { duration: 150 }));

/** Fade + subtle upward slide for list items / content (200ms) */
export const fadeUp = guarded((node: Element, delay: number = 0): TransitionConfig =>
  fly(node, { duration: 200, delay, y: 8, easing: easeOut }));

/** Slide down from top for notifications / bars (200ms) */
export const slideDown = guarded((node: Element): TransitionConfig =>
  fly(node, { duration: 200, y: -12, easing: easeOut }));

// ── Modal & overlay transitions ─────────────────────────────────

/** Scale + fade for modal dialogs (200ms, starts at 96%) */
export const modalScale = guarded((node: Element): TransitionConfig =>
  scale(node, { duration: 200, start: 0.96, opacity: 0, easing: easeOut }));

/** Scale + fade for confirm dialogs (slightly snappier) */
export const dialogPop = guarded((node: Element): TransitionConfig =>
  scale(node, { duration: 150, start: 0.92, opacity: 0, easing: springOut }));

/** Backdrop fade (120ms) */
export const backdropFade = guarded((node: Element): TransitionConfig =>
  fade(node, { duration: 120 }));

/** Plain fade for overlay wrappers (App-level detail/settings panels). */
export const overlayFade = guarded(
  (node: Element, params?: { duration?: number }): TransitionConfig =>
    fade(node, { duration: params?.duration ?? 150 }));

/** Fly for overlay wrappers (e.g. memory editor). */
export const overlayFlyUp = guarded((node: Element): TransitionConfig =>
  fly(node, { duration: 200, y: 20, opacity: 0 }));

// ── Expand / collapse ───────────────────────────────────────────

/** Slide for tree expand / accordion (smooth height) */
export const expandSlide = guarded((node: Element): TransitionConfig =>
  slide(node, { duration: 180, easing: easeOut }));

/** Fly from left for sidebar / panel slide-in */
export const slideInRight = guarded((node: Element): TransitionConfig =>
  fly(node, { duration: 200, x: -20, easing: easeOut, opacity: 0 }));
