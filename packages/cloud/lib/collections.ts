import type { GitMeta } from './types';

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
