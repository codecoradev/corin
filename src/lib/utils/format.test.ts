import { describe, it, expect } from 'vitest';
import {
  formatDate,
  formatDuration,
  formatTime,
  relativeTime,
  getWordCount,
  getReadingTime,
} from './format';

describe('formatDate', () => {
  it('formats valid ISO date', () => {
    const result = formatDate('2025-01-15T10:30:00Z');
    expect(result).toMatch(/Jan/);
    expect(result).toMatch(/15/);
    expect(result).toMatch(/2025/);
  });

  it('returns empty string for null/undefined', () => {
    expect(formatDate(null)).toBe('');
    expect(formatDate(undefined)).toBe('');
  });

  it('returns empty string for invalid date', () => {
    expect(formatDate('not-a-date')).toBe('');
  });
});

describe('formatDuration', () => {
  it('formats milliseconds', () => {
    expect(formatDuration(50)).toBe('50ms');
    expect(formatDuration(999)).toBe('999ms');
  });

  it('formats seconds', () => {
    expect(formatDuration(1500)).toBe('1.5s');
    expect(formatDuration(30_000)).toBe('30.0s');
  });

  it('formats minutes', () => {
    expect(formatDuration(90_000)).toBe('1.5m');
    expect(formatDuration(120_000)).toBe('2.0m');
  });
});

describe('getWordCount', () => {
  it('counts words correctly', () => {
    expect(getWordCount('hello world')).toBe(2);
    expect(getWordCount('one two three four five')).toBe(5);
  });

  it('returns 0 for empty/whitespace', () => {
    expect(getWordCount('')).toBe(0);
    expect(getWordCount('   ')).toBe(0);
  });

  it('handles newlines and extra spaces', () => {
    expect(getWordCount('hello\n\nworld  foo')).toBe(3);
  });
});

describe('getReadingTime', () => {
  it('returns empty string for < 200 words', () => {
    expect(getReadingTime(0)).toBe('');
    expect(getReadingTime(199)).toBe('');
  });

  it('returns "1 min read" for 200+ words', () => {
    expect(getReadingTime(200)).toBe('1 min read');
    expect(getReadingTime(400)).toBe('2 min read');
  });

  it('rounds up', () => {
    expect(getReadingTime(201)).toBe('2 min read');
  });
});

describe('relativeTime', () => {
  it('returns "just now" for recent times', () => {
    expect(relativeTime(new Date().toISOString())).toBe('just now');
  });

  it('returns minutes ago', () => {
    const fiveMinAgo = new Date(Date.now() - 5 * 60 * 1000).toISOString();
    expect(relativeTime(fiveMinAgo)).toBe('5m ago');
  });

  it('returns hours ago', () => {
    const threeHrAgo = new Date(Date.now() - 3 * 60 * 60 * 1000).toISOString();
    expect(relativeTime(threeHrAgo)).toBe('3h ago');
  });

  it('falls back to formatted date for invalid input', () => {
    expect(relativeTime('not-a-date')).toBe('not-a-date');
  });
});
