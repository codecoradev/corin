/**
 * GitHub Code Scanning helpers for the Cora review history viewer (#20).
 * Cora uploads findings as SARIF via `cora upload-sarif`; GitHub surfaces
 * them through the code-scanning REST API, which this module wraps as pure,
 * unit-testable functions.
 */

export type Severity = 'critical' | 'high' | 'medium' | 'low' | 'none';

export interface ScanAlert {
  number: number;
  rule: string;
  description: string;
  message: string;
  file: string;
  line: number;
  severity: Severity;
  state: 'open' | 'dismissed' | 'fixed';
  created_at: string;
  tool: string;
}

/** Raw GitHub code-scanning alert (only the fields we consume). */
interface RawAlert {
  number?: number;
  rule?: {
    id?: string;
    description?: string;
    security_severity_level?: string;
  };
  most_recent_instance?: {
    message?: { text?: string };
    location?: { path?: string; start_line?: number };
  };
  state?: string;
  created_at?: string;
  tool?: { name?: string };
}

function toSeverity(raw: RawAlert): Severity {
  const s = (raw.rule?.security_severity_level ?? '').toLowerCase();
  if (s === 'critical' || s === 'high' || s === 'medium' || s === 'low') return s;
  return 'none';
}

/** Normalize one page of GitHub alerts; tolerant of missing fields. */
export function normalizeAlerts(rows: RawAlert[]): ScanAlert[] {
  return rows.map((r) => ({
    number: r.number ?? 0,
    rule: r.rule?.id ?? 'unknown',
    description: r.rule?.description ?? '',
    message: r.most_recent_instance?.message?.text ?? '',
    file: r.most_recent_instance?.location?.path ?? '',
    line: r.most_recent_instance?.location?.start_line ?? 0,
    severity: toSeverity(r),
    state: (r.state as ScanAlert['state']) ?? 'open',
    created_at: r.created_at ?? '',
    tool: r.tool?.name ?? 'CodeCora',
  }));
}

/** New alerts per ISO-week bucket, oldest first — the trend series. */
export function alertTrend(alerts: ScanAlert[], weeks = 12): { label: string; count: number }[] {
  const buckets = new Map<string, number>();
  for (const a of alerts) {
    if (!a.created_at) continue;
    const d = new Date(a.created_at);
    if (Number.isNaN(d.getTime())) continue;
    // ISO week key: YYYY-Www (Monday-based)
    const target = new Date(Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate()));
    const day = (target.getUTCDay() + 6) % 7;
    target.setUTCDate(target.getUTCDate() - day);
    const key = target.toISOString().slice(0, 10);
    buckets.set(key, (buckets.get(key) ?? 0) + 1);
  }
  const out: { label: string; count: number }[] = [];
  const cursor = new Date();
  cursor.setUTCHours(0, 0, 0, 0);
  cursor.setUTCDate(cursor.getUTCDate() - ((cursor.getUTCDay() + 6) % 7)); // this Monday
  for (let w = weeks - 1; w >= 0; w--) {
    const d = new Date(cursor);
    d.setUTCDate(cursor.getUTCDate() - w * 7);
    const key = d.toISOString().slice(0, 10);
    out.push({ label: key, count: buckets.get(key) ?? 0 });
  }
  return out;
}

export interface SeverityBreakdown {
  critical: number;
  high: number;
  medium: number;
  low: number;
  none: number;
}

export function severityBreakdown(alerts: ScanAlert[]): SeverityBreakdown {
  const b: SeverityBreakdown = { critical: 0, high: 0, medium: 0, low: 0, none: 0 };
  for (const a of alerts) b[a.severity]++;
  return b;
}

export interface AlertFilter {
  severity: Severity | 'all';
  state: ScanAlert['state'] | 'all';
  query: string;
}

export function filterAlerts(alerts: ScanAlert[], f: AlertFilter): ScanAlert[] {
  const q = f.query.trim().toLowerCase();
  return alerts.filter((a) => {
    if (f.severity !== 'all' && a.severity !== f.severity) return false;
    if (f.state !== 'all' && a.state !== f.state) return false;
    if (q && !`${a.rule} ${a.file} ${a.message} ${a.description}`.toLowerCase().includes(q)) return false;
    return true;
  });
}

/** Fetch code-scanning alerts. Token optional for public repos. */
export async function fetchAlerts(
  repo: string,
  token: string,
  opts?: { state?: string; perPage?: number },
): Promise<ScanAlert[]> {
  const [owner, name] = repo.split('/').map((p) => p.trim());
  if (!owner || !name) throw new Error('Repo harus format owner/name');
  const url = `https://api.github.com/repos/${encodeURIComponent(owner)}/${encodeURIComponent(name)}/code-scanning/alerts?state=${opts?.state ?? 'all'}&per_page=${opts?.perPage ?? 100}`;
  const res = await fetch(url, {
    headers: {
      Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
  });
  if (res.status === 404) throw new Error('Repo tidak ditemukan atau token tanpa akses code scanning (butuh scope security_events).');
  if (res.status === 401) throw new Error('Token GitHub tidak valid (401).');
  if (res.status === 403) throw new Error('Akses ditolak (403) — cek scope token atau rate limit.');
  if (!res.ok) throw new Error(`GitHub API ${res.status}`);
  return normalizeAlerts(await res.json());
}
