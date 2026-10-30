import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import { SITE_URL as LIVE_SITE_URL } from '@/lib/source';
import {
  changedSince,
  ENDPOINT,
  INDEXNOW_KEY,
  parseSitemap,
  payload,
  SITE_URL,
  submit,
} from '@/scripts/indexnow.mjs';

// The shape Next.js renders app/sitemap.ts in.
const XML = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
<url>
<loc>https://www.curl-runner.com</loc>
<lastmod>2026-10-28T22:17:56.000Z</lastmod>
<changefreq>monthly</changefreq>
<priority>1</priority>
</url>
<url>
<loc>https://www.curl-runner.com/docs/features/http2</loc>
<lastmod>2026-10-01T10:00:00.000Z</lastmod>
<changefreq>monthly</changefreq>
<priority>0.7</priority>
</url>
<url>
<loc>https://www.curl-runner.com/docs/tutorials</loc>
<changefreq>monthly</changefreq>
<priority>0.7</priority>
</url>
</urlset>`;

describe('indexnow', () => {
  it('serves the key file at the site root', () => {
    const file = readFileSync(join(__dirname, `../../public/${INDEXNOW_KEY}.txt`), 'utf8');
    expect(file.trim()).toBe(INDEXNOW_KEY);
    expect(INDEXNOW_KEY).toMatch(/^[a-f0-9]{8,128}$/);
  });

  it('targets the live site', () => {
    expect(SITE_URL).toBe(LIVE_SITE_URL);
  });

  it('parses locs and lastmods, keeping entries without a lastmod', () => {
    expect(parseSitemap(XML)).toEqual([
      { url: 'https://www.curl-runner.com', lastModified: new Date('2026-10-28T22:17:56Z') },
      {
        url: 'https://www.curl-runner.com/docs/features/http2',
        lastModified: new Date('2026-10-01T10:00:00Z'),
      },
      { url: 'https://www.curl-runner.com/docs/tutorials' },
    ]);
  });

  it('picks only URLs modified after the given date', () => {
    const entries = parseSitemap(XML);
    expect(changedSince(entries, new Date('2026-10-20T00:00:00Z'))).toEqual([
      'https://www.curl-runner.com',
    ]);
    expect(changedSince(entries, new Date('2026-10-28T22:17:56Z'))).toEqual([]);
    expect(changedSince(entries, new Date('2026-01-01T00:00:00Z'))).toHaveLength(2);
  });

  it('builds the IndexNow payload', () => {
    expect(payload(['https://www.curl-runner.com/docs'])).toEqual({
      host: 'www.curl-runner.com',
      key: INDEXNOW_KEY,
      keyLocation: `https://www.curl-runner.com/${INDEXNOW_KEY}.txt`,
      urlList: ['https://www.curl-runner.com/docs'],
    });
  });

  it('caps the payload at 10,000 URLs', () => {
    const urls = Array.from({ length: 10_001 }, (_, i) => `${SITE_URL}/${i}`);
    expect(payload(urls).urlList).toHaveLength(10_000);
  });

  it('posts the payload as JSON', async () => {
    const fetchImpl = vi.fn(async () => new Response(null, { status: 202 }));
    await expect(submit([`${SITE_URL}/docs`], fetchImpl as unknown as typeof fetch)).resolves.toBe(
      202,
    );
    const [url, init] = fetchImpl.mock.calls[0] as unknown as [string, RequestInit];
    expect(url).toBe(ENDPOINT);
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body as string).urlList).toEqual([`${SITE_URL}/docs`]);
  });

  it('throws on an error answer', async () => {
    const fetchImpl = vi.fn(async () => new Response('Invalid key', { status: 403 }));
    await expect(
      submit([`${SITE_URL}/docs`], fetchImpl as unknown as typeof fetch),
    ).rejects.toThrow('403: Invalid key');
  });
});
