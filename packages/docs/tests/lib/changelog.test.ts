import { readFileSync } from 'node:fs';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('next/cache', () => ({
  unstable_cache: <T extends (...args: never[]) => unknown>(fn: T) => fn,
}));

const gitState = vi.hoisted(() => ({ fail: false }));

vi.mock('node:child_process', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:child_process')>();
  return {
    ...actual,
    execFileSync: ((...args: Parameters<typeof actual.execFileSync>) => {
      if (gitState.fail) {
        throw new Error('git: not a repository');
      }
      return actual.execFileSync(...args);
    }) as typeof actual.execFileSync,
  };
});

import { CHANGELOG_PATH, getReleaseDates, parseChangelog, splitEntry } from '@/lib/changelog';

const real = readFileSync(CHANGELOG_PATH, 'utf8');
const headingCount = real.match(/^## \d+\.\d+\.\d+\s*$/gm)?.length ?? 0;
// The newest heading moves with every release; read it rather than pin it.
const newest = parseChangelog(real)[0].version;

describe('parseChangelog on packages/cli/CHANGELOG.md', () => {
  const releases = parseChangelog(real);

  it('has one release per `## x.y.z` heading, newest first', () => {
    expect(headingCount).toBeGreaterThan(30);
    expect(releases).toHaveLength(headingCount);
    expect(releases.at(-1)?.version).toBe('1.0.1');
  });

  it('reads 2.1.0 as a minor release with a headline', () => {
    const release = releases.find((r) => r.version === '2.1.0');
    expect(release?.bump).toBe('minor');
    expect(release?.entries[0].headline).toBe('Install with Homebrew');
  });

  it('reads 2.0.0 as a major release with its PR and short commit', () => {
    const release = releases.find((r) => r.version === '2.0.0');
    expect(release?.bump).toBe('major');
    expect(release?.entries[0].pr).toBe(127);
    expect(release?.entries[0].commit).toBe('bec01c6');
    expect(release?.entries[0].headline).toBe('Rewritten in Rust, on libcurl');
    expect(release?.entries[0].body).toContain('### Migration: timeouts are now milliseconds');
  });

  it('strips the changesets prefix and dedents continuation lines', () => {
    for (const release of releases) {
      expect(release.entries.length).toBeGreaterThan(0);
      for (const entry of release.entries) {
        expect(entry.body).not.toMatch(/^\[#/);
        expect(entry.body).not.toMatch(/^Thanks/);
        expect(entry.body).not.toMatch(/^\*\*/);
      }
    }
    const body = releases.find((r) => r.version === '2.0.0')?.entries[0].body ?? '';
    expect(body).toMatch(/^### Migration: timeouts are now milliseconds$/m);
    expect(body).toMatch(/^- \*\*Improved:\*\* The CLI is now a native Rust binary/m);
  });

  it('keeps every entry of a multi-entry release', () => {
    const release = releases.find((r) => r.version === '1.0.2');
    expect(release?.entries.map((e) => e.pr)).toEqual([8, 7]);
  });

  it('keeps fenced code blocks intact', () => {
    const release = releases.find((r) => r.version === '1.8.0');
    expect(release?.entries[0].body).toContain('```yaml\nretry:\n  count: 3');
  });
});

describe('parseChangelog on inline fixtures', () => {
  const fixture = `# pkg

## 3.0.0

### Minor Changes

- [#9](https://github.com/o/r/pull/9) [\`abcdef1234\`](https://github.com/o/r/commit/abcdef1234) Thanks [@u](https://github.com/u)! - **New thing**

  First paragraph.

  Second paragraph
  continues here.

### Patch Changes

- Fix something small without a PR prefix

## 2.9.9

### Patch Changes

- [#8](https://github.com/o/r/pull/8) [\`1234567890\`](https://github.com/o/r/commit/1234567890) Thanks [@u](https://github.com/u)! - Plain summary
`;
  const releases = parseChangelog(fixture);

  it('takes the highest bump across sections', () => {
    expect(releases).toHaveLength(2);
    expect(releases[0].bump).toBe('minor');
    expect(releases[0].entries).toHaveLength(2);
    expect(releases[0].entries.map((e) => e.bump)).toEqual(['minor', 'patch']);
    expect(releases[1].bump).toBe('patch');
  });

  it('reads a leading bold headline and keeps paragraphs', () => {
    const entry = releases[0].entries[0];
    expect(entry.headline).toBe('New thing');
    expect(entry.pr).toBe(9);
    expect(entry.commit).toBe('abcdef1');
    expect(entry.body).toBe('First paragraph.\n\nSecond paragraph\ncontinues here.');
  });

  it('handles entries without a prefix or headline', () => {
    const bare = releases[0].entries[1];
    expect(bare).toEqual({
      bump: 'patch',
      headline: null,
      body: 'Fix something small without a PR prefix',
      lead: 'Fix something small without a PR prefix',
      items: [],
      notes: '',
      pr: null,
      commit: null,
    });
    expect(releases[1].entries[0].headline).toBeNull();
    expect(releases[1].entries[0].body).toBe('Plain summary');
  });

  it('keeps a release whose sections are empty or unknown', () => {
    const odd = parseChangelog('## 1.0.0\n\nSome loose note.\n\n## 0.9.0\n');
    expect(odd.map((r) => r.version)).toEqual(['1.0.0', '0.9.0']);
    expect(odd[0].entries).toHaveLength(1);
    expect(odd[0].entries[0].body).toBe('Some loose note.');
    expect(odd[1].entries).toEqual([]);
    expect(odd[1].bump).toBe('patch');
  });
});

describe('getReleaseDates', () => {
  beforeEach(() => {
    gitState.fail = false;
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('resolves {} when fetch rejects', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('offline')));
    await expect(getReleaseDates()).resolves.toEqual({});
  });

  it('resolves {} on a non-OK response', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 403 }));
    await expect(getReleaseDates()).resolves.toEqual({});
  });

  const apiPayload = [
    { tag_name: '@curl-runner/cli@2.0.0', published_at: '2026-10-28T10:00:00Z' },
    { tag_name: '@curl-runner/cli@1.25.0', published_at: '2026-06-10T09:00:00Z' },
    { tag_name: 'docs-v1', published_at: '2026-01-01T00:00:00Z' },
    { tag_name: '@curl-runner/cli@1.24.0', published_at: null },
  ];

  it('maps cli tags to versions and dates the newest release from git', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => apiPayload }),
    );
    const dates = await getReleaseDates();
    expect(dates['2.0.0']).toBe('2026-10-28T10:00:00Z');
    expect(dates['1.25.0']).toBe('2026-06-10T09:00:00Z');
    expect(dates['1.24.0']).toBeUndefined();
    expect(dates['1.23.5']).toBeUndefined();
    expect(Object.keys(dates)).not.toContain('docs-v1');
    // The newest heading is absent from the API, so git dates it.
    expect(Number.isNaN(Date.parse(dates[newest]))).toBe(false);
  });

  it('falls back to the build time when git is unavailable', async () => {
    gitState.fail = true;
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => apiPayload }),
    );
    const before = Date.now();
    const dates = await getReleaseDates();
    expect(Date.parse(dates[newest])).toBeGreaterThanOrEqual(before - 1000);
  });
});

describe('splitEntry scan', () => {
  it('treats a tagged marker without text as untagged', () => {
    expect(splitEntry('Lead.\n\n- **New:**').items).toEqual([]);
    expect(splitEntry('- **New:**   \n- **Fixed:** b').items).toEqual([
      { tag: 'fixed', text: 'b' },
    ]);
  });

  it('keeps a long fence open until a closing fence of the same char and length', () => {
    const body = '````md\n```\n- **New:** inside\n```\n````\n\n- **Fixed:** real';
    expect(splitEntry(body).items).toEqual([{ tag: 'fixed', text: 'real' }]);
    const tilde = '~~~~\n~~~\n- **New:** inside\n~~~~\n- **Fixed:** real';
    expect(splitEntry(tilde).items).toEqual([{ tag: 'fixed', text: 'real' }]);
  });

  it('ignores tagged lines inside a multi-line HTML comment', () => {
    const body = 'Lead.\n\n<!--\n- **New:** hidden\n-->\n\n- **Fixed:** shown';
    expect(splitEntry(body).items).toEqual([{ tag: 'fixed', text: 'shown' }]);
    expect(splitEntry('<!--\n- **New:** hidden\n-->').items).toEqual([]);
  });
});

describe('splitEntry', () => {
  it('splits lead, tagged items and notes', () => {
    const body = [
      'Intro line.',
      '',
      '- **New:** one',
      '- **Improved:** two',
      '- **Fixed:** three',
      '',
      '```yaml',
      'a: 1',
      '```',
    ].join('\n');
    const result = splitEntry(body);
    expect(result.lead).toBe('Intro line.');
    expect(result.items).toEqual([
      { tag: 'new', text: 'one' },
      { tag: 'improved', text: 'two' },
      { tag: 'fixed', text: 'three' },
    ]);
    expect(result.notes).toBe('```yaml\na: 1\n```');
  });

  it('joins a wrapped item continuation into one item', () => {
    const result = splitEntry('- **New:** first line\n  second line\n- **Fixed:** other');
    expect(result.items).toEqual([
      { tag: 'new', text: 'first line second line' },
      { tag: 'fixed', text: 'other' },
    ]);
  });

  it('keeps colons inside the item text', () => {
    expect(splitEntry('- **New:** use `a:b` here').items[0].text).toBe('use `a:b` here');
  });

  it('reads a lowercase label', () => {
    expect(splitEntry('- **fixed:** x').items[0].tag).toBe('fixed');
  });

  it('returns the whole body as the lead when there are no items', () => {
    expect(splitEntry('Just text.\n\n- plain bullet')).toEqual({
      lead: 'Just text.\n\n- plain bullet',
      items: [],
      notes: '',
    });
  });

  it('keeps an untagged bullet before the first tagged item in the lead', () => {
    const result = splitEntry('Intro\n\n- plain bullet\n\n- **New:** x');
    expect(result.lead).toBe('Intro\n\n- plain bullet');
    expect(result.items).toEqual([{ tag: 'new', text: 'x' }]);
  });

  it('keeps blank lines between tagged items inside the run', () => {
    const result = splitEntry('- **New:** a\n\n- **Fixed:** b\n\nNotes here.');
    expect(result.items.map((i) => i.tag)).toEqual(['new', 'fixed']);
    expect(result.notes).toBe('Notes here.');
  });

  it('sends a trailing untagged list to the notes', () => {
    const result = splitEntry('- **New:** a\n\n- step 1\n- step 2');
    expect(result.items).toEqual([{ tag: 'new', text: 'a' }]);
    expect(result.notes).toBe('- step 1\n- step 2');
  });

  it('ends the run at an untagged bullet between tagged items', () => {
    const result = splitEntry('- **New:** a\n- plain\n- **Fixed:** b');
    expect(result.items).toEqual([{ tag: 'new', text: 'a' }]);
    expect(result.notes).toBe('- plain\n- **Fixed:** b');
  });

  it('ends the run at an unindented or indented fence', () => {
    expect(splitEntry('- **New:** a\n```sh\nx\n```').notes).toBe('```sh\nx\n```');
    const indented = splitEntry('- **New:** a\n\n  ```sh\n  x\n  ```');
    expect(indented.items).toEqual([{ tag: 'new', text: 'a' }]);
    expect(indented.notes).toContain('```sh');
  });

  it('ends the run at a heading with no blank line before it', () => {
    const result = splitEntry('- **New:** a\n### Migration: x\n\nDo this.');
    expect(result.items).toEqual([{ tag: 'new', text: 'a' }]);
    expect(result.notes).toBe('### Migration: x\n\nDo this.');
  });

  it('ignores tagged-looking lines inside a lead code fence', () => {
    const result = splitEntry('```md\n- **New:** fake\n```\n\n- **Fixed:** real');
    expect(result.items).toEqual([{ tag: 'fixed', text: 'real' }]);
    expect(result.lead).toBe('```md\n- **New:** fake\n```');
  });

  it('accepts * bullets', () => {
    expect(splitEntry('* **Fixed:** x').items).toEqual([{ tag: 'fixed', text: 'x' }]);
  });

  it('populates items per entry in parseChangelog', () => {
    const releases = parseChangelog(`## 2.0.0

### Minor Changes

- [#1](https://x/1) Thanks [@u](https://x)! - **Head**

  Lead text.

  - **New:** a
  - **Breaking:** b

## 1.0.0

### Patch Changes

- Thanks [@u](https://x)! - **Other**

  - **Fixed:** c
`);
    expect(releases[0].entries[0].lead).toBe('Lead text.');
    expect(releases[0].entries[0].items).toEqual([
      { tag: 'new', text: 'a' },
      { tag: 'breaking', text: 'b' },
    ]);
    expect(releases[1].entries[0].items).toEqual([{ tag: 'fixed', text: 'c' }]);
  });
});
