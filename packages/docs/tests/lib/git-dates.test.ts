import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterAll, describe, expect, it } from 'vitest';
import { lastCommitDates, parseGitLog } from '@/lib/git-dates';

describe('parseGitLog', () => {
  const log = [
    '',
    '2026-10-03T00:00:00+02:00 bbb\n\npkg/a.mdx\n',
    '2026-10-02T00:00:00+02:00 aaa\n\npkg/a.mdx\npkg/b.mdx\n',
    '2026-10-01T00:00:00+02:00 \n\npkg/a.mdx\npkg/b.mdx\npkg/c.mdx\n',
  ].join('\0');

  it('keeps the newest date of each file', () => {
    const dates = parseGitLog(log, false);
    expect(dates.get('pkg/a.mdx')).toEqual(new Date('2026-10-03T00:00:00+02:00'));
    expect(dates.get('pkg/b.mdx')).toEqual(new Date('2026-10-02T00:00:00+02:00'));
    expect(dates.get('pkg/c.mdx')).toEqual(new Date('2026-10-01T00:00:00+02:00'));
  });

  it('skips parentless commits when asked (a shallow clone boundary)', () => {
    const dates = parseGitLog(log, true);
    expect(dates.get('pkg/b.mdx')).toEqual(new Date('2026-10-02T00:00:00+02:00'));
    expect(dates.has('pkg/c.mdx')).toBe(false);
  });

  it('returns nothing for empty output', () => {
    expect(parseGitLog('', false).size).toBe(0);
  });
});

describe('lastCommitDates', () => {
  const root = mkdtempSync(join(tmpdir(), 'git-dates-'));
  const repo = join(root, 'repo');
  const git = (cwd: string, args: string[], date?: string) =>
    execFileSync('git', args, {
      cwd,
      stdio: 'ignore',
      env: {
        ...process.env,
        GIT_AUTHOR_NAME: 't',
        GIT_AUTHOR_EMAIL: 't@example.com',
        GIT_COMMITTER_NAME: 't',
        GIT_COMMITTER_EMAIL: 't@example.com',
        ...(date && { GIT_AUTHOR_DATE: date, GIT_COMMITTER_DATE: date }),
      },
    });
  const commit = (files: string[], date: string) => {
    for (const file of files) {
      writeFileSync(join(repo, file), date);
    }
    git(repo, ['add', '.']);
    git(repo, ['commit', '-m', date], date);
  };

  mkdirSync(join(repo, 'docs'), { recursive: true });
  mkdirSync(join(repo, 'cli'));
  git(repo, ['init', '-q']);
  commit(['docs/old.mdx', 'docs/new.mdx', 'cli/CHANGELOG.md'], '2026-01-01T00:00:00Z');
  commit(['docs/new.mdx'], '2026-02-01T00:00:00Z');
  commit(['cli/CHANGELOG.md'], '2026-03-01T00:00:00Z');

  afterAll(() => rmSync(root, { recursive: true, force: true }));

  it('dates paths relative to cwd, including ones outside it', () => {
    const dates = lastCommitDates(
      ['old.mdx', 'new.mdx', '../cli/CHANGELOG.md'],
      join(repo, 'docs'),
    );
    expect(dates.get('old.mdx')).toEqual(new Date('2026-01-01T00:00:00Z'));
    expect(dates.get('new.mdx')).toEqual(new Date('2026-02-01T00:00:00Z'));
    expect(dates.get('../cli/CHANGELOG.md')).toEqual(new Date('2026-03-01T00:00:00Z'));
  });

  it('leaves out untracked paths', () => {
    expect(lastCommitDates(['missing.mdx'], join(repo, 'docs')).size).toBe(0);
  });

  it('leaves out files only dated by a shallow clone boundary', () => {
    const shallow = join(root, 'shallow');
    git(root, ['clone', '-q', '--depth', '2', `file://${repo}`, shallow]);
    const dates = lastCommitDates(
      ['old.mdx', 'new.mdx', '../cli/CHANGELOG.md'],
      join(shallow, 'docs'),
    );
    expect(dates.get('../cli/CHANGELOG.md')).toEqual(new Date('2026-03-01T00:00:00Z'));
    expect(dates.has('new.mdx')).toBe(false);
    expect(dates.has('old.mdx')).toBe(false);
  });

  it('returns nothing outside a git repository', () => {
    expect(lastCommitDates(['x'], tmpdir()).size).toBe(0);
  });
});
