import type { Metadata } from 'next';
import { notFound } from 'next/navigation';
import { SiteHeader } from '@/components/site/SiteHeader';
import { formatPostDate, getPost, getPosts } from '@/lib/blog';
import { blogPostingJsonLd, getBlogOgUrl } from '@/lib/blog-seo';
import { INSTALL_METHODS } from '@/lib/install-methods';
import { pageMetadata } from '@/lib/site';
import { getMDXComponents } from '@/mdx-components';

export const dynamicParams = false;

type Props = { params: Promise<{ slug: string }> };

const FOCUS = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand';

const LINK = `text-fg underline underline-offset-2 hover:text-white ${FOCUS}`;

const INSTALL = INSTALL_METHODS.find((method) => method.id === 'curl')?.command;

export function generateStaticParams() {
  return getPosts().map((post) => ({ slug: post.slug }));
}

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const post = getPost((await params).slug);
  if (!post) {
    notFound();
  }
  const meta = pageMetadata({
    title: post.title,
    description: post.description,
    path: post.url,
    ownImage: true,
  });
  const images = [{ url: getBlogOgUrl(post), width: 1200, height: 630, alt: post.title }];
  return {
    ...meta,
    authors: [{ name: post.author }],
    alternates: { ...meta.alternates, types: { 'application/rss+xml': '/blog/rss.xml' } },
    openGraph: {
      ...meta.openGraph,
      type: 'article',
      publishedTime: post.date,
      modifiedTime: post.updated ?? post.date,
      authors: [post.author],
      images,
    },
    twitter: { ...meta.twitter, images },
  };
}

export default async function BlogPostPage({ params }: Props) {
  const post = getPost((await params).slug);
  if (!post) {
    notFound();
  }
  const Body = post.body;

  return (
    <>
      <SiteHeader active="blog" />
      <main id="main-content" className="mx-auto max-w-[768px] px-6 pt-20 pb-24">
        <script
          type="application/ld+json"
          // biome-ignore lint/security/noDangerouslySetInnerHtml: JSON built from the post's own frontmatter, with < escaped
          dangerouslySetInnerHTML={{ __html: blogPostingJsonLd(post) }}
        />
        <a href="/blog" className={`text-sm text-fg-secondary hover:text-fg ${FOCUS}`}>
          ← Blog
        </a>
        <h1 className="mt-6 mb-0 text-[clamp(34px,5vw,52px)] leading-[1.08] font-bold tracking-[-0.035em] text-fg">
          {post.title}
        </h1>
        <p className="mt-5 mb-0 text-sm text-fg-tertiary">
          {post.author} · <time dateTime={post.date}>{formatPostDate(post.date)}</time> ·{' '}
          {post.readingMinutes} min read
        </p>

        <article className="blog-prose mt-12">
          <Body components={getMDXComponents()} />
        </article>

        <aside
          aria-label="Try curl-runner"
          className="mt-16 flex flex-col gap-4 rounded-2xl border border-line bg-surface p-6"
        >
          <h2 className="m-0 text-xl font-bold tracking-[-0.02em] text-fg">
            Write your first smoke test
          </h2>
          {INSTALL ? (
            <pre className="m-0 overflow-x-auto rounded-[10px] border border-hairline bg-ground px-4 py-3 font-mono text-sm text-fg">
              <code>{INSTALL}</code>
            </pre>
          ) : null}
          <p className="m-0 text-[15px] leading-[1.6] text-fg-secondary">
            Start with the{' '}
            <a href="/docs/quick-start" className={LINK}>
              quick start
            </a>
            , then put it in your pipeline with{' '}
            <a href="/docs/features/ci-integration" className={LINK}>
              CI integration
            </a>
            .
          </p>
        </aside>
      </main>
    </>
  );
}
