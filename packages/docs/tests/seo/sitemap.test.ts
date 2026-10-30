import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const dates = new Map<string, Date>();
vi.mock('@/lib/git-dates', () => ({ lastCommitDates: vi.fn(() => dates) }));

import sitemap from '@/app/sitemap';
import { getAllFeatures } from '@/lib/features-data';
import { lastCommitDates } from '@/lib/git-dates';
import { DOCS_CONTENT_DIR, FEATURE_SOURCES, MARKETING_PAGES } from '@/lib/page-sources';
import { source } from '@/lib/source';

const BASE = 'https://www.curl-runner.com';
const PACKAGE_DIR = join(__dirname, '../..');
const urls = sitemap().map((entry) => entry.url);

function entry(url: string) {
  return sitemap().find((e) => e.url === url);
}

describe('sitemap', () => {
  beforeEach(() => dates.clear());

  it('lists the marketing routes', () => {
    for (const path of ['', '/alternatives', '/downloads', '/tools-database', '/changelog']) {
      expect(urls).toContain(`${BASE}${path}`);
    }
  });

  it('lists every feature page', () => {
    const features = getAllFeatures();
    expect(features).toHaveLength(20);
    for (const feature of features) {
      expect(urls).toContain(`${BASE}/features/${feature.slug}`);
    }
  });

  it('lists every docs page from the source', () => {
    const pages = source.getPages();
    expect(pages.length).toBeGreaterThan(0);
    for (const page of pages) {
      expect(urls).toContain(`${BASE}${page.url}`);
    }
  });

  it('has no duplicates', () => {
    expect(new Set(urls).size).toBe(urls.length);
  });

  it('points every page at source files that exist', () => {
    const files = [
      ...MARKETING_PAGES.flatMap((page) => page.sources),
      ...FEATURE_SOURCES,
      ...source.getPages().map((page) => `${DOCS_CONTENT_DIR}/${page.path}`),
    ];
    for (const file of files) {
      expect(existsSync(join(PACKAGE_DIR, file)), file).toBe(true);
    }
  });

  it('asks git for each source once, in one call', () => {
    vi.mocked(lastCommitDates).mockClear();
    sitemap();
    expect(lastCommitDates).toHaveBeenCalledTimes(1);
    const [paths] = vi.mocked(lastCommitDates).mock.calls[0];
    expect(new Set(paths).size).toBe(paths.length);
    expect(paths).toContain('content/docs/features/http2.mdx');
  });

  it('dates a docs page by the last commit to its MDX file', () => {
    dates.set('content/docs/features/http2.mdx', new Date('2026-10-01T10:00:00Z'));
    expect(entry(`${BASE}/docs/features/http2`)?.lastModified).toEqual(
      new Date('2026-10-01T10:00:00Z'),
    );
  });

  it('dates a page by its newest source', () => {
    dates.set('app/(marketing)/changelog/page.tsx', new Date('2026-01-01T00:00:00Z'));
    dates.set('../cli/CHANGELOG.md', new Date('2026-10-28T22:17:56Z'));
    expect(entry(`${BASE}/changelog`)?.lastModified).toEqual(new Date('2026-10-28T22:17:56Z'));
  });

  it('leaves lastModified out when no source has a known date', () => {
    for (const e of sitemap()) {
      expect(e.lastModified, e.url).toBeUndefined();
    }
  });
});
