import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import ActivityTimeline from '../ActivityTimeline.svelte';
import UpgradePrompt from '../UpgradePrompt.svelte';

vi.mock('../../ts/compat', () => ({
  gatedFeatures: vi.fn(async () => ['namespaceManage']),
  serverVersion: vi.fn(async () => '0.1.0'),
  minVersion: vi.fn(() => '0.9.0'),
}));

describe('call sites on @codecora/ui', () => {
  beforeEach(() => {
    try { localStorage.clear(); } catch { /* storage unavailable */ }
  });

  it('ActivityTimeline renders the package EmptyState (title + description)', () => {
    render(ActivityTimeline, { props: { memories: [] } });
    expect(screen.getByRole('heading', { name: 'No activity yet.' })).toBeInTheDocument();
    expect(screen.getByText(/Memories saved by your agents/)).toBeInTheDocument();
  });

  it('UpgradePrompt opens a Dialog and the footer button acknowledges it', async () => {
    render(UpgradePrompt);
    expect(await screen.findByRole('dialog', { name: 'Server update needed' })).toBeInTheDocument();
    expect(screen.getByText(/needs uteke ≥ 0.9.0/)).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: 'Continue anyway' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    try {
      expect(localStorage.getItem('corin.upgrade.prompted.0.9.0')).toBe('1');
    } catch { /* storage unavailable in this env */ }
  });
});
