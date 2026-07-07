/// Shared formatting utilities — used across DocumentsView, SettingsModal, RoomsView.

/** Format an ISO date string to a readable "MMM d, yyyy" format. */
export function formatDate(iso: string | null | undefined): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (isNaN(d.getTime())) return '';
  return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}

/** Format milliseconds to a human-readable duration (e.g. "1.2s", "350ms"). */
export function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60_000).toFixed(1)}m`;
}

/** Format an ISO datetime to a locale time string (e.g. "2:30 PM"). */
export function formatTime(iso: string): string {
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso;
  return d.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
}

/** Relative time (e.g. "3m ago", "2h ago", "5d ago"). */
export function relativeTime(dateStr: string): string {
  const d = new Date(dateStr);
  if (isNaN(d.getTime())) return dateStr;
  const diff = Date.now() - d.getTime();
  const sec = Math.floor(diff / 1000);
  if (sec < 60) return 'just now';
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.floor(hr / 24);
  if (day < 30) return `${day}d ago`;
  return formatDate(dateStr);
}

/** Count words in a text string. */
export function getWordCount(text: string): number {
  return text.trim() ? text.trim().split(/\s+/).length : 0;
}

/** Estimate reading time from word count (200 wpm). Returns "" if < 1 min. */
export function getReadingTime(words: number): string {
  if (words < 200) return '';
  const mins = Math.ceil(words / 200);
  return `${mins} min read`;
}
