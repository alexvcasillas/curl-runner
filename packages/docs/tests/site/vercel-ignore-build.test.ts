import { execFileSync, spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

const script = resolve(__dirname, '../../scripts/vercel-ignore-build.sh');

let repo: string;

const git = (...args: string[]) =>
  execFileSync('git', args, { cwd: repo, encoding: 'utf8' }).trim();

const write = (path: string, content: string) => {
  mkdirSync(dirname(join(repo, path)), { recursive: true });
  writeFileSync(join(repo, path), content);
};

const commit = (path: string, content = String(Math.random())) => {
  write(path, content);
  git('add', '-A');
  git('commit', '-qm', `change ${path}`);
  return git('rev-parse', 'HEAD');
};

/** Runs the script the way Vercel does: from the docs root. 0 skips, 1 builds. */
const run = (env: Record<string, string> = {}) =>
  spawnSync('sh', [script], {
    cwd: join(repo, 'packages/docs'),
    // A clean environment, so Vercel's variables from the real one can't leak in.
    env: { NODE_ENV: 'test', PATH: process.env.PATH, ...env },
  }).status;

beforeEach(() => {
  repo = mkdtempSync(join(tmpdir(), 'vercel-ignore-'));
  git('init', '-q');
  git('config', 'user.email', 'test@example.com');
  git('config', 'user.name', 'Test');
  write('packages/docs/page.mdx', 'docs');
  write('packages/cli/CHANGELOG.md', '# changelog');
  write('packages/cli/package.json', '{"version":"1.0.0"}');
  write('packages/cli/src/main.ts', 'cli');
  write('crates/studio/src/lib.rs', 'studio');
  write('pnpm-lock.yaml', 'lock');
  git('add', '-A');
  git('commit', '-qm', 'init');
});

afterEach(() => rmSync(repo, { recursive: true, force: true }));

describe('vercel-ignore-build.sh', () => {
  it('skips release PR branches', () => {
    expect(run({ VERCEL_GIT_COMMIT_REF: 'changeset-release/main' })).toBe(0);
  });

  it('builds when there is no previous deployment to compare with', () => {
    expect(run()).toBe(1);
  });

  it('builds when the previous deployment commit is not in the clone', () => {
    expect(run({ VERCEL_GIT_PREVIOUS_SHA: 'a'.repeat(40) })).toBe(1);
  });

  it('skips commits that only touch other packages', () => {
    const base = git('rev-parse', 'HEAD');
    commit('packages/cli/src/main.ts');
    commit('crates/studio/src/lib.rs');

    expect(run({ VERCEL_GIT_PREVIOUS_SHA: base })).toBe(0);
  });

  it.each([
    ['a docs page', 'packages/docs/page.mdx'],
    ['the CLI changelog the docs render', 'packages/cli/CHANGELOG.md'],
    ['the CLI version the docs show', 'packages/cli/package.json'],
    ['the lockfile', 'pnpm-lock.yaml'],
  ])('builds when %s changed since the last deployment', (_, path) => {
    const base = git('rev-parse', 'HEAD');
    commit(path);
    // A later unrelated commit must not hide the earlier docs change.
    commit('crates/studio/src/lib.rs');

    expect(run({ VERCEL_GIT_PREVIOUS_SHA: base })).toBe(1);
  });
});
