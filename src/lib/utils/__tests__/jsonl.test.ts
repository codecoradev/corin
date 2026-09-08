import { describe, it, expect } from 'vitest';
import { parseJsonl } from '../jsonl';

describe('parseJsonl', () => {
  it('parses entries and aggregates namespaces and tags', () => {
    const data = [
      JSON.stringify({ id: 'a', content: 'hello', tags: ['x', 'y'], namespace: 'default' }),
      JSON.stringify({ id: 'b', content: 'world', tags: ['x'], namespace: 'research' }),
    ].join('\n');

    const p = parseJsonl(data);
    expect(p.count).toBe(2);
    expect(p.malformed).toBe(0);
    expect(p.namespaces).toEqual(['default', 'research']);
    expect(p.tags).toEqual(['x', 'y']);
    expect(p.first5).toHaveLength(2);
    expect(p.first5[0].content).toBe('hello');
  });

  it('counts malformed lines without failing', () => {
    const p = parseJsonl('{"content":"ok"}\nnot-json\n\n{also bad}');
    expect(p.count).toBe(1);
    expect(p.malformed).toBe(2);
  });

  it('returns an empty preview for blank input', () => {
    const p = parseJsonl('\n  \n');
    expect(p.count).toBe(0);
    expect(p.first5).toHaveLength(0);
    expect(p.namespaces).toEqual([]);
  });

  it('keeps at most 5 entries in first5', () => {
    const data = Array.from({ length: 8 }, (_, i) => JSON.stringify({ content: `m${i}` })).join('\n');
    expect(parseJsonl(data).first5).toHaveLength(5);
  });

  it('tolerates entries without tags or namespace', () => {
    const p = parseJsonl('{"content":"bare"}');
    expect(p.count).toBe(1);
    expect(p.first5[0].tags).toEqual([]);
    expect(p.first5[0].namespace).toBeNull();
  });
});
