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
    { name: 'Features', url: `${SITE_URL}/features` },
    { name: feature.title, url: `${SITE_URL}/features/${feature.slug}` },
  ]);
}

interface TreeNode {
  type: string;
  name?: unknown;
  url?: string;
  children?: TreeNode[];
}

/** Title of the page-tree folder whose pages live under `url/`. */
function folderName(nodes: TreeNode[], url: string): string | null {
  for (const node of nodes) {
    if (node.type !== 'folder') {
      continue;
    }
    if (typeof node.name === 'string' && node.children?.some((c) => c.url?.startsWith(`${url}/`))) {
      return node.name;
    }
    const nested = node.children ? folderName(node.children, url) : null;
    if (nested) {
      return nested;
    }
  }
  return null;
}

/** Home → Docs → intermediate sections (real titles) → the page. */
export function docsBreadcrumbs(page: Pick<DocsPage, 'url' | 'slugs' | 'data'>) {
  const items: BreadcrumbItem[] = [
    { name: 'Home', url: SITE_URL },
    { name: 'Docs', url: `${SITE_URL}/docs` },
  ];
  const tree = (source.pageTree as unknown as { children: TreeNode[] }).children;
  for (let i = 1; i <= page.slugs.length; i++) {
    const slugs = page.slugs.slice(0, i);
    const isLast = i === page.slugs.length;
    const found = isLast ? page : source.getPage(slugs);
    const url = `/docs/${slugs.join('/')}`;
    const name = found ? found.data.title : folderName(tree, url);
    if (name) {
      items.push({ name, url: `${SITE_URL}${url}` });
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
