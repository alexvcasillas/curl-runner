import type { MetadataRoute } from 'next';
import { getPosts } from '@/lib/blog';
import { getAllFeatures } from '@/lib/features-data';
import { fetchAllCollections } from '@/lib/gallery';
import { lastCommitDates } from '@/lib/git-dates';
import {
  BLOG_POST_SOURCES,
  DOCS_CONTENT_DIR,
  FEATURE_SOURCES,
  MARKETING_PAGES,
} from '@/lib/page-sources';
import { SITE_URL, source } from '@/lib/source';

interface Entry {
  url: string;
  sources: string[];
  priority: number;
}

export default async function sitemap(): Promise<MetadataRoute.Sitemap> {
  const posts = getPosts();
  const collections = await fetchAllCollections();
  const entries: Entry[] = [
    ...MARKETING_PAGES.map(({ path, sources }) => ({
      url: `${SITE_URL}${path}`,
      sources: path === '/blog' ? [...sources, ...posts.map((post) => post.file)] : sources,
      priority: path === '' ? 1 : 0.8,
    })),
    ...getAllFeatures().map((feature) => ({
      url: `${SITE_URL}/features/${feature.slug}`,
      sources: FEATURE_SOURCES,
      priority: 0.8,
    })),
    ...posts.map((post) => ({
      url: `${SITE_URL}${post.url}`,
      sources: [post.file, ...BLOG_POST_SOURCES],
      priority: 0.7,
    })),
    ...source.getPages().map((page) => ({
      url: `${SITE_URL}${page.url}`,
      sources: [`${DOCS_CONTENT_DIR}/${page.path}`],
      priority: page.url === '/docs' ? 0.9 : 0.7,
    })),
  ];

  const galleryPages: MetadataRoute.Sitemap = [...new Set(collections.map((c) => c.team.slug))].map(
    (team) => ({
      url: `${SITE_URL}/collections/${team}`,
      changeFrequency: 'weekly' as const,
      priority: 0.6,
    }),
  );
  const galleryEntries: MetadataRoute.Sitemap = collections.map((c) => {
    const updated = new Date(c.updated_at);
    return {
      url: `${SITE_URL}/collections/${c.team.slug}/${c.slug}`,
      ...(!Number.isNaN(updated.getTime()) && { lastModified: updated }),
      changeFrequency: 'weekly' as const,
      priority: 0.6,
    };
  });

  // lastmod is the last commit to a page's sources, not the build time: a date that moves
  // on every deploy tells crawlers (and IndexNow) that every page changed.
  const dates = lastCommitDates([...new Set(entries.flatMap((entry) => entry.sources))]);

  const pages = entries.map(({ url, sources, priority }) => {
    const times = sources.flatMap((file) => dates.get(file)?.getTime() ?? []);
    return {
      url,
      ...(times.length > 0 && { lastModified: new Date(Math.max(...times)) }),
      changeFrequency: 'monthly' as const,
      priority,
    };
  });
  return [...pages, ...galleryPages, ...galleryEntries];
}
