import { describe, expect, it, vi } from 'vitest';

vi.mock('@/lib/blog', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/blog')>();
  const [post] = actual.getPosts();
  return {
    ...actual,
    getPosts: () => [
      { ...post, title: 'Tom & "Jerry"', description: 'a <b>bold</b> & "quoted" claim' },
    ],
  };
});

import { GET as getFeed } from '@/app/blog/rss.xml/route';
import { blogPostingJsonLd } from '@/lib/blog-seo';
import { escapeXml } from '@/lib/xml';

describe('escaping', () => {
  it('escapes XML special characters', () => {
    expect(escapeXml(`<a href="x">Tom & Jerry's</a>`)).toBe(
      '&lt;a href=&quot;x&quot;&gt;Tom &amp; Jerry&apos;s&lt;/a&gt;',
    );
  });

  it('escapes titles and descriptions in the blog feed', async () => {
    const xml = await (await getFeed()).text();
    expect(xml).toContain('<title>Tom &amp; &quot;Jerry&quot;</title>');
    expect(xml).toContain('a &lt;b&gt;bold&lt;/b&gt; &amp; &quot;quoted&quot; claim');
  });

  it('keeps JSON-LD from closing its script tag', () => {
    const json = blogPostingJsonLd({
      slug: 'x',
      url: '/blog/x',
      title: '</script><script>alert(1)</script>',
      description: 'd',
      date: '2026-10-31',
      author: 'Alex Casillas',
    });
    expect(json).not.toContain('</script');
    expect(JSON.parse(json).headline).toBe('</script><script>alert(1)</script>');
  });
});
