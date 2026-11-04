import { act, fireEvent, render as renderDom, screen, within } from '@testing-library/react';
import { prerenderToNodeStream } from 'react-dom/static.node';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const getReleaseDates = vi.fn();
// Returns a Release[] to replace the real CHANGELOG, or undefined to read it.
const releasesOverride = vi.fn();
vi.mock('@/lib/changelog', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/changelog')>();
  return {
    ...actual,
    getReleaseDates: () => getReleaseDates(),
    getReleases: () => releasesOverride() ?? actual.getReleases(),
  };
});

import ChangelogPage, { metadata } from '@/app/(marketing)/changelog/page';
import { GET } from '@/app/rss.xml/route';
import { getReleases, parseChangelog } from '@/lib/changelog';
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

  it('renders every release with exact anchors, marks those past ten as older, then links GitHub', async () => {
    const html = await render();
    const releases = getReleases();
    for (const release of releases) {
      expect(html).toContain(`id="${release.version}"`);
      expect(html).toContain(`href="#${release.version}"`);
    }
    expect(html).not.toMatch(new RegExp(`id="${releases[9].version}"[^>]*data-older`));
    expect(html).toMatch(new RegExp(`id="${releases[10].version}"[^>]*data-older`));
    expect(html).toContain('Older releases on GitHub');
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
    // The notes fold, not the folded item list (`data-more-items`).
    const details = article.indexOf('<details class="group">');
    expect(details).toBeGreaterThan(-1);
    expect(article.indexOf('release-migration')).toBeGreaterThan(-1);
    expect(article.indexOf('release-migration')).toBeLessThan(details);
    expect(article.indexOf('Behavior changes')).toBeGreaterThan(details);
    expect(html).not.toContain('<script');
  });

  it('exports metadata', () => {
    expect(metadata.title).toEqual({ absolute: 'Changelog — curl-runner' });
    expect(metadata.alternates?.canonical).toBe('https://www.curl-runner.com/changelog');
  });
});

const PREFIX = (pr: number, sha: string) =>
  `[#${pr}](https://github.com/alexvcasillas/curl-runner/pull/${pr}) [\`${sha}\`](https://github.com/alexvcasillas/curl-runner/commit/${sha}) Thanks [@alexvcasillas](https://github.com/alexvcasillas)! - `;

// 3 hand-written releases, then 20 generated ones (2.19.0 down to 2.0.0): 23 in all.
const TAGGED_CHANGELOG = [
  '# @curl-runner/cli',
  '',
  '## 3.2.0',
  '',
  '### Minor Changes',
  '',
  `- ${PREFIX(200, 'abc1234')}**Tagged headline**`,
  '',
  '  The tagged lead paragraph.',
  '',
  '  - **New:** A `--thing` flag.',
  '  - **Improved:** Faster runs.',
  '  - **Fixed:** A crash on empty files.',
  ...Array.from({ length: 8 }, (_, n) => `  - **Improved:** Extra ${n + 1}.`),
  '  - **Fixed:** Late fix.',
  '',
  '  ```yaml',
  '  retry: 3',
  '  ```',
  '',
  '## 3.1.1',
  '',
  '### Patch Changes',
  '',
  `- ${PREFIX(199, 'def5678')}**Retry fix**`,
  '',
  '  Lead two.',
  '',
  '  - **Fixed:** Retry counts start at one.',
  '',
  '## 3.1.0',
  '',
  '### Minor Changes',
  '',
  `- ${PREFIX(198, 'aaa1111')}An untagged entry body.`,
  '',
  ...Array.from({ length: 20 }, (_, n) => {
    const minor = 19 - n;
    const tag = minor % 2 === 0 ? 'New' : 'Improved';
    return [
      `## 2.${minor}.0`,
      '',
      '### Minor Changes',
      '',
      `- ${PREFIX(100 + minor, `b${String(minor).padStart(6, '0')}`)}**Release ${minor}**`,
      '',
      `  - **${tag}:** Change ${minor}.`,
      '',
    ].join('\n');
  }),
].join('\n');

const TAGGED_RELEASES = parseChangelog(TAGGED_CHANGELOG);

function release(version: string): HTMLElement {
  const el = document.getElementById(version);
  if (!el) {
    throw new Error(`No release ${version}`);
  }
  return el;
}

describe('changelog page with tagged entries', () => {
  beforeEach(() => {
    getReleaseDates.mockReset();
    getReleaseDates.mockResolvedValue({});
    releasesOverride.mockReturnValue(TAGGED_RELEASES);
    window.history.replaceState(null, '', '/changelog');
  });

  afterEach(() => {
    releasesOverride.mockReset();
  });

  it('renders tagged rows with their label and per-tag color, and the lead', async () => {
    const html = await render();
    expect(html).toContain('The tagged lead paragraph.');
    expect(html).toMatch(/data-tags="new improved fixed"/);
    expect(html).toMatch(/<li[^>]*data-tag="new"[^>]*>.*?<span[^>]*text-brand[^>]*>New<\/span>/);
    expect(html).toMatch(
      /<li[^>]*data-tag="improved"[^>]*>.*?<span[^>]*text-fg-secondary[^>]*>Improved<\/span>/,
    );
    expect(html).toMatch(
      /<li[^>]*data-tag="fixed"[^>]*>.*?<span[^>]*text-yaml-string[^>]*>Fixed<\/span>/,
    );
    expect(html).toContain('A <code>--thing</code> flag.');
    // The notes after the items still render, with Shiki.
    const article = html.slice(html.indexOf('id="3.2.0"'), html.indexOf('id="3.1.1"'));
    expect(article).toContain('class="shiki');
  });

  it('renders an untagged entry as before', async () => {
    const html = await render();
    const article = html.slice(html.indexOf('id="3.1.0"'), html.indexOf('id="2.19.0"'));
    expect(article).toContain('data-tags=""');
    expect(article).toContain('<div class="release-notes"><p>An untagged entry body.</p>');
    expect(article).not.toContain('data-tag=');
  });

  it('shows ten releases, then loads ten more at a time', async () => {
    renderDom(await ChangelogPage());
    expect(release('3.2.0')).toBeVisible();
    expect(release('2.13.0')).toBeVisible(); // 10th
    expect(release('2.12.0')).not.toBeVisible(); // 11th
    fireEvent.click(screen.getByRole('button', { name: 'Load older releases' }));
    expect(release('2.12.0')).toBeVisible();
    expect(release('2.3.0')).toBeVisible(); // 20th
    expect(release('2.2.0')).not.toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Load older releases' }));
    expect(release('2.0.0')).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Load older releases' })).toBeNull();
  });

  it('filters rows, entries and releases by tag and keeps it in the URL', async () => {
    renderDom(await ChangelogPage());
    const filters = screen.getByRole('group', { name: 'Filter changes by tag' });
    fireEvent.click(within(filters).getByRole('button', { name: 'Fixed' }));
    expect(within(filters).getByRole('button', { name: 'Fixed' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    expect(window.location.search).toBe('?tag=fixed');
    const tagged = release('3.2.0');
    expect(tagged).toBeVisible();
    expect(within(tagged).getByText('A crash on empty files.')).toBeVisible();
    expect(within(tagged).getByText('Faster runs.')).not.toBeVisible();
    expect(within(tagged).getByText('The tagged lead paragraph.')).toBeVisible();
    expect(release('3.1.1')).toBeVisible();
    expect(release('3.1.0')).not.toBeVisible();
    expect(release('2.19.0')).not.toBeVisible();
    expect(screen.queryByRole('button', { name: 'Load older releases' })).toBeNull();

    fireEvent.click(within(filters).getByRole('button', { name: 'All' }));
    expect(window.location.search).toBe('');
    expect(within(tagged).getByText('Faster runs.')).toBeVisible();
    expect(release('3.1.0')).toBeVisible();
  });

  it('reads ?tag= on load and says when no change has the tag', async () => {
    window.history.replaceState(null, '', '/changelog?tag=breaking');
    renderDom(await ChangelogPage());
    expect(screen.getByText('No changes with this tag yet.')).toBeVisible();
    expect(release('3.2.0')).not.toBeVisible();
    expect(
      within(screen.getByRole('group', { name: 'Filter changes by tag' })).getByRole('button', {
        name: 'Breaking',
      }),
    ).toHaveAttribute('aria-pressed', 'true');
  });

  it('reveals an older release targeted by the hash, on load and on hashchange', async () => {
    window.history.replaceState(null, '', '/changelog#2.5.0');
    const { unmount } = renderDom(await ChangelogPage());
    expect(release('2.5.0')).toBeVisible();
    unmount();

    window.history.replaceState(null, '', '/changelog');
    renderDom(await ChangelogPage());
    expect(release('2.0.0')).not.toBeVisible();
    act(() => {
      window.history.replaceState(null, '', '/changelog#2.0.0');
      window.dispatchEvent(new HashChangeEvent('hashchange'));
    });
    expect(release('2.0.0')).toBeVisible();
  });

  it('shows the filter as unavailable and every release without JS', async () => {
    const html = await render();
    expect(html).not.toMatch(/\shidden=""/);
    expect(html).not.toContain('Load older releases');
    expect(html).toMatch(/<div role="group" aria-label="Filter changes by tag" class="invisible /);
  });

  it('folds an entry with more than ten items after the first eight', async () => {
    renderDom(await ChangelogPage());
    const tagged = release('3.2.0');
    const more = tagged.querySelector('details[data-more-items]') as HTMLDetailsElement;
    expect(more).not.toBeNull();
    expect(more.open).toBe(false);
    expect(tagged.querySelectorAll('[data-tag]')).toHaveLength(12);
    expect(more.querySelectorAll('[data-tag]')).toHaveLength(4);
    expect(within(more).getByText('Show all 12 changes')).toBeInTheDocument();
    expect(within(tagged).getByText('Extra 5.')).toBeVisible();
    expect(within(tagged).getByText('Late fix.')).not.toBeVisible();
    // Short lists are not folded.
    expect(release('3.1.1').querySelector('details[data-more-items]')).toBeNull();
  });

  it('opens folded items for a filter and restores the fold on All', async () => {
    renderDom(await ChangelogPage());
    const tagged = release('3.2.0');
    const more = tagged.querySelector('details[data-more-items]') as HTMLDetailsElement;
    fireEvent.click(screen.getByRole('button', { name: 'Fixed' }));
    expect(more.open).toBe(true);
    expect(more.querySelector('summary')).not.toBeVisible();
    expect(within(tagged).getByText('Late fix.')).toBeVisible();
    expect(within(tagged).getByText('Extra 8.')).not.toBeVisible();
    // No New item is folded, so the fold hides entirely.
    fireEvent.click(screen.getByRole('button', { name: 'New' }));
    expect(more).not.toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'All' }));
    // jest-dom calls a closed <details> itself not visible, so check the attribute.
    expect(more).not.toHaveAttribute('hidden');
    expect(more.open).toBe(false);
    expect(more.querySelector('summary')).toBeVisible();
    expect(within(tagged).getByText('Late fix.')).not.toBeVisible();

    // A fold the reader opened stays open after filtering.
    more.open = true;
    fireEvent.click(screen.getByRole('button', { name: 'Fixed' }));
    fireEvent.click(screen.getByRole('button', { name: 'All' }));
    expect(more.open).toBe(true);
  });

  it('re-scrolls to a matching hash target once the filter hides releases above it', async () => {
    const scroll = vi.fn();
    Element.prototype.scrollIntoView = scroll;
    try {
      window.history.replaceState(null, '', '/changelog?tag=fixed#3.1.1');
      renderDom(await ChangelogPage());
      expect(release('3.1.0')).not.toBeVisible();
      expect(scroll).toHaveBeenCalled();
      expect(scroll.mock.contexts.at(-1)).toBe(release('3.1.1'));
    } finally {
      // biome-ignore lint/suspicious/noExplicitAny: jsdom has no scrollIntoView; remove the stub
      delete (Element.prototype as any).scrollIntoView;
    }
  });

  it('clears an active filter when the hash targets a release it hides', async () => {
    window.history.replaceState(null, '', '/changelog?tag=fixed#2.0.0');
    renderDom(await ChangelogPage());
    expect(release('2.0.0')).toBeVisible();
    expect(screen.getByRole('button', { name: 'All' })).toHaveAttribute('aria-pressed', 'true');
    expect(window.location.search).toBe('');
    expect(window.location.hash).toBe('#2.0.0');
  });

  it('ignores an unknown ?tag= and removes it from the URL', async () => {
    window.history.replaceState(null, '', '/changelog?tag=bogus');
    renderDom(await ChangelogPage());
    expect(window.location.search).toBe('');
    expect(screen.getByRole('button', { name: 'All' })).toHaveAttribute('aria-pressed', 'true');
    expect(release('3.1.0')).toBeVisible();
  });

  it('follows ?tag= on back and forward', async () => {
    renderDom(await ChangelogPage());
    act(() => {
      window.history.replaceState(null, '', '/changelog?tag=fixed');
      window.dispatchEvent(new PopStateEvent('popstate'));
    });
    expect(screen.getByRole('button', { name: 'Fixed' })).toHaveAttribute('aria-pressed', 'true');
    expect(release('3.1.0')).not.toBeVisible();
    act(() => {
      window.history.replaceState(null, '', '/changelog');
      window.dispatchEvent(new PopStateEvent('popstate'));
    });
    expect(screen.getByRole('button', { name: 'All' })).toHaveAttribute('aria-pressed', 'true');
    expect(release('3.1.0')).toBeVisible();
    act(() => {
      window.history.replaceState(null, '', '/changelog?tag=improved#3.2.0');
      window.dispatchEvent(new HashChangeEvent('hashchange'));
    });
    expect(screen.getByRole('button', { name: 'Improved' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    expect(release('3.1.1')).not.toBeVisible();
  });

  it('pages within the matching releases and resets the page when the tag changes', async () => {
    renderDom(await ChangelogPage());
    // New: 3.2.0 and 2.18.0 down to 2.0.0 (11 releases).
    fireEvent.click(screen.getByRole('button', { name: 'New' }));
    expect(release('2.2.0')).toBeVisible(); // 10th New release
    expect(release('2.0.0')).not.toBeVisible();
    expect(release('2.19.0')).not.toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Load older releases' }));
    expect(release('2.0.0')).toBeVisible();
    expect(release('2.1.0')).not.toBeVisible();
    expect(screen.queryByRole('button', { name: 'Load older releases' })).toBeNull();
    // Improved: 3.2.0 and 2.19.0 down to 2.1.0 (11 releases), back to the first ten.
    fireEvent.click(screen.getByRole('button', { name: 'Improved' }));
    expect(release('2.3.0')).toBeVisible();
    expect(release('2.1.0')).not.toBeVisible();
    expect(screen.getByRole('button', { name: 'Load older releases' })).toBeInTheDocument();
  });

  it('moves focus to the first revealed release and announces the count', async () => {
    renderDom(await ChangelogPage());
    fireEvent.click(screen.getByRole('button', { name: 'Load older releases' }));
    expect(document.activeElement).toBe(
      within(release('2.12.0')).getByRole('link', { name: '2.12.0' }),
    );
    expect(screen.getByText('Showing 20 releases')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Load older releases' }));
    expect(screen.queryByRole('button', { name: 'Load older releases' })).toBeNull();
    expect(document.activeElement).toBe(
      within(release('2.2.0')).getByRole('link', { name: '2.2.0' }),
    );
    expect(screen.getByText('Showing 23 releases')).toBeInTheDocument();
  });
});

describe('renderEntry', () => {
  const base = {
    bump: 'patch' as const,
    headline: null,
    lead: '',
    items: [],
    notes: '',
    pr: null,
    commit: null,
  };

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

  it('renders tagged items inline and only the notes as preview', async () => {
    const out = await renderEntry({
      ...base,
      body: 'Intro **x**\n\n- **New:** use `a:b` <b>y</b>\n\n```yaml\na: 1\n```',
      lead: 'Intro **x**',
      items: [{ tag: 'new', text: 'use `a:b` <b>y</b>' }],
      notes: '```yaml\na: 1\n```',
    });
    expect(out.leadHtml).toContain('<strong>x</strong>');
    expect(out.items).toHaveLength(1);
    expect(out.items[0].tag).toBe('new');
    expect(out.items[0].html).toContain('<code>a:b</code>');
    expect(out.items[0].html).not.toContain('<b>');
    expect(out.items[0].html).toContain('&lt;b&gt;');
    expect(out.previewHtml).toContain('class="shiki');
    expect(out.previewHtml).not.toContain('Intro');
  });

  it('leaves leadHtml and items empty for untagged entries', async () => {
    const out = await renderEntry({ ...base, body: 'Short.' });
    expect(out.leadHtml).toBe('');
    expect(out.items).toEqual([]);
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
      '<description>Install with Homebrew\nNew: Install with brew install alexvcasillas/tap/curl-runner (prebuilt binaries',
    );
    expect(xml).not.toContain('blazing');
  });

  it('lists tagged items as Tag: text lines after the lead', async () => {
    releasesOverride.mockReturnValue(TAGGED_RELEASES);
    try {
      const xml = await (await GET()).text();
      expect(xml).toContain('<title>curl-runner 3.2.0: Tagged headline</title>');
      expect(xml).toContain(
        `<description>The tagged lead paragraph.\nNew: A --thing flag.\nImproved: Faster runs.\nFixed: A crash on empty files.\n${Array.from({ length: 8 }, (_, n) => `Improved: Extra ${n + 1}.`).join('\n')}\nFixed: Late fix.</description>`,
      );
      expect(xml).toContain('<description>An untagged entry body.</description>');
      expect(xml).toContain('<description>Release 18\nNew: Change 18.</description>');
    } finally {
      releasesOverride.mockReset();
    }
  });
});
