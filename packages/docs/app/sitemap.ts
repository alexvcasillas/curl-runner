import type { MetadataRoute } from 'next';
import { getAllFeatures } from '@/lib/features-data';
import { SITE_URL, source } from '@/lib/source';

const MARKETING_ROUTES = ['', '/alternatives', '/downloads', '/tools-database', '/changelog'];

export default function sitemap(): MetadataRoute.Sitemap {
  const lastModified = new Date();

  const marketing = MARKETING_ROUTES.map((path) => ({
    url: `${SITE_URL}${path}`,
    lastModified,
    changeFrequency: 'monthly' as const,
    priority: path === '' ? 1 : 0.8,
  }));

  const features = getAllFeatures().map((feature) => ({
    url: `${SITE_URL}/features/${feature.slug}`,
    lastModified,
    changeFrequency: 'monthly' as const,
    priority: 0.8,
  }));

  const docs = source.getPages().map((page) => ({
    url: `${SITE_URL}${page.url}`,
    lastModified,
    changeFrequency: 'monthly' as const,
    priority: page.url === '/docs' ? 0.9 : 0.7,
  }));

  return [...marketing, ...features, ...docs];
}
