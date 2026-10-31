import { prerenderToNodeStream } from 'react-dom/static.node';
import { describe, expect, it } from 'vitest';
import BlogPostPage, { generateMetadata } from '@/app/(marketing)/blog/[slug]/page';
import { metadata as indexMetadata } from '@/app/(marketing)/blog/page';
import { GET as getFeed } from '@/app/blog/rss.xml/route';
import { GET as getPostMarkdown } from '@/app/llms.blog/[slug]/route';
import { GET as getLlmsTxt } from '@/app/llms.txt/route';
import { GET as getOgImage } from '@/app/og/blog/[slug]/image.png/route';
import sitemap from '@/app/sitemap';

const SITE = 'https://www.curl-runner.com';
const SLUG = 'testing-in-the-agent-era';
const params = (slug: string) => ({ params: Promise.resolve({ slug }) });

async function postHtml(): Promise<string> {
  const { prelude } = await prerenderToNodeStream(await BlogPostPage(params(SLUG)));
  const chunks: Buffer[] = [];
  for await (const chunk of prelude) {
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks).toString('utf8');
}

describe('/blog/rss.xml', () => {
  it('is an RSS feed of the posts', async () => {
    const res = await getFeed();
    expect(res.headers.get('content-type')).toContain('xml');
    const xml = await res.text();
    expect(xml).toContain('<title>curl-runner blog</title>');
    expect(xml).toContain(`<link>${SITE}/blog/${SLUG}</link>`);
    expect(xml).toContain('<pubDate>Sat, 31 Oct 2026 00:00:00 GMT</pubDate>');
    expect(xml).toContain(`<atom:link href="${SITE}/blog/rss.xml"`);
  });
});

describe('sitemap', () => {
  it('lists the blog and each post', () => {
    const urls = sitemap().map((entry) => entry.url);
    expect(urls).toContain(`${SITE}/blog`);
    expect(urls).toContain(`${SITE}/blog/${SLUG}`);
  });
});

describe('llms', () => {
  it('lists posts in llms.txt with their Markdown URLs', async () => {
    const body = await (await getLlmsTxt()).text();
    expect(body).toContain('## Blog');
    expect(body).toContain(`${SITE}/blog/${SLUG}.md`);
  });

  it('serves a post as Markdown', async () => {
    const res = await getPostMarkdown(new Request(`${SITE}/llms.blog/${SLUG}`), params(SLUG));
    expect(res.headers.get('content-type')).toMatch(/^text\/markdown/);
    expect(await res.text()).toMatch(/^# The importance of testing/);
  });

  it('404s on an unknown post', async () => {
    await expect(
      getPostMarkdown(new Request(`${SITE}/llms.blog/nope`), params('nope')),
    ).rejects.toThrow(/NEXT_HTTP_ERROR_FALLBACK;404|NEXT_NOT_FOUND/);
  });
});

describe('post metadata', () => {
  it('is an article with its own Open Graph image and the blog feed', async () => {
    const meta = await generateMetadata(params(SLUG));
    const og = meta.openGraph as Record<string, unknown>;
    expect(og.type).toBe('article');
    expect(og.publishedTime).toBe('2026-10-31');
    expect(og.authors).toEqual(['Alex Casillas']);
    expect(JSON.stringify(og.images)).toContain(`/og/blog/${SLUG}/image.png`);
    expect(JSON.stringify(meta.twitter)).toContain(`/og/blog/${SLUG}/image.png`);
    expect(meta.alternates?.types).toEqual({ 'application/rss+xml': '/blog/rss.xml' });
    expect(indexMetadata.alternates?.types).toEqual({ 'application/rss+xml': '/blog/rss.xml' });
  });

  it('embeds BlogPosting structured data', async () => {
    const html = await postHtml();
    const match = html.match(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/);
    expect(match).not.toBeNull();
    const data = JSON.parse(match?.[1] ?? '{}');
    expect(data['@type']).toBe('BlogPosting');
    expect(data.headline).toBe('The importance of testing in the Agent era');
    expect(data.datePublished).toBe('2026-10-31');
    expect(data.author).toEqual({ '@type': 'Person', name: 'Alex Casillas' });
    expect(data.image).toBe(`${SITE}/og/blog/${SLUG}/image.png`);
    expect(data.mainEntityOfPage).toBe(`${SITE}/blog/${SLUG}`);
  });
});

describe('/og/blog/<slug>/image.png', () => {
  it('404s on an unknown post', async () => {
    await expect(
      getOgImage(new Request(`${SITE}/og/blog/nope/image.png`), params('nope')),
    ).rejects.toThrow(/NEXT_HTTP_ERROR_FALLBACK;404|NEXT_NOT_FOUND/);
  });
});
