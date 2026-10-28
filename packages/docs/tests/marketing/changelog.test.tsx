import { prerenderToNodeStream } from 'react-dom/static.node';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const getReleaseDates = vi.fn();
vi.mock('@/lib/changelog', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/changelog')>()),
  getReleaseDates: () => getReleaseDates(),
}));

import ChangelogPage, { metadata } from '@/app/(marketing)/changelog/page';
import { GET } from '@/app/rss.xml/route';
import { getReleases } from '@/lib/changelog';
import { entrySummary, renderEntry } from '@/lib/changelog-markdown';

async function render(): Promise<string> {
  const { prelude } = await prerenderToNodeStream(await ChangelogPage());
  const chunks: Buffer[] = [];
  for await (const chunk of prelude) {
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks).toString('utf8');
}

const DATES = {
  '2.1.0': '2026-10-28T13:42:19Z',
  '2.0.0': '2026-10-28T09:00:00Z',
  '1.25.0': '2026-06-10T09:00:00Z',
};

describe('changelog page', () => {
  beforeEach(() => {
    getReleaseDates.mockReset();
    getReleaseDates.mockResolvedValue(DATES);
  });

  it('renders the newest ten releases with exact anchors, then links older releases', async () => {
    const html = await render();
    const releases = getReleases();
    for (const release of releases.slice(0, 10)) {
      expect(html).toContain(`id="${release.version}"`);
      expect(html).toContain(`href="#${release.version}"`);
    }
    expect(html).not.toContain(`id="${releases[10].version}"`);
    expect(html).toContain('Older releases');
    expect(html).toContain('href="https://github.com/alexvcasillas/curl-runner/releases"');
  });

  it('shows headlines, dates when known, and bump badges', async () => {
    const html = await render();
    expect(html).toContain('What’s new in curl-runner.');
    expect(html).toContain('Install with Homebrew');
    expect(html).toContain('Rewritten in Rust, on libcurl');
    expect(html).toContain('<code>$absent</code> and <code>$exists</code> assertions');
    expect(html).toMatch(/<time datetime="2026-10-28T13:42:19Z"[^>]*>October 28, 2026<\/time>/i);
    expect(html).toContain('June 10, 2026');
    expect(html).toMatch(/>Major</);
    expect(html).toMatch(/>Minor</);
    expect(html).toMatch(/>Patch</);
  });

  it('renders without dates when the API is unavailable', async () => {
    getReleaseDates.mockResolvedValue({});
    const html = await render();
    expect(html).not.toContain('<time');
    expect(html).toContain('id="2.0.0"');
  });

  it('links each entry to its PR and commit', async () => {
    const html = await render();
    expect(html).toContain('href="https://github.com/alexvcasillas/curl-runner/pull/127"');
    expect(html).toContain('href="https://github.com/alexvcasillas/curl-runner/commit/bec01c6"');
  });

  it('collapses the long 2.0.0 entry behind native details, keeping the migration box visible', async () => {
    const html = await render();
    const article = html.slice(html.indexOf('id="2.0.0"'), html.indexOf('id="1.25.0"'));
    expect(article).toContain('<details');
    expect(article).toContain('Show all 2.0.0 changes');
    const details = article.indexOf('<details');
    expect(article.indexOf('release-migration')).toBeGreaterThan(-1);
    expect(article.indexOf('release-migration')).toBeLessThan(details);
    expect(article.indexOf('Install changes')).toBeGreaterThan(details);
    expect(html).not.toContain('<script');
  });

  it('exports metadata', () => {
    expect(metadata.title).toEqual({ absolute: 'Changelog — curl-runner' });
    expect(metadata.alternates?.canonical).toBe('/changelog');
  });
});

describe('renderEntry', () => {
  const base = { bump: 'patch' as const, headline: null, pr: null, commit: null };

  it('escapes raw HTML instead of rendering it', async () => {
    const out = await renderEntry({ ...base, body: 'Hi <script>alert(1)</script> <b>x</b>' });
    expect(out.previewHtml).not.toContain('<script>');
    expect(out.previewHtml).not.toContain('<b>');
    expect(out.previewHtml).toContain('&lt;script&gt;');
  });

  it('highlights fenced code with Shiki and shifts headings below the headline', async () => {
    const out = await renderEntry({
      ...base,
      body: '### Example\n\n```yaml\nretry:\n  count: 3\n```',
    });
    expect(out.previewHtml).toContain('<h4>Example</h4>');
    expect(out.previewHtml).toContain('class="shiki');
    expect(out.restHtml).toBeNull();
  });

  it('does not collapse short entries', async () => {
    const out = await renderEntry({ ...base, body: 'Short.' });
    expect(out.restHtml).toBeNull();
  });

  it('summarises entries as plain text', () => {
    expect(
      entrySummary({ ...base, headline: '`$absent` and `$exists` assertions', body: '' }),
    ).toBe('$absent and $exists assertions');
    expect(entrySummary({ ...base, body: 'Use **bold** [link](https://x.y)\n\nMore' })).toBe(
      'Use bold link',
    );
  });
});

describe('rss.xml', () => {
  beforeEach(() => {
    getReleaseDates.mockReset();
    getReleaseDates.mockResolvedValue(DATES);
  });

  it('has one item per release (newest 20) linking to the changelog anchor', async () => {
    const xml = await (await GET()).text();
    const items = xml.match(/<item>/g) ?? [];
    expect(items).toHaveLength(Math.min(20, getReleases().length));
    expect(xml).toContain('<link>https://www.curl-runner.com/changelog#2.1.0</link>');
    expect(xml).toContain('<title>curl-runner 2.1.0: Install with Homebrew</title>');
    expect(xml).toContain('<pubDate>Wed, 28 Oct 2026 13:42:19 GMT</pubDate>');
    expect(xml).toContain(
      '<description>Homebrew: install with brew install alexvcasillas/tap/curl-runner (prebuilt binaries',
    );
    expect(xml).not.toContain('blazing');
  });
});
