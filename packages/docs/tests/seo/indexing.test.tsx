import { prerenderToNodeStream } from 'react-dom/static.node';
import { describe, expect, it, vi } from 'vitest';

// DocsPage needs the fumadocs layout context; the template's own output is what is under test.
vi.mock('fumadocs-ui/layouts/docs/page', () => {
  const Plain = ({ children }: { children?: React.ReactNode }) => <div>{children}</div>;
  return { DocsPage: Plain, DocsBody: Plain, DocsTitle: Plain, DocsDescription: Plain };
});

import FeaturePage, {
  generateMetadata as featureMetadata,
} from '@/app/(marketing)/features/[slug]/page';
import { generateMetadata as docsMetadata } from '@/app/docs/[[...slug]]/page';
import { getAllFeatures } from '@/lib/features-data';
import { docsBreadcrumbs, featureBreadcrumbs, featureOverviewHref } from '@/lib/seo';
import { SITE_URL, source } from '@/lib/source';

async function render(element: React.ReactElement): Promise<string> {
  const { prelude } = await prerenderToNodeStream(element);
  const chunks: Buffer[] = [];
  for await (const chunk of prelude) {
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks).toString('utf8');
}

function jsonLd(html: string): Record<string, unknown>[] {
  return [...html.matchAll(/<script type="application\/ld\+json">(.*?)<\/script>/g)].map((m) =>
    JSON.parse(m[1]),
  );
}

describe('feature pages link to their docs page', () => {
  for (const feature of getAllFeatures()) {
    it(feature.slug, async () => {
      const html = await render(<FeaturePage params={Promise.resolve({ slug: feature.slug })} />);
      expect(source.getPage(['features', feature.slug])).toBeDefined();
      expect(html).toContain(`href="/docs/features/${feature.slug}"`);
    });
  }
});

describe('docs feature pages link back to the overview', () => {
  it('only emits links that resolve to a marketing page', () => {
    const slugs = new Set(getAllFeatures().map((f) => f.slug));
    let linked = 0;
    for (const page of source.getPages()) {
      const href = featureOverviewHref(page.slugs);
      if (href === null) {
        continue;
      }
      linked += 1;
      expect(href).toMatch(/^\/features\/[a-z0-9-]+$/);
      expect(slugs.has(href.split('/')[2])).toBe(true);
      expect(page.slugs[0]).toBe('features');
    }
    expect(linked).toBe(getAllFeatures().length);
  });

  it('has no overview link for docs pages without a marketing page', () => {
    expect(featureOverviewHref(['features', 'upgrade'])).toBeNull();
    expect(featureOverviewHref(['installation'])).toBeNull();
    expect(featureOverviewHref([])).toBeNull();
  });
});

describe('breadcrumbs', () => {
  it('feature pages: Home, title (there is no /features index page)', async () => {
    const feature = getAllFeatures()[0];
    const html = await render(<FeaturePage params={Promise.resolve({ slug: feature.slug })} />);
    const crumbs = jsonLd(html).find((d) => d['@type'] === 'BreadcrumbList');
    expect(crumbs).toEqual(featureBreadcrumbs(feature));
    expect(crumbs?.itemListElement).toEqual([
      { '@type': 'ListItem', position: 1, name: 'Home', item: SITE_URL },
      {
        '@type': 'ListItem',
        position: 2,
        name: feature.title,
        item: `${SITE_URL}/features/${feature.slug}`,
      },
    ]);
  });

  it('docs pages: Home, Docs, then real page titles, ending at the page URL', () => {
    const page = source.getPage(['features', 'retry-mechanism']);
    if (!page) {
      throw new Error('missing page');
    }
    const crumbs = docsBreadcrumbs(page);
    const items = crumbs.itemListElement as { name: string; item: string; position: number }[];
    expect(crumbs['@type']).toBe('BreadcrumbList');
    expect(items[0]).toMatchObject({ position: 1, name: 'Home', item: SITE_URL });
    expect(items[1]).toMatchObject({ position: 2, name: 'Docs', item: `${SITE_URL}/docs` });
    expect(items.at(-1)).toMatchObject({ name: page.data.title, item: `${SITE_URL}${page.url}` });
    expect(items.map((i) => i.position)).toEqual(items.map((_, i) => i + 1));
  });

  it('docs index has Home and Docs only', () => {
    const page = source.getPage([]);
    if (!page) {
      throw new Error('missing page');
    }
    const items = docsBreadcrumbs(page).itemListElement as unknown[];
    expect(items).toHaveLength(2);
  });
});

describe('breadcrumb URLs resolve', () => {
  it('every emitted feature breadcrumb item is the home page or a feature page', () => {
    const real = new Set<string>([
      SITE_URL,
      ...getAllFeatures().map((f) => `${SITE_URL}/features/${f.slug}`),
    ]);
    for (const feature of getAllFeatures()) {
      const items = featureBreadcrumbs(feature).itemListElement as {
        item: string;
        position: number;
      }[];
      expect(items.map((i) => i.position)).toEqual(items.map((_, i) => i + 1));
      for (const entry of items) {
        expect(real.has(entry.item), `${feature.slug}: ${entry.item}`).toBe(true);
      }
    }
  });

  it('every emitted docs breadcrumb item is a real page', () => {
    const real = new Set<string>([
      SITE_URL,
      ...source.getPages().map((p) => `${SITE_URL}${p.url}`),
    ]);
    for (const page of source.getPages()) {
      const items = docsBreadcrumbs(page).itemListElement as { item: string; position: number }[];
      expect(items.map((i) => i.position)).toEqual(items.map((_, i) => i + 1));
      for (const entry of items) {
        expect(real.has(entry.item), `${page.url}: ${entry.item}`).toBe(true);
      }
    }
  });
});

describe('canonical URLs', () => {
  it('feature pages canonicalise to themselves', async () => {
    for (const f of getAllFeatures()) {
      const meta = await featureMetadata({ params: Promise.resolve({ slug: f.slug }) });
      expect(meta.alternates?.canonical).toBe(`${SITE_URL}/features/${f.slug}`);
    }
  });

  it('docs pages canonicalise to themselves', async () => {
    for (const page of source.getPages()) {
      const meta = await docsMetadata({ params: Promise.resolve({ slug: page.slugs }) });
      expect(meta.alternates?.canonical).toBe(`${SITE_URL}${page.url}`);
    }
  });
});

describe('docs page template', () => {
  it('renders the back-link and breadcrumbs for a feature docs page', async () => {
    const { default: Page } = await import('@/app/docs/[[...slug]]/page');
    const html = await render(
      await Page({ params: Promise.resolve({ slug: ['features', 'retry-mechanism'] }) }),
    );
    expect(html).toContain('href="/features/retry-mechanism"');
    expect(html).toContain('Feature overview');
    expect(jsonLd(html).some((d) => d['@type'] === 'BreadcrumbList')).toBe(true);
  });

  it('has no back-link on other docs pages', async () => {
    const { default: Page } = await import('@/app/docs/[[...slug]]/page');
    const html = await render(await Page({ params: Promise.resolve({ slug: ['installation'] }) }));
    expect(html).not.toContain('Feature overview');
  });
});
