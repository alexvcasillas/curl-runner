import { afterEach, describe, expect, it, vi } from 'vitest';
import fixture from '../fixtures/release-2.1.0.json';

vi.mock('next/cache', () => ({
  unstable_cache: <T extends (...args: never[]) => unknown>(fn: T) => fn,
}));

import {
  formatFileSize,
  getLatestRelease,
  getPlatformLabel,
  parseRelease,
} from '@/lib/github-release';

describe('parseRelease', () => {
  it('reads the version, date and checksum URL', () => {
    const release = parseRelease(fixture);
    expect(release?.version).toBe('2.1.0');
    expect(release?.publishedAt).toBe('2026-10-28T13:42:19Z');
    expect(release?.checksumUrl).toMatch(/SHA256SUMS\.txt$/);
  });

  it('orders the five binaries by platform and formats sizes', () => {
    const release = parseRelease(fixture);
    const labels = release?.binaries.map((b) => getPlatformLabel(b.os, b.arch));
    expect(labels).toEqual([
      'macOS Apple Silicon',
      'macOS Intel',
      'Linux x64',
      'Linux ARM64',
      'Windows x64',
    ]);
    expect(release?.binaries.map((b) => formatFileSize(b.size))).toEqual([
      '4.2 MB',
      '4.4 MB',
      '4.9 MB',
      '4.6 MB',
      '4.0 MB',
    ]);
  });

  it('returns null for malformed input', () => {
    expect(parseRelease({ tag_name: 'v1' })).toBeNull();
    expect(parseRelease(null)).toBeNull();
    expect(parseRelease('nope')).toBeNull();
  });
});

describe('getLatestRelease', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('resolves null when fetch rejects', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('offline')));
    await expect(getLatestRelease()).resolves.toBeNull();
  });

  it('resolves null on a non-OK response', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 403 }));
    await expect(getLatestRelease()).resolves.toBeNull();
  });
});
