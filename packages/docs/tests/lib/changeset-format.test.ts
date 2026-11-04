import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { describe, expect, it, vi } from 'vitest';

vi.mock('next/cache', () => ({
  unstable_cache: <T extends (...args: never[]) => unknown>(fn: T) => fn,
}));

import {
  checkChangelogText,
  checkChangesetDir,
  checkChangesetText,
} from '../../lib/changeset-format';

const repoRoot = path.resolve(__dirname, '../../../..');
const FM = '---\n"@curl-runner/cli": minor\n---\n\n';

describe('changeset format checker', () => {
  it('accepts a headline with tagged items', () => {
    const text = `${FM}**Title**\n\nLead.\n\n- **New:** a thing.\n- **Fixed:** b.\n`;
    expect(checkChangesetText(text, 'ok.md')).toEqual([]);
  });

  it('names the file and the expected format for a bad changeset', () => {
    const dir = mkdtempSync(path.join(tmpdir(), 'changesets-'));
    writeFileSync(path.join(dir, 'README.md'), 'ignored');
    writeFileSync(path.join(dir, 'bad-one.md'), `${FM}Just a plain paragraph.\n`);
    writeFileSync(path.join(dir, 'no-items.md'), `${FM}**Title**\n\nLead only.\n`);
    mkdirSync(path.join(dir, 'sub'));
    const problems = checkChangesetDir(dir).join('\n');
    expect(problems).toContain('bad-one.md: the summary must start with a bold headline');
    expect(problems).toContain('no-items.md: no tagged items found.');
    expect(problems).toContain('`- **New|Improved|Fixed|Breaking:** text`');
  });

  it('names the release with no tagged items', () => {
    const md = '## 1.0.0\n\n### Minor Changes\n\n- **Title**\n\n  Plain text.\n';
    expect(checkChangelogText(md).join('\n')).toContain('release 1.0.0 has no tagged items');
  });
});

describe('repository changelog format', () => {
  it('every pending changeset has a headline and tagged items', () => {
    expect(checkChangesetDir(path.join(repoRoot, '.changeset')).join('\n')).toBe('');
  });

  it('every release in CHANGELOG.md has tagged items', () => {
    const md = readFileSync(path.join(repoRoot, 'packages/cli/CHANGELOG.md'), 'utf8');
    expect(checkChangelogText(md).join('\n')).toBe('');
  });
});
