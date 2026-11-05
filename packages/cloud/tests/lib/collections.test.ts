import { describe, expect, it } from 'vitest';
import { commitUrl, shortSha } from '@/lib/collections';
import { absoluteTime, relativeTime } from '@/lib/time';

const git = (
  repo_url: string | null,
  commit: string | null = '3f9a2c1d8e7b6a5f4e3d2c1b0a9f8e7d6c5b4a39',
) => ({
  commit,
  ref: 'main',
  repo_url,
});

describe('commitUrl', () => {
  it('links github.com and gitlab.com over https', () => {
    expect(commitUrl(git('https://github.com/acme/payments'))).toBe(
      'https://github.com/acme/payments/commit/3f9a2c1d8e7b6a5f4e3d2c1b0a9f8e7d6c5b4a39',
    );
    expect(commitUrl(git('https://gitlab.com/acme/group/payments.git'))).toBe(
      'https://gitlab.com/acme/group/payments/commit/3f9a2c1d8e7b6a5f4e3d2c1b0a9f8e7d6c5b4a39',
    );
    expect(commitUrl(git('https://github.com/acme/payments/'))).toMatch(/payments\/commit\//);
  });

  it('is plain text for any other host, scheme or shape', () => {
    expect(commitUrl(git('https://git.example.com/acme/payments'))).toBeNull();
    expect(commitUrl(git('http://github.com/acme/payments'))).toBeNull();
    expect(commitUrl(git('git@github.com:acme/payments.git'))).toBeNull();
    expect(commitUrl(git('https://github.com.evil.example/acme/payments'))).toBeNull();
    expect(commitUrl(git('https://user:pw@github.com/acme/payments'))).toBeNull();
    expect(commitUrl(git('https://github.com/acme'))).toBeNull();
    expect(commitUrl(git('javascript:alert(1)'))).toBeNull();
    expect(commitUrl(git(null))).toBeNull();
    expect(commitUrl(git('https://github.com/acme/payments', null))).toBeNull();
    expect(commitUrl(git('https://github.com/acme/payments', '../x'))).toBeNull();
    expect(commitUrl(null)).toBeNull();
  });
});

describe('shortSha', () => {
  it('keeps 7 characters', () => {
    expect(shortSha('3f9a2c1d8e7b')).toBe('3f9a2c1');
  });
});

describe('time', () => {
  const now = new Date('2026-11-05T15:00:00Z');
  it('says how long ago, then the date', () => {
    expect(relativeTime('2026-11-05T14:59:40Z', now)).toBe('Just now');
    expect(relativeTime('2026-11-05T14:55:00Z', now)).toBe('5 minutes ago');
    expect(relativeTime('2026-11-05T14:00:00Z', now)).toBe('1 hour ago');
    expect(relativeTime('2026-11-05T13:02:00Z', now)).toBe('1 hour ago');
    expect(relativeTime('2026-11-05T01:00:00Z', now)).toBe('14 hours ago');
    expect(relativeTime('2026-11-04T17:40:00Z', now)).toBe('Yesterday');
    expect(relativeTime('2026-11-02T09:15:00Z', now)).toBe('Nov 2');
    expect(relativeTime('2025-10-30T09:15:00Z', now)).toBe('Oct 30, 2025');
  });
  it('gives an exact UTC time for the tooltip', () => {
    expect(absoluteTime('2026-11-05T13:02:00Z')).toBe('Nov 5, 2026, 13:02 UTC');
  });
});
