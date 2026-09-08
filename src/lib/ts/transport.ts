/**
 * Transport layer for all CorIn IPC.
 *
 * - Desktop (Tauri): delegates straight to `invoke()` — behavior identical
 *   to before this layer existed.
 * - Web (plain browser, no `__TAURI_INTERNALS__`): routes commands through
 *   the REST handlers in `web-routes.ts` (uteke-serve via the `/api` proxy).
 */
import { invoke } from '@tauri-apps/api/core';
import { webHandlers, type Payload } from './web-routes';

/** True when the UI runs in a plain browser instead of the Tauri webview. */
export const isWebMode: boolean = typeof window !== 'undefined' && !('__TAURI_INTERNALS__' in window);

/**
 * Invoke a CorIn command through the active transport.
 * Signature mirrors Tauri's `invoke` so ipc.ts call sites stay mechanical.
 */
export async function call<T>(command: string, payload?: Payload): Promise<T> {
  if (!isWebMode) return invoke<T>(command, payload);
  const handler = webHandlers[command];
  if (!handler) throw new Error(`Perintah '${command}' tidak tersedia di mode web`);
  return handler(payload ?? {}) as Promise<T>;
}
