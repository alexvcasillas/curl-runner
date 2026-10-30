import type { MetadataRoute } from 'next';
import { getAllFeatures } from '@/lib/features-data';
import { lastCommitDates } from '@/lib/git-dates';
import { DOCS_CONTENT_DIR, FEATURE_SOURCES, MARKETING_PAGES } from '@/lib/page-sources';
import { SITE_URL, source } from '@/lib/source';

interface Entry {
  url: string;
  sources: string[];
  priority: number;
}

export default function sitemap(): MetadataRoute.Sitemap {
  const entries: Entry[] = [
    ...MARKETING_PAGES.map(({ path, sources }) => ({
      url: `${SITE_URL}${path}`,
      sources,
      priority: path === '' ? 1 : 0.8,
    })),
    ...getAllFeatures().map((feature) => ({
      url: `${SITE_URL}/features/${feature.slug}`,
      sources: FEATURE_SOURCES,
      priority: 0.8,
    })),
    ...source.getPages().map((page) => ({
      url: `${SITE_URL}${page.url}`,
      sources: [`${DOCS_CONTENT_DIR}/${page.path}`],
      priority: page.url === '/docs' ? 0.9 : 0.7,
    })),
  ];

  // lastmod is the last commit to a page's sources, not the build time: a date that moves
  // on every deploy tells crawlers (and IndexNow) that every page changed.
  const dates = lastCommitDates([...new Set(entries.flatMap((entry) => entry.sources))]);

  return entries.map(({ url, sources, priority }) => {
    const times = sources.flatMap((file) => dates.get(file)?.getTime() ?? []);
    return {
      url,
      ...(times.length > 0 && { lastModified: new Date(Math.max(...times)) }),
      changeFrequency: 'monthly' as const,
      priority,
    };
  });
}
