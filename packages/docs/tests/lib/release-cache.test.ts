import { afterEach, describe, expect, it, vi } from 'vitest';
import fixture from '../fixtures/release-2.1.0.json';

// Like the real unstable_cache: resolved values are stored, thrown errors are not.
vi.mock('next/cache', () => ({
  unstable_cache: <T extends () => Promise<unknown>>(fn: T) => {
    let stored: { value: Awaited<ReturnType<T>> } | undefined;
    return async () => {
      if (!stored) {
        stored = { value: (await fn()) as Awaited<ReturnType<T>> };
      }
      return stored.value;
    };
  },
}));

import { getReleaseDates } from '@/lib/changelog';
import { getLatestRelease } from '@/lib/github-release';

describe('GitHub failures are not cached', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('getLatestRelease retries after a failed fetch and then caches the success', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce({ ok: false, status: 503 })
      .mockResolvedValue({ ok: true, status: 200, json: async () => fixture });
    vi.stubGlobal('fetch', fetchMock);

    await expect(getLatestRelease()).resolves.toBeNull();
    const second = await getLatestRelease();
    expect(second?.version).toBe('2.1.0');
    await getLatestRelease();
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it('getReleaseDates retries after a failed fetch', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const fetchMock = vi
      .fn()
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValue({
        ok: true,
        status: 200,
        json: async () => [
          { tag_name: '@curl-runner/cli@2.1.0', published_at: '2026-10-28T13:42:19Z' },
        ],
      });
    vi.stubGlobal('fetch', fetchMock);

    await expect(getReleaseDates()).resolves.toEqual({});
    const dates = await getReleaseDates();
    expect(dates['2.1.0']).toBe('2026-10-28T13:42:19Z');
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });
});
