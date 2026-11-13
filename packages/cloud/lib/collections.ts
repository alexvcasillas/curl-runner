import type { GitMeta, Source } from './types';

/** What a collection's source reads as: files with git details are `git`,
 * files without are `files`, and `cloud` syncs from the studio. The API
 * keeps two values; the third is the version's own git details. */
export type SourceKind = 'git' | 'files' | 'cloud';

export function sourceKind(source: Source, git: GitMeta | null | undefined): SourceKind {
  if (source === 'cloud') {
    return 'cloud';
  }
  return git && (git.commit || git.ref || git.repo_url) ? 'git' : 'files';
}

/** `acme/api · main@4f2a9c1`, or as much of it as the version has. */
export function gitSummary(git: GitMeta | null | undefined): string | null {
  if (!git) {
    return null;
  }
  let repo: string | null = null;
  if (git.repo_url) {
    try {
      repo = new URL(git.repo_url).pathname.replace(/^\/|\.git$/g, '') || null;
    } catch {
      repo = null;
    }
  }
  const branch = git.ref?.replace(/^refs\/(heads|tags)\//, '') ?? null;
  const commit = git.commit ? shortSha(git.commit) : null;
  const at = branch && commit ? `${branch}@${commit}` : (branch ?? commit);
  const parts = [repo, at].filter(Boolean);
  return parts.length ? parts.join(' · ') : null;
}

/** The first 7 characters of a commit, like `git log --oneline`. */
export function shortSha(sha: string): string {
  return sha.slice(0, 7);
}

const COMMIT_HOSTS = new Set(['github.com', 'gitlab.com']);

/**
 * `repo_url/commit/sha`, only when the repository is https on github.com or
 * gitlab.com. The URL comes from CI, so anything else stays plain text.
 */
export function commitUrl(git: GitMeta | null): string | null {
  if (!git?.commit || !git.repo_url || !/^[0-9a-f]{7,64}$/i.test(git.commit)) {
    return null;
  }
  let url: URL;
  try {
    url = new URL(git.repo_url);
  } catch {
    return null;
  }
  if (
    url.protocol !== 'https:' ||
    !COMMIT_HOSTS.has(url.hostname) ||
    url.username ||
    url.password ||
    url.port
  ) {
    return null;
  }
  const path = url.pathname.replace(/\/+$/, '').replace(/\.git$/, '');
  const segments = path.split('/').filter(Boolean);
  if (segments.length < 2 || !segments.every((s) => /^[\w.-]+$/.test(s))) {
    return null;
  }
  return `https://${url.hostname}/${segments.join('/')}/commit/${git.commit}`;
}

/** The footer note under the collections list: what the plan covers. */
export function planNote(plan: 'free' | 'team', billing: boolean): string {
  if (!billing) {
    return 'Self-hosted: every feature is on.';
  }
  return plan === 'free'
    ? 'Free plan: public collections are unlimited. Restricted ones need Team.'
    : 'Team plan';
}
