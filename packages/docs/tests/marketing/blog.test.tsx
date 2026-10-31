import { render, screen } from '@testing-library/react';
import type { ReactElement } from 'react';
import { prerenderToNodeStream } from 'react-dom/static.node';
import { describe, expect, it } from 'vitest';
import BlogPostPage, { generateStaticParams } from '@/app/(marketing)/blog/[slug]/page';
import BlogIndexPage from '@/app/(marketing)/blog/page';
import { SiteHeader } from '@/components/site/SiteHeader';
import { getPosts } from '@/lib/blog';

const SLUG = 'testing-in-the-agent-era';

async function html(element: ReactElement): Promise<string> {
  const { prelude } = await prerenderToNodeStream(element);
  const chunks: Buffer[] = [];
  for await (const chunk of prelude) {
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks).toString('utf8');
}

const postPage = (slug: string) => BlogPostPage({ params: Promise.resolve({ slug }) });

describe('blog index', () => {
  it('lists each post with a linked title, excerpt, date and reading time', async () => {
    const out = await html(BlogIndexPage());
    const post = getPosts().find((p) => p.slug === SLUG);
    expect(out).toContain('>Blog<');
    expect(out).toContain(`<article id="${SLUG}"`);
    expect(out).toMatch(
      new RegExp(`<h2[^>]*><a[^>]*href="/blog/${SLUG}"[^>]*>The importance of testing`),
    );
    expect(out).toContain(post?.description.replace(/'/g, '&#x27;'));
    expect(out).toMatch(/<time[^>]*dateTime="2026-10-31"[^>]*>October 31, 2026<\/time>/i);
    expect(out).toContain('min read');
    expect(out).toContain('href="/blog/rss.xml"');
  });
});

describe('blog post page', () => {
  it('renders the title, byline, back link and the call to action', async () => {
    const out = await html(await postPage(SLUG));
    expect(out).toMatch(/<h1[^>]*>The importance of testing in the Agent era<\/h1>/);
    expect(out).toContain('Alex Casillas');
    expect(out).toMatch(/<time[^>]*dateTime="2026-10-31"/i);
    expect(out).toContain('min read');
    expect(out).toContain('href="/blog"');
    expect(out).toContain('href="/docs/quick-start"');
    expect(out).toContain('href="/docs/features/ci-integration"');
  });

  it('is prerendered once per post', () => {
    expect(generateStaticParams()).toContainEqual({ slug: SLUG });
  });

  it('404s on an unknown slug', async () => {
    await expect(postPage('nope')).rejects.toThrow(/NEXT_HTTP_ERROR_FALLBACK;404|NEXT_NOT_FOUND/);
  });
});

describe('header', () => {
  it('links to the blog and marks it current', () => {
    render(<SiteHeader active="blog" />);
    expect(screen.getByRole('link', { name: 'Blog' })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('link', { name: 'Blog' })).toHaveAttribute('href', '/blog');
  });
});
