import { type StructuredPatchHunk, structuredPatch } from 'diff';
import type { BundleFile } from './types';

export type Hunk = StructuredPatchHunk;

export type FileDiff = {
  path: string;
  status: 'added' | 'removed' | 'changed';
  /** Each line starts with a space (context), `+` or `-`, or is a `\ No newline` note. */
  hunks: Hunk[];
};

const CONTEXT = 3;

function patch(path: string, before: string, after: string): Hunk[] {
  return structuredPatch(path, path, before, after, undefined, undefined, { context: CONTEXT })
    .hunks;
}

/**
 * What changed going from `prev` to `next`, by path. Unchanged files are left
 * out, and the result is sorted by path. With no `prev`, every file is added.
 */
export function diffVersions(prev: BundleFile[], next: BundleFile[]): FileDiff[] {
  const before = new Map(prev.map((f) => [f.path, f.text]));
  const after = new Map(next.map((f) => [f.path, f.text]));
  const out: FileDiff[] = [];

  for (const [path, text] of after) {
    const old = before.get(path);
    if (old === undefined) {
      out.push({ path, status: 'added', hunks: patch(path, '', text) });
    } else if (old !== text) {
      out.push({ path, status: 'changed', hunks: patch(path, old, text) });
    }
  }
  for (const [path, text] of before) {
    if (!after.has(path)) {
      out.push({ path, status: 'removed', hunks: patch(path, text, '') });
    }
  }
  return out.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
}

/** Added and removed line counts, for "+14 -0". */
export function lineCounts(diff: FileDiff): { plus: number; minus: number } {
  let plus = 0;
  let minus = 0;
  for (const hunk of diff.hunks) {
    for (const line of hunk.lines) {
      if (line.startsWith('+')) {
        plus += 1;
      } else if (line.startsWith('-')) {
        minus += 1;
      }
    }
  }
  return { plus, minus };
}

/** `@@ -9,6 +9,11 @@`. An empty side starts at 0, as `git diff` writes it. */
export function hunkHeader(h: Hunk): string {
  const oldStart = h.oldLines === 0 ? 0 : h.oldStart;
  const newStart = h.newLines === 0 ? 0 : h.newStart;
  return `@@ -${oldStart},${h.oldLines} +${newStart},${h.newLines} @@`;
}
