import { readdirSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { parseChangelog, splitEntry } from './changelog';

const EXPECTED = [
  'Expected: a bold headline line, a short lead, then tagged items:',
  '**Headline**, then `- **New|Improved|Fixed|Breaking:** text` lines (see CONTRIBUTE.md).',
].join('\n');

const HEADLINE = /^\*\*(.+?)\*\*[ \t]*(?:\n|$)/;
const FRONT_MATTER = /^---\r?\n(?:[\s\S]*?\r?\n)?---\r?\n?/;

/** Returns the problems in one changeset file's text (empty when it is fine). */
export function checkChangesetText(text: string, name: string): string[] {
  const body = text
    .replace(/^\uFEFF/, '')
    .replace(/\r\n?/g, '\n')
    .replace(FRONT_MATTER, '')
    .trim();
  // `pnpm changeset --empty` writes no body: no release notes, so nothing to check.
  if (body === '') {
    return [];
  }
  const problems: string[] = [];
  const headline = body.match(HEADLINE);
  if (!headline) {
    problems.push(`${name}: the summary must start with a bold headline line.`);
  }
  const rest = headline ? body.slice(headline[0].length) : body;
  if (splitEntry(rest.trim()).items.length === 0) {
    problems.push(`${name}: no tagged items found.`);
  }
  return problems.length > 0 ? [...problems, EXPECTED] : [];
}

/** Checks every `*.md` in a changeset directory, except README.md. */
export function checkChangesetDir(dir: string): string[] {
  return readdirSync(dir)
    .filter((f) => f.endsWith('.md') && f !== 'README.md')
    .sort()
    .flatMap((f) => checkChangesetText(readFileSync(path.join(dir, f), 'utf8'), f));
}

/** Returns one problem per release in a CHANGELOG without a tagged item. */
export function checkChangelogText(markdown: string): string[] {
  const problems = parseChangelog(markdown)
    .filter((r) => !r.entries.some((e) => e.items.length > 0))
    .map((r) => `CHANGELOG.md: release ${r.version} has no tagged items.`);
  return problems.length > 0 ? [...problems, EXPECTED] : [];
}
