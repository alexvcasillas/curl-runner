import type { FeatureData } from '@/lib/features-data';
import { getFeatureBySlug } from '@/lib/features-data';
import type { DocsPage } from '@/lib/source';
import { SITE_URL, source } from '@/lib/source';

export interface BreadcrumbItem {
  name: string;
  url: string;
}

/** schema.org BreadcrumbList with factual fields only: name and absolute URL per step. */
export function breadcrumbList(items: BreadcrumbItem[]) {
  return {
    '@context': 'https://schema.org',
    '@type': 'BreadcrumbList',
    itemListElement: items.map((entry, index) => ({
      '@type': 'ListItem',
      position: index + 1,
      name: entry.name,
      item: entry.url,
    })),
  };
}

export function featureBreadcrumbs(feature: Pick<FeatureData, 'slug' | 'title'>) {
  return breadcrumbList([
    { name: 'Home', url: SITE_URL },
    { name: feature.title, url: `${SITE_URL}/features/${feature.slug}` },
  ]);
}

/** Home → Docs → intermediate pages that exist (folders without an index page are skipped) → the page. */
export function docsBreadcrumbs(page: Pick<DocsPage, 'url' | 'slugs' | 'data'>) {
  const items: BreadcrumbItem[] = [
    { name: 'Home', url: SITE_URL },
    { name: 'Docs', url: `${SITE_URL}/docs` },
  ];
  for (let i = 1; i <= page.slugs.length; i++) {
    const slugs = page.slugs.slice(0, i);
    const isLast = i === page.slugs.length;
    const found = isLast ? page : source.getPage(slugs);
    const url = `/docs/${slugs.join('/')}`;
    if (found) {
      items.push({ name: found.data.title, url: `${SITE_URL}${url}` });
    }
  }
  return breadcrumbList(items);
}

/** `/features/<slug>` for `/docs/features/<slug>` pages that have a marketing page, else null. */
export function featureOverviewHref(slugs: string[]): string | null {
  if (slugs.length !== 2 || slugs[0] !== 'features') {
    return null;
  }
  return getFeatureBySlug(slugs[1]) ? `/features/${slugs[1]}` : null;
}
