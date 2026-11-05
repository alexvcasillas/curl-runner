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

describe('studio preview post', () => {
  const STUDIO_SLUG = 'curl-runner-studio-preview';

  it('is listed', () => {
    expect(getPosts().map((p) => p.slug)).toContain(STUDIO_SLUG);
  });

  it('shows the studio screenshots and links to the download and the cask', async () => {
    const out = await html(await postPage(STUDIO_SLUG));
    for (const shot of [
      'collection-run',
      'editor-quick-fix',
      'inspector',
      'failed-request',
      'environments',
      'history',
    ]) {
      expect(out, shot).toMatch(new RegExp(`studio(/|%2F)${shot}\\.png`));
    }
    expect(out).toContain('href="/downloads"');
    const text = out.replace(/<[^>]+>/g, '');
    expect(text).toContain('brew trust alexvcasillas/tap');
    expect(text).toContain('brew install --cask alexvcasillas/tap/curl-runner-studio');
    expect(out).not.toContain('—');
  });
});

describe('2.4 Postman import post', () => {
  const SLUG = 'curl-runner-2-4-postman-import';

  it('is listed', () => {
    expect(getPosts().map((p) => p.slug)).toContain(SLUG);
  });

  it('shows the Postman screenshots and links to the docs, downloads and changelog', async () => {
    const out = await html(await postPage(SLUG));
    for (const shot of ['postman-import', 'postman-import-summary']) {
      expect(out, shot).toMatch(new RegExp(`studio(/|%2F)${shot}\\.png`));
    }
    expect(out).toContain('href="/docs/features/postman-import"');
    expect(out).toContain('href="/downloads"');
    expect(out).toContain('href="/changelog#2.4.0"');
    const text = out.replace(/<[^>]+>/g, '');
    expect(text).toContain('curl-runner convert postman');
    expect(text).toContain('brew trust alexvcasillas/tap');
    expect(out).not.toContain('—');
  });
});

describe('2.5 OpenAPI import post', () => {
  const SLUG = 'curl-runner-2-5-openapi-import';

  it('is listed', () => {
    expect(getPosts().map((p) => p.slug)).toContain(SLUG);
  });

  it('shows the OpenAPI screenshots and links to the docs, downloads and changelog', async () => {
    const out = await html(await postPage(SLUG));
    for (const shot of ['openapi-import', 'openapi-import-summary']) {
      expect(out, shot).toMatch(new RegExp(`studio(/|%2F)${shot}\\.png`));
    }
    expect(out).toContain('href="/docs/features/openapi-import"');
    expect(out).toContain('href="/downloads"');
    expect(out).toContain('href="/changelog#2.5.0"');
    const text = out.replace(/<[^>]+>/g, '');
    expect(text).toContain('curl-runner convert openapi');
    expect(out).not.toContain('—');
  });
});

describe('Cloud private preview post', () => {
  const SLUG = 'introducing-curl-runner-cloud';

  it('is the newest post', () => {
    expect(getPosts()[0]?.slug).toBe(SLUG);
  });

  it('shows the flow diagram and links to the docs, self-hosting and changelog', async () => {
    const out = await html(await postPage(SLUG));
    const text = out.replace(/<[^>]+>/g, '');
    for (const step of ['Your repo', 'CI', 'curl-runner Cloud', 'Your customers']) {
      expect(text, step).toContain(step);
    }
    expect(out).toContain('href="/docs/features/cloud"');
    expect(out).toContain('href="/docs/self-hosting"');
    expect(out).toContain('href="/changelog#2.6.0"');
    expect(text).toContain('curl-runner publish');
    expect(text).toContain('version: 2.6.0');
    // The dashboard isn't live yet, so nothing may link to it.
    expect(out).not.toContain('href="https://cloud.curl-runner.com');
    expect(out).not.toContain('—');
  });
});
