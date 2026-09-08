import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Mock the ipc layer before importing the module under test
vi.mock('../ts/ipc', () => ({
  docs: {
    versionStatus: vi.fn(async () => ({ current: '0.16.0', required: '0.7.0', supported: true })),
  },
}));

import { docs } from '../ts/ipc';
import { has, minVersion, gatedFeatures, serverVersion, _resetCompatCache } from './compat';

const mockVersion = vi.mocked(docs.versionStatus);

describe('compat gate (#289)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    _resetCompatCache();
    mockVersion.mockImplementation(async () => ({ current: '0.16.0', required: '0.7.0', supported: true }));
    vi.useFakeTimers({ shouldAdvanceTime: true });
  });
  afterEach(() => vi.useRealTimers());

  it('gates graph write + namespace on 0.16.0', async () => {
    expect(await has('graphEdgeWrite')).toBe(false);
    expect(await has('namespaceMove')).toBe(false);
    expect(await has('namespaceManage')).toBe(false);
    expect(await gatedFeatures()).toEqual(['graphEdgeWrite', 'namespaceMove', 'namespaceManage']);
  });

  it('enables features on 0.16.1+', async () => {
    mockVersion.mockImplementation(async () => ({ current: '0.16.1', required: '0.7.0', supported: true }));
    // force cache refresh
    vi.advanceTimersByTime(61_000);
    expect(await has('graphEdgeWrite')).toBe(true);
    expect(await has('namespaceManage')).toBe(true);
    expect(await gatedFeatures()).toEqual([]);
  });

  it('parses v-prefixed versions', async () => {
    mockVersion.mockImplementation(async () => ({ current: 'v0.17.0', required: '0.7.0', supported: true }));
    vi.advanceTimersByTime(61_000);
    expect(await has('namespaceMove')).toBe(true);
    // second call inside cache window returns the refreshed value
    expect(await serverVersion()).toBe('v0.17.0');
  });

  it('minVersion strings are correct', () => {
    expect(minVersion('graphEdgeWrite')).toBe('0.16.1');
    expect(minVersion('namespaceManage')).toBe('0.16.1');
  });

  it('unknown server -> null (caller decides)', async () => {
    mockVersion.mockImplementation(async () => { throw new Error('down'); });
    vi.advanceTimersByTime(61_000);
    expect(await has('graphEdgeWrite')).toBe(null);
    expect(await gatedFeatures()).toBe(null);
  });
});
