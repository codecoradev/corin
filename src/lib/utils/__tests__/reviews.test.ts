import { describe, it, expect } from 'vitest';
import {
  normalizeAlerts,
  alertTrend,
  severityBreakdown,
  filterAlerts,
} from '../reviews';

const fixture = [
  {
    number: 1,
    rule: {
      id: 'js/sql-injection',
      description: 'SQL built from user input',
      security_severity_level: 'high',
    },
    most_recent_instance: {
      message: { text: 'Unsanitized input used in query' },
      location: { path: 'src/db.ts', start_line: 42 },
    },
    state: 'open',
    created_at: '2026-08-01T10:00:00Z',
    tool: { name: 'CodeCora' },
  },
  {
    number: 2,
    rule: {
      id: 'js/hardcoded-secret',
      description: 'Hardcoded credential',
      security_severity_level: 'critical',
    },
    most_recent_instance: {
      message: { text: 'Secret literal in source' },
      location: { path: 'src/config.ts', start_line: 7 },
    },
    state: 'fixed',
    created_at: '2026-08-20T10:00:00Z',
  },
  {
    number: 3,
    rule: {
      id: 'js/xss',
      description: 'XSS via innerHTML',
      security_severity_level: 'medium',
    },
    most_recent_instance: {
      message: { text: 'DOM write with tainted data' },
      location: { path: 'src/ui.ts', start_line: 12 },
    },
    state: 'open',
    created_at: '2026-08-22T10:00:00Z',
  },
];

describe('normalizeAlerts', () => {
  it('maps the fields the viewer needs', () => {
    const alerts = normalizeAlerts(fixture);
    expect(alerts).toHaveLength(3);
    expect(alerts[0]).toMatchObject({
      number: 1,
      rule: 'js/sql-injection',
      file: 'src/db.ts',
      line: 42,
      severity: 'high',
      state: 'open',
    });
  });

  it('falls back to none for unknown severity and defaults missing fields', () => {
    const alerts = normalizeAlerts([{}]);
    expect(alerts[0].severity).toBe('none');
    expect(alerts[0].rule).toBe('unknown');
    expect(alerts[0].line).toBe(0);
  });
});

describe('severityBreakdown', () => {
  it('counts per severity', () => {
    const b = severityBreakdown(normalizeAlerts(fixture));
    expect(b).toEqual({ critical: 1, high: 1, medium: 1, low: 0, none: 0 });
  });
});

describe('filterAlerts', () => {
  const alerts = normalizeAlerts(fixture);

  it('filters by severity', () => {
    expect(filterAlerts(alerts, { severity: 'high', state: 'all', query: '' })).toHaveLength(1);
  });

  it('filters by state', () => {
    expect(filterAlerts(alerts, { severity: 'all', state: 'fixed', query: '' })).toHaveLength(1);
  });

  it('matches rule, file, or message text', () => {
    expect(filterAlerts(alerts, { severity: 'all', state: 'all', query: 'config.ts' })).toHaveLength(1);
    expect(filterAlerts(alerts, { severity: 'all', state: 'all', query: 'xss' })).toHaveLength(1);
    expect(filterAlerts(alerts, { severity: 'all', state: 'all', query: 'tidak-ada' })).toHaveLength(0);
  });
});

describe('alertTrend', () => {
  it('produces the requested number of weekly buckets, oldest first', () => {
    const trend = alertTrend(normalizeAlerts(fixture), 4);
    expect(trend).toHaveLength(4);
    expect(trend[3].label <= trend[2].label).toBe(false); // ascending
  });

  it('counts alerts into their week bucket', () => {
    const trend = alertTrend(normalizeAlerts(fixture), 12);
    const total = trend.reduce((sum, t) => sum + t.count, 0);
    expect(total).toBe(3);
  });
});
