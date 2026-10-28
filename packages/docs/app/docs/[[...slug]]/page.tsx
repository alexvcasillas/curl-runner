import { DocsBody, DocsDescription, DocsPage, DocsTitle } from 'fumadocs-ui/layouts/docs/page';
import { createRelativeLink } from 'fumadocs-ui/mdx';
import type { Metadata } from 'next';
import Link from 'next/link';
import { notFound } from 'next/navigation';
import { StructuredData } from '@/components/structured-data';
import { docsBreadcrumbs, featureOverviewHref } from '@/lib/seo';
import { getDocsOgUrl, SITE_URL, source } from '@/lib/source';
import { getMDXComponents } from '@/mdx-components';

type Props = { params: Promise<{ slug?: string[] }> };

export const dynamicParams = false;

export default async function Page({ params }: Props) {
  const { slug } = await params;
  const page = source.getPage(slug);
  if (!page) {
    notFound();
  }

  const MDX = page.data.body;
  const overviewHref = featureOverviewHref(page.slugs);

  return (
    <DocsPage toc={page.data.toc} full={page.data.full}>
      <StructuredData data={docsBreadcrumbs(page)} />
      <DocsTitle>{page.data.title}</DocsTitle>
      <DocsDescription>{page.data.description}</DocsDescription>
      {overviewHref ? (
        <Link
          href={overviewHref}
          className="not-prose -mt-2 mb-4 inline-block text-sm text-fd-muted-foreground underline-offset-4 hover:text-fd-foreground hover:underline"
        >
          Feature overview →
        </Link>
      ) : null}
      <DocsBody>
        <MDX components={getMDXComponents({ a: createRelativeLink(source, page) })} />
      </DocsBody>
    </DocsPage>
  );
}

export function generateStaticParams() {
  return source.generateParams();
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { slug } = await params;
  const page = source.getPage(slug);
  if (!page) {
    notFound();
  }

  const url = `${SITE_URL}${page.url}`;
  const isIndex = page.slugs.length === 0;
  const ogImage = getDocsOgUrl(page);
  const docsTitle = isIndex ? 'curl-runner docs' : `${page.data.title} — curl-runner docs`;

  return {
    title: { absolute: docsTitle },
    description: page.data.description,
    alternates: {
      canonical: url,
      types: { 'text/markdown': `${url}.md` },
    },
    openGraph: {
      url,
      type: 'article',
      siteName: 'curl-runner',
      title: docsTitle,
      description: page.data.description,
      images: [{ url: ogImage, width: 1200, height: 630, alt: page.data.title }],
    },
    twitter: { card: 'summary_large_image', title: docsTitle, images: [ogImage] },
  };
}
