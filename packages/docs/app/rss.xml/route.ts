import { NextResponse } from 'next/server';
import { getReleaseDates, getReleases } from '@/lib/changelog';
import { entrySummary } from '@/lib/changelog-markdown';

// Prerendered from packages/cli/CHANGELOG.md like /changelog (see next.config.ts).
export const dynamic = 'force-static';

const SITE_URL = 'https://www.curl-runner.com';
const FEED_ITEMS = 20;

function escapeXml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}

export async function GET() {
  const releases = getReleases().slice(0, FEED_ITEMS);
  const dates = await getReleaseDates();

  const items = releases
    .map((release) => {
      const url = `${SITE_URL}/changelog#${release.version}`;
      const first = release.entries[0];
      const headline = first?.headline ? entrySummary(first) : null;
      const title = headline
        ? `curl-runner ${release.version}: ${headline}`
        : `curl-runner ${release.version}`;
      const description = release.entries
        .map((entry) => entrySummary({ ...entry, headline: null }))
        .join(' ');
      const date = dates[release.version] ? new Date(dates[release.version]) : null;
      const pubDate =
        date && !Number.isNaN(date.getTime())
          ? `\n      <pubDate>${date.toUTCString()}</pubDate>`
          : '';
      return `    <item>
      <title>${escapeXml(title)}</title>
      <link>${url}</link>
      <description>${escapeXml(description)}</description>
      <guid isPermaLink="true">${url}</guid>${pubDate}
    </item>`;
    })
    .join('\n');

  const rss = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>curl-runner releases</title>
    <description>Release notes for curl-runner, a CLI that runs HTTP requests and API tests described in YAML files.</description>
    <link>${SITE_URL}/changelog</link>
    <language>en-us</language>
    <lastBuildDate>${new Date().toUTCString()}</lastBuildDate>
    <atom:link href="${SITE_URL}/rss.xml" rel="self" type="application/rss+xml"/>
${items}
  </channel>
</rss>
`;

  return new NextResponse(rss, {
    status: 200,
    headers: {
      'Content-Type': 'application/rss+xml; charset=utf-8',
      'Cache-Control': 'public, max-age=3600, stale-while-revalidate=86400',
    },
  });
}
