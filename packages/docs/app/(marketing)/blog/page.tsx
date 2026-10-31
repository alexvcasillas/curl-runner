import type { Metadata } from 'next';
import { Eyebrow } from '@/components/site/Eyebrow';
import { SiteHeader } from '@/components/site/SiteHeader';
import { formatPostDate, getPosts } from '@/lib/blog';
import { pageMetadata } from '@/lib/site';

// Same layout as /changelog: one row per post, newest first.
export const dynamic = 'force-static';

const blogMeta = pageMetadata({
  title: 'Blog',
  description:
    'Writing on API testing, CI and shipping with coding agents, from the people building curl-runner.',
  path: '/blog',
});

export const metadata: Metadata = {
  ...blogMeta,
  alternates: { ...blogMeta.alternates, types: { 'application/rss+xml': '/blog/rss.xml' } },
};

const FOCUS = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand';

export default function BlogIndexPage() {
  const posts = getPosts();

  return (
    <>
      <SiteHeader active="blog" />
      <main id="main-content">
        <section className="mx-auto flex max-w-[1200px] flex-wrap items-end justify-between gap-6 px-6 pt-24 pb-14">
          <div>
            <Eyebrow>Blog</Eyebrow>
            <h1 className="mt-4 mb-0 text-[clamp(40px,6vw,72px)] leading-[1.02] font-bold tracking-[-0.045em] text-fg">
              Notes on testing APIs.
            </h1>
            <p className="mt-5 mb-0 max-w-[38rem] text-lg leading-[1.6] text-fg-secondary">
              Writing on testing, CI and shipping with agents, from the people building curl-runner.
            </p>
          </div>
          <a
            href="/blog/rss.xml"
            className={`inline-flex h-11 items-center gap-2 rounded-[10px] border border-line px-4 text-sm text-fg transition-colors hover:bg-hover ${FOCUS}`}
          >
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
              className="size-4"
            >
              <path d="M4 11a9 9 0 0 1 9 9" />
              <path d="M4 4a16 16 0 0 1 16 16" />
              <circle cx="5" cy="19" r="1" />
            </svg>
            RSS
            <span className="sr-only"> feed of blog posts</span>
          </a>
        </section>

        <section aria-label="Posts" className="mx-auto max-w-[1200px] px-6 pb-24">
          {posts.map((post) => (
            <article
              key={post.slug}
              id={post.slug}
              aria-labelledby={`post-${post.slug}`}
              className="flex scroll-mt-6 flex-wrap gap-x-12 gap-y-4 border-t border-hairline py-12 last-of-type:border-b"
            >
              <div className="flex flex-[0_0_220px] flex-col gap-2.5 text-sm text-fg-tertiary">
                <time dateTime={post.date}>{formatPostDate(post.date)}</time>
                <span>{post.readingMinutes} min read</span>
              </div>

              <div className="flex min-w-0 flex-[999_1_560px] flex-col gap-4">
                <h2
                  id={`post-${post.slug}`}
                  className="m-0 text-[26px] leading-tight font-bold tracking-[-0.02em]"
                >
                  <a href={post.url} className={`text-fg hover:text-white ${FOCUS}`}>
                    {post.title}
                  </a>
                </h2>
                <p className="m-0 max-w-[44rem] text-base leading-[1.65] text-fg-secondary">
                  {post.description}
                </p>
                <a
                  href={post.url}
                  aria-label={`Read “${post.title}”`}
                  className={`self-start text-sm text-fg-secondary underline-offset-2 hover:text-fg hover:underline ${FOCUS}`}
                >
                  Read the post →
                </a>
              </div>
            </article>
          ))}
        </section>
      </main>
    </>
  );
}
