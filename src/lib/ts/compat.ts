/**
 * Uteke server compatibility gate — issue #289.
 *
 * Compatibility map (feature -> minimum uteke version):
 *   graphEdgeWrite    0.16.1  (#1182: graph add/remove resolved memory IDs
 *                              to nodes; on 0.16.0 both endpoints 500)
 *   namespaceMove     0.16.1  (#1183: PUT /memory accepts `namespace`)
 *   namespaceManage   0.16.1  (#1183: /namespaces/rename + /namespaces/delete
 *                              + active/deprecated count breakdown)
 *   (the whole v0.4 core redesign runs on 0.16.0)
 *
 * Detection is semver compare against /health — all gated capabilities
 * shipped in the same release, so version == capability here.
 */
import { docs } from './ipc';

export type Feature = 'graphEdgeWrite' | 'namespaceMove' | 'namespaceManage';

const MIN_VERSIONS: Record<Feature, [number, number, number]> = {
  graphEdgeWrite: [0, 16, 1],
  namespaceMove: [0, 16, 1],
  namespaceManage: [0, 16, 1],
};

let cachedVersion: string | null = null;
let cachedAt = 0;
const CACHE_MS = 60_000;

/** Test hook: clear the version cache (not part of the public API). */
export function _resetCompatCache(): void {
  cachedVersion = null;
  cachedAt = 0;
}

function parseSemver(v: string): [number, number, number] | null {
  const m = /^v?(\d+)\.(\d+)\.(\d+)/.exec(v.trim());
  return m ? [Number(m[1]), Number(m[2]), Number(m[3])] : null;
}

function gte(a: [number, number, number], b: [number, number, number]): boolean {
  return a[0] > b[0] || (a[0] === b[0] && (a[1] > b[1] || (a[1] === b[1] && a[2] >= b[2])));
}

/** Server version with a 60s cache; returns the stale value when unreachable. */
export async function serverVersion(): Promise<string | null> {
  if (cachedVersion && Date.now() - cachedAt < CACHE_MS) return cachedVersion;
  try {
    const vs = await docs.versionStatus();
    const v = vs.current ?? null;
    if (v) {
      cachedVersion = v;
      cachedAt = Date.now();
    }
    return v ?? cachedVersion;
  } catch {
    return cachedVersion;
  }
}

export function minVersion(feature: Feature): string {
  const [a, b, c] = MIN_VERSIONS[feature];
  return `${a}.${b}.${c}`;
}

/** true when the connected server supports `feature`; null when unknown server. */
export async function has(feature: Feature): Promise<boolean | null> {
  const v = await serverVersion();
  if (!v) return null;
  const cur = parseSemver(v);
  if (!cur) return null;
  return gte(cur, MIN_VERSIONS[feature]);
}

/** Features NOT supported. null = server unknown (callers must not claim support). */
export async function gatedFeatures(): Promise<Feature[] | null> {
  const v = await serverVersion();
  if (!v) return null;
  const cur = parseSemver(v);
  if (!cur) return null;
  return (Object.keys(MIN_VERSIONS) as Feature[]).filter((f) => !gte(cur, MIN_VERSIONS[f]));
}
