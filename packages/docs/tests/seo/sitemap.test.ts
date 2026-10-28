import { describe, expect, it } from 'vitest';
import sitemap from '@/app/sitemap';
import { getAllFeatures } from '@/lib/features-data';
import { source } from '@/lib/source';

const BASE = 'https://www.curl-runner.com';
const urls = sitemap().map((entry) => entry.url);

describe('sitemap', () => {
  it('lists the marketing routes', () => {
    for (const path of ['', '/alternatives', '/downloads', '/tools-database', '/changelog']) {
      expect(urls).toContain(`${BASE}${path}`);
    }
  });

  it('lists every feature page', () => {
    const features = getAllFeatures();
    expect(features).toHaveLength(19);
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
});
