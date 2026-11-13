import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/lib/git-dates', () => ({ lastCommitDates: vi.fn(() => new Map()) }));

import sitemap from '@/app/sitemap';
import { GLOBEX, json, PAYMENTS, USERS } from './fixtures';

const BASE = 'https://www.curl-runner.com';
const fetchMock = vi.fn();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal('fetch', fetchMock);
});
afterEach(() => vi.unstubAllGlobals());

describe('sitemap gallery entries', () => {
  it('lists /collections, listed collections and team pages across pages', async () => {
    fetchMock
      .mockImplementationOnce(async () =>
        json({ collections: [PAYMENTS, USERS], next_cursor: 'c2', total: 3, teams: 2 }),
      )
      .mockImplementationOnce(async () =>
        json({ collections: [GLOBEX], next_cursor: null, total: 3, teams: 2 }),
      );
    const urls = (await sitemap()).map((e) => e.url);
    expect(fetchMock.mock.calls[1][0]).toContain('cursor=c2');
    expect(urls).toContain(`${BASE}/collections`);
    expect(urls).toContain(`${BASE}/collections/acme/payments-api`);
    expect(urls).toContain(`${BASE}/collections/globex/orders`);
    expect(urls).toContain(`${BASE}/collections/acme`);
    expect(urls).toContain(`${BASE}/collections/globex`);
    expect(urls.filter((u) => u === `${BASE}/collections/acme`)).toHaveLength(1);
    expect(new Set(urls).size).toBe(urls.length);
  });

  it('still builds the static pages when the API fails', async () => {
    fetchMock.mockRejectedValue(new Error('offline'));
    const urls = (await sitemap()).map((e) => e.url);
    expect(urls).toContain(`${BASE}/collections`);
    expect(urls).toContain(`${BASE}/downloads`);
    expect(urls.some((u) => u.startsWith(`${BASE}/collections/`))).toBe(false);
  });
});
