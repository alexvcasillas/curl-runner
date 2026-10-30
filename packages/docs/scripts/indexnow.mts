// Pings IndexNow (Bing, Yandex, Seznam, Naver, …) with the pages a production deploy changed:
// the live sitemap's URLs whose lastmod is after `--since` (the deployed commit's parent date).
// Plain Node, no dependencies: `node scripts/indexnow.mts --since <ISO date> [--dry-run]`.

import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

export const SITE_URL = 'https://www.curl-runner.com';
/** Served at `${SITE_URL}/${INDEXNOW_KEY}.txt` (public/) to prove we own the host. */
export const INDEXNOW_KEY = 'd7213017d0574809aa42e7ecc4801a52';
export const ENDPOINT = 'https://api.indexnow.org/indexnow';
/** IndexNow's per-request URL cap. */
const MAX_URLS = 10_000;

export interface SitemapEntry {
  url: string;
  lastModified?: Date;
}

export function parseSitemap(xml: string): SitemapEntry[] {
  return [...xml.matchAll(/<url>([\s\S]*?)<\/url>/g)].flatMap(([, body]) => {
    const loc = body.match(/<loc>\s*(.*?)\s*<\/loc>/)?.[1];
    if (!loc) {
      return [];
    }
    const lastmod = body.match(/<lastmod>\s*(.*?)\s*<\/lastmod>/)?.[1];
    return [{ url: loc, ...(lastmod && { lastModified: new Date(lastmod) }) }];
  });
}

/** URLs modified after `since`; entries without a lastmod are not known to have changed. */
export function changedSince(entries: SitemapEntry[], since: Date): string[] {
  return entries
    .filter(({ lastModified }) => lastModified && lastModified.getTime() > since.getTime())
    .map(({ url }) => url);
}

export function payload(urls: string[]) {
  return {
    host: new URL(SITE_URL).host,
    key: INDEXNOW_KEY,
    keyLocation: `${SITE_URL}/${INDEXNOW_KEY}.txt`,
    urlList: urls.slice(0, MAX_URLS),
  };
}

/** Submits `urls`; throws on a non-2xx answer (200 accepted, 202 key pending validation). */
export async function submit(urls: string[], fetchImpl: typeof fetch = fetch): Promise<number> {
  const response = await fetchImpl(ENDPOINT, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json; charset=utf-8' },
    body: JSON.stringify(payload(urls)),
  });
  if (!response.ok) {
    throw new Error(`IndexNow answered ${response.status}: ${await response.text()}`);
  }
  return response.status;
}

async function main(): Promise<void> {
  const { values } = parseArgs({
    options: { since: { type: 'string' }, 'dry-run': { type: 'boolean', default: false } },
  });
  const since = new Date(values.since ?? '');
  if (Number.isNaN(since.getTime())) {
    throw new Error('--since <ISO date> is required');
  }

  const response = await fetch(`${SITE_URL}/sitemap.xml`, { cache: 'no-store' });
  if (!response.ok) {
    throw new Error(`sitemap.xml answered ${response.status}`);
  }
  const urls = changedSince(parseSitemap(await response.text()), since);
  console.log(`${urls.length} URL(s) changed since ${since.toISOString()}`);
  for (const url of urls) {
    console.log(`  ${url}`);
  }
  if (urls.length === 0 || values['dry-run']) {
    return;
  }
  console.log(`IndexNow: ${await submit(urls)}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error: unknown) => {
    console.error(error instanceof Error ? error.message : error);
    process.exit(1);
  });
}
