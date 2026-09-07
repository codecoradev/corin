/**
 * Platform-aware keyboard helpers.
 *
 * Apple platforms use ⌘ (Command) as the primary modifier; everything else
 * uses Ctrl. Handlers accept the platform key via hasMod(), labels render
 * via kbdCombo()/MOD_LABEL — no hardcoded "Ctrl+…" anywhere.
 */

/** True for macOS / iOS-style platforms (their modifier is Command). */
export function isApple(platform: string): boolean {
  return /Mac|iPhone|iPad/i.test(platform);
}

/** '⌘' on Apple platforms, 'Ctrl' elsewhere. */
export function modLabelFor(platform: string): string {
  return isApple(platform) ? '⌘' : 'Ctrl';
}

/**
 * Full combo label for display: '⌘N' on Apple (macOS convention has no
 * separator), 'Ctrl+N' elsewhere.
 */
export function kbdComboFor(platform: string, key: string): string {
  return isApple(platform) ? `⌘${key}` : `Ctrl+${key}`;
}

function platformFromNavigator(): string {
  if (typeof navigator === 'undefined') return '';
  return navigator.platform || navigator.userAgent || '';
}

export const IS_MAC = isApple(platformFromNavigator());

/** '⌘' or 'Ctrl' for the running platform. */
export const MOD_LABEL = modLabelFor(platformFromNavigator());

/** Full combo ('⌘N' / 'Ctrl+N') for the running platform. */
export function kbdCombo(key: string): string {
  return kbdComboFor(platformFromNavigator(), key);
}

/**
 * Whether the platform modifier (⌘ on Apple, Ctrl elsewhere) is held.
 * Keyboard handlers use this instead of checking ctrlKey directly so the
 * native modifier works everywhere.
 */
export function hasMod(e: { metaKey: boolean; ctrlKey: boolean }): boolean {
  return IS_MAC ? e.metaKey : e.ctrlKey;
}
