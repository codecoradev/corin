// Connections store — reactive list + periodic health polling.
//
// Phase 6: status stays fresh without user manually clicking "Test".
// Polling calls test_connection on each connection (writes status to DB),
// then refreshes the list. Polls the PRIMARY connection more frequently
// (every 15s) and others every 60s to keep load light.

import { connection, type ConnectionInfo, type HealthInfo } from '../ts/ipc';
import { invalidateAll } from './cache.svelte';

const PRIMARY_POLL_MS = 15_000;
const ALL_POLL_MS = 60_000;

let connections = $state<ConnectionInfo[]>([]);
let loading = $state(true);
let primaryPolling = $state(false);
let allPolling = $state(false);
let lastError = $state<string | null>(null);

let primaryTimer: ReturnType<typeof setInterval> | null = null;
let allTimer: ReturnType<typeof setInterval> | null = null;

/** Refresh the connection list from the backend (DB read). */
async function refresh() {
  try {
    connections = await connection.list();
    lastError = null;
  } catch (e) {
    lastError = String(e);
    console.error('connections.refresh failed', e);
  } finally {
    loading = false;
  }
}

/** Health-check the primary connection, then refresh status badges. */
async function pollPrimary() {
  const primary = connections.find((c) => c.is_primary);
  if (!primary) return;
  try {
    await connection.test(primary.id);
    await refresh();
  } catch (e) {
    console.error('connections.pollPrimary failed', e);
  }
}

/** Health-check every connection concurrently, then refresh. */
async function pollAll() {
  if (connections.length === 0) return;
  await Promise.allSettled(connections.map((c) => connection.test(c.id)));
  await refresh();
}

/** Start periodic polling. Safe to call multiple times. */
function startPolling() {
  if (primaryTimer === null) {
    primaryTimer = setInterval(pollPrimary, PRIMARY_POLL_MS);
    primaryPolling = true;
  }
  if (allTimer === null) {
    allTimer = setInterval(pollAll, ALL_POLL_MS);
    allPolling = true;
  }
}

/** Stop periodic polling. */
function stopPolling() {
  if (primaryTimer !== null) {
    clearInterval(primaryTimer);
    primaryTimer = null;
    primaryPolling = false;
  }
  if (allTimer !== null) {
    clearInterval(allTimer);
    allTimer = null;
    allPolling = false;
  }
}

/** Reconnect to a connection (rebuilds live backend, no app restart). */
async function reconnect(id: string): Promise<HealthInfo> {
  const result = await connection.reconnect(id);
  invalidateAll(); // backend changed — stats/namespaces cache is stale
  await refresh();
  return result;
}

/** Disconnect the active memory backend (drops the live client). */
async function disconnect(): Promise<void> {
  await connection.disconnect();
  invalidateAll();
  await refresh();
}

/** Set primary + live-rebuild the active backend. */
async function setPrimary(id: string) {
  await connection.setPrimary(id);
  invalidateAll();
  await refresh();
}

/** Delete a connection (backend clears token before row removal). */
async function remove(id: string) {
  await connection.delete(id);
  await refresh();
}

export function getConnectionsStore() {
  // Initial load (fire-and-forget; callers read `loading`).
  void refresh();

  return {
    get connections() { return connections; },
    get loading() { return loading; },
    get lastError() { return lastError; },
    get primaryPolling() { return primaryPolling; },
    get allPolling() { return allPolling; },
    refresh,
    reconnect,
    disconnect,
    setPrimary,
    remove,
    startPolling,
    stopPolling,
  };
}
