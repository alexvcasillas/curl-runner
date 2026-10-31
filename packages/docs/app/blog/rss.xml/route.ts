import { getPosts } from '@/lib/blog';
import { SITE_URL } from '@/lib/site';
import { escapeXml } from '@/lib/xml';

export const dynamic = 'force-static';

// Blog posts only; /rss.xml stays the releases feed.
export function GET(): Response {
  const items = getPosts()
    .map((post) => {
      const url = `${SITE_URL}${post.url}`;
      return `    <item>
      <title>${escapeXml(post.title)}</title>
      <link>${url}</link>
      <description>${escapeXml(post.description)}</description>
      <author>${escapeXml(post.author)}</author>
      <guid isPermaLink="true">${url}</guid>
      <pubDate>${new Date(`${post.date}T00:00:00Z`).toUTCString()}</pubDate>
    </item>`;
    })
    .join('\n');

  const rss = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>curl-runner blog</title>
    <description>Writing on API testing, CI and shipping with coding agents, from the people building curl-runner.</description>
    <link>${SITE_URL}/blog</link>
    <language>en-us</language>
    <atom:link href="${SITE_URL}/blog/rss.xml" rel="self" type="application/rss+xml"/>
${items}
  </channel>
</rss>
`;

  return new Response(rss, {
    headers: { 'Content-Type': 'application/rss+xml; charset=utf-8' },
  });
}
