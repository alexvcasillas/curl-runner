import { execFileSync } from 'node:child_process';
import { posix } from 'node:path';

function git(cwd: string, args: string[]): string {
  return execFileSync('git', args, { cwd, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] });
}

/**
 * Parses `git log --format=%x00%cI %P --name-only` output (newest first) into the latest
 * commit date of each repo-relative file. With `skipRoots`, parentless commits are ignored:
 * at a shallow clone's boundary they appear to add every file, so their dates would lie.
 */
export function parseGitLog(output: string, skipRoots: boolean): Map<string, Date> {
  const dates = new Map<string, Date>();
  for (const record of output.split('\0')) {
    const [header, ...files] = record.split('\n');
    const [date, ...parents] = header.trim().split(' ');
    if (!date || (skipRoots && parents.length === 0)) {
      continue;
    }
    for (const file of files) {
      if (file && !dates.has(file)) {
        dates.set(file, new Date(date));
      }
    }
  }
  return dates;
}

/**
 * Last commit date of each path (relative to `cwd`), from one `git log` pass. Paths with no
 * trustworthy date (untracked, beyond a shallow clone, or no git) are left out.
 */
export function lastCommitDates(paths: string[], cwd = process.cwd()): Map<string, Date> {
  const dates = new Map<string, Date>();
  try {
    const prefix = git(cwd, ['rev-parse', '--show-prefix']).trim();
    const shallow = git(cwd, ['rev-parse', '--is-shallow-repository']).trim() === 'true';
    const log = git(cwd, ['log', '--format=%x00%cI %P', '--name-only', '--', ...paths]);
    const byRepoPath = parseGitLog(log, shallow);
    for (const path of paths) {
      const date = byRepoPath.get(posix.normalize(prefix + path));
      if (date) {
        dates.set(path, date);
      }
    }
  } catch {
    // No git or no history (e.g. a tarball build): leave every date out.
  }
  return dates;
}
