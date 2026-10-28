import { DocsBody, DocsDescription, DocsPage, DocsTitle } from 'fumadocs-ui/layouts/docs/page';
import { createRelativeLink } from 'fumadocs-ui/mdx';
import type { Metadata } from 'next';
import { notFound } from 'next/navigation';
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

  return (
    <DocsPage toc={page.data.toc} full={page.data.full}>
      <DocsTitle>{page.data.title}</DocsTitle>
      <DocsDescription>{page.data.description}</DocsDescription>
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

  return {
    // The index keeps the site-wide docs title instead of "Introduction | curl-runner Documentation".
    title: isIndex ? { absolute: 'curl-runner Documentation' } : page.data.title,
    description: page.data.description,
    alternates: {
      canonical: url,
      types: { 'text/markdown': `${url}.md` },
    },
    openGraph: {
      url,
      type: 'article',
      title: page.data.title,
      description: page.data.description,
      images: [{ url: ogImage, width: 1200, height: 630, alt: page.data.title }],
    },
    twitter: { card: 'summary_large_image', images: [ogImage] },
  };
}
