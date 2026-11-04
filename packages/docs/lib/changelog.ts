import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { unstable_cache } from 'next/cache';

export type Bump = 'major' | 'minor' | 'patch';

export type ChangeTag = 'new' | 'improved' | 'fixed' | 'breaking';

export const CHANGE_TAGS: readonly ChangeTag[] = ['new', 'improved', 'fixed', 'breaking'];

export const TAG_LABEL: Record<ChangeTag, string> = {
  new: 'New',
  improved: 'Improved',
  fixed: 'Fixed',
  breaking: 'Breaking',
};

export interface ChangeItem {
  tag: ChangeTag;
  text: string;
}

export interface ChangeEntry {
  bump: Bump;
  headline: string | null;
  /** Markdown, dedented, without the changesets prefix or the headline. */
  body: string;
  /** Markdown before the first tagged item (the whole body when there are none). */
  lead: string;
  items: ChangeItem[];
  /** Markdown after the tagged items, such as code blocks and migration sections. */
  notes: string;
  pr: number | null;
  /** Short (7-character) commit SHA. */
  commit: string | null;
}

export interface Release {
  version: string;
  /** The highest bump among the release's entries. */
  bump: Bump;
  entries: ChangeEntry[];
}

/** `packages/cli/CHANGELOG.md`, resolved from the docs app root (the build's working directory). */
export const CHANGELOG_PATH = path.resolve(process.cwd(), '../cli/CHANGELOG.md');

const GITHUB_REPO = 'alexvcasillas/curl-runner';
export const REPO_URL = `https://github.com/${GITHUB_REPO}`;
const RELEASES_API_URL = `https://api.github.com/repos/${GITHUB_REPO}/releases?per_page=100`;

const BUMP_RANK: Record<Bump, number> = { patch: 0, minor: 1, major: 2 };

const RELEASE_HEADING = /^## (\d+\.\d+\.\d+\S*)\s*$/;
const SECTION_HEADING = /^### (Major|Minor|Patch) Changes\s*$/i;
const ENTRY_START = /^- (.*)$/;
// `[#N](url) [`sha`](url) Thanks [@user](url)! - ` — every part optional, as
// @changesets/changelog-github omits the ones it cannot resolve.
const PREFIX =
  /^(?:\[#(\d+)\]\([^)]*\)\s*)?(?:\[`([0-9a-f]{7,40})`\]\([^)]*\)\s*)?(?:Thanks [^!]*!\s*)?- /;
const HEADLINE = /^\*\*(.+?)\*\*[ \t]*(?:\n|$)/;

const TAGGED_ITEM = /^[-*] \*\*(new|improved|fixed|breaking):\*\*[ \t]*(.*)$/i;
const BLOCK_START = /^\s*(?:```|~~~|#{1,6}\s|[-*+] |\d+[.)] )/;
const FENCE_LINE = /^\s*(```|~~~)/;

/**
 * Splits an entry body into a lead, the run of `- **Tag:** text` items, and
 * the notes after it. The run is only consecutive tagged items with their
 * indented or wrapped continuation lines and the blank lines between them.
 * Anything else (an untagged bullet, a fence, a heading, a paragraph) ends the
 * run, and it plus everything after it is the notes. Tagged-looking lines
 * inside fenced code are ignored. Without tagged items the whole body is the lead.
 */
export function splitEntry(body: string): { lead: string; items: ChangeItem[]; notes: string } {
  const lines = body.split('\n');
  let first = -1;
  let fence: string | null = null;
  for (let n = 0; n < lines.length && first === -1; n++) {
    const marker = lines[n].match(FENCE_LINE)?.[1];
    if (fence) {
      if (marker === fence) {
        fence = null;
      }
    } else if (marker) {
      fence = marker;
    } else if (TAGGED_ITEM.test(lines[n])) {
      first = n;
    }
  }
  if (first === -1) {
    return { lead: body.trim(), items: [], notes: '' };
  }

  const items: ChangeItem[] = [];
  const parts: string[][] = [];
  let i = first;
  let sawBlank = false;
  for (; i < lines.length; i++) {
    const line = lines[i];
    const tagged = line.match(TAGGED_ITEM);
    if (tagged) {
      items.push({ tag: tagged[1].toLowerCase() as ChangeTag, text: '' });
      parts.push([tagged[2].trim()]);
      sawBlank = false;
    } else if (line.trim() === '') {
      sawBlank = true;
    } else if (BLOCK_START.test(line)) {
      break;
    } else if (/^\s/.test(line) || !sawBlank) {
      parts[parts.length - 1].push(line.trim());
      sawBlank = false;
    } else {
      break;
    }
  }

  for (const [n, item] of items.entries()) {
    item.text = parts[n].filter(Boolean).join(' ');
  }
  return {
    lead: lines.slice(0, first).join('\n').trim(),
    items,
    notes: lines.slice(i).join('\n').trim(),
  };
}

interface Draft {
  bump: Bump;
  lines: string[];
}

function finishEntry(draft: Draft): ChangeEntry {
  let text = draft.lines.join('\n').trim();
  let pr: number | null = null;
  let commit: string | null = null;

  const prefix = text.match(PREFIX);
  if (prefix && (prefix[1] || prefix[2] || /Thanks /.test(prefix[0]))) {
    pr = prefix[1] ? Number(prefix[1]) : null;
    commit = prefix[2] ? prefix[2].slice(0, 7) : null;
    text = text.slice(prefix[0].length);
  }

  let headline: string | null = null;
  const bold = text.match(HEADLINE);
  if (bold) {
    headline = bold[1].trim();
    text = text.slice(bold[0].length);
  }

  const body = text.trim();
  return { bump: draft.bump, headline, body, ...splitEntry(body), pr, commit };
}

/**
 * Parses a changesets CHANGELOG into releases, in file order (newest first).
 * Every `## x.y.z` heading yields one release, even when it has no entries.
 */
export function parseChangelog(markdown: string): Release[] {
  const releases: Release[] = [];
  let release: Release | null = null;
  let section: Bump = 'patch';
  let draft: Draft | null = null;
  let previousBlank = true;

  const flush = () => {
    if (release && draft) {
      release.entries.push(finishEntry(draft));
    }
    draft = null;
  };

  for (const line of markdown.replace(/\r\n?/g, '\n').split('\n')) {
    const heading = line.match(RELEASE_HEADING);
    if (heading) {
      flush();
      release = { version: heading[1], bump: 'patch', entries: [] };
      releases.push(release);
      section = 'patch';
      previousBlank = true;
      continue;
    }
    if (!release) {
      continue;
    }

    const sectionHeading = line.match(SECTION_HEADING);
    if (sectionHeading) {
      flush();
      section = sectionHeading[1].toLowerCase() as Bump;
      previousBlank = true;
      continue;
    }

    const blank = line.trim() === '';
    const entryStart = line.match(ENTRY_START);
    if (entryStart) {
      flush();
      draft = { bump: section, lines: [entryStart[1]] };
    } else if (blank) {
      draft?.lines.push('');
    } else if (/^\s/.test(line) || (draft && !previousBlank)) {
      // Continuation: indented, or a lazy continuation of the previous line.
      if (!draft) {
        draft = { bump: section, lines: [] };
      }
      draft.lines.push(line.replace(/^ {1,2}/, ''));
    } else {
      // A loose top-level paragraph inside a release: keep it as its own entry.
      flush();
      draft = { bump: section, lines: [line] };
    }
    previousBlank = blank;
  }
  flush();

  for (const r of releases) {
    r.bump = r.entries.reduce<Bump>(
      (max, e) => (BUMP_RANK[e.bump] > BUMP_RANK[max] ? e.bump : max),
      'patch',
    );
  }
  return releases;
}

/** Reads the CLI changelog. Throws (failing the build) rather than rendering an empty page. */
export function readChangelog(): string {
  try {
    return readFileSync(CHANGELOG_PATH, 'utf8');
  } catch (error) {
    throw new Error(
      `Cannot read the CLI changelog at ${CHANGELOG_PATH}. The /changelog page and /rss.xml are generated from packages/cli/CHANGELOG.md, so it must be present when the docs site builds.`,
      { cause: error },
    );
  }
}

export function getReleases(): Release[] {
  const releases = parseChangelog(readChangelog());
  if (releases.length === 0) {
    throw new Error(`No "## x.y.z" release headings found in ${CHANGELOG_PATH}.`);
  }
  return releases;
}

/** Throws on any failure so that `unstable_cache` never stores it. */
async function fetchPublishedDates(): Promise<Record<string, string>> {
  const headers: HeadersInit = { Accept: 'application/vnd.github.v3+json' };
  if (process.env.GITHUB_TOKEN) {
    headers.Authorization = `token ${process.env.GITHUB_TOKEN}`;
  }
  const response = await fetch(RELEASES_API_URL, { headers, next: { revalidate: false } });
  if (!response.ok) {
    throw new Error(`GitHub API error: ${response.status}`);
  }
  const json: unknown = await response.json();
  if (!Array.isArray(json)) {
    throw new Error('Unexpected GitHub releases payload');
  }
  const dates: Record<string, string> = {};
  for (const item of json) {
    if (typeof item !== 'object' || item === null) {
      continue;
    }
    const { tag_name: tag, published_at: publishedAt } = item as Record<string, unknown>;
    const match = typeof tag === 'string' ? tag.match(/^@curl-runner\/cli@(\S+)$/) : null;
    if (match && typeof publishedAt === 'string') {
      dates[match[1]] = publishedAt;
    }
  }
  return dates;
}

const getPublishedDates = unstable_cache(fetchPublishedDates, ['github-release-dates'], {
  revalidate: 3600,
  tags: ['github-release'],
});

/** Committer date of the last commit touching the changelog, or null without git history. */
function lastChangelogCommitDate(): string | null {
  try {
    const out = execFileSync('git', ['log', '-1', '--format=%cI', '--', 'CHANGELOG.md'], {
      cwd: path.dirname(CHANGELOG_PATH),
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
      timeout: 5000,
    }).trim();
    return out && !Number.isNaN(Date.parse(out)) ? out : null;
  } catch {
    return null;
  }
}

/**
 * Version → ISO publish date from the GitHub Releases API (`{}` when the API
 * fails). The newest version's GitHub Release is created minutes after the
 * deploy that ships its changelog, so when the API does not list it yet it is
 * dated by the last commit touching the changelog (or the build time without
 * git history). Other versions missing from the API have no date.
 */
export async function getReleaseDates(): Promise<Record<string, string>> {
  let published: Record<string, string>;
  try {
    published = await getPublishedDates();
  } catch (error) {
    // Not cached: the next request retries.
    console.error('Failed to fetch release dates:', error);
    return {};
  }
  const dates = { ...published };
  let newest: string | undefined;
  try {
    newest = parseChangelog(readChangelog())[0]?.version;
  } catch {
    newest = undefined;
  }
  if (newest && !dates[newest]) {
    dates[newest] = lastChangelogCommitDate() ?? new Date().toISOString();
  }
  return dates;
}
