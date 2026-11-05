import { type StructuredPatchHunk, structuredPatch } from 'diff';
import type { BundleFile } from './types';

export type Hunk = StructuredPatchHunk;

export type FileDiff = {
  path: string;
  status: 'added' | 'removed' | 'changed';
  /** Each line starts with a space (context), `+` or `-`, or is a `\ No newline` note. */
  hunks: Hunk[];
  /** Too big to show: `hunks` is empty and the page points at the download. */
  tooLarge?: boolean;
};

const CONTEXT = 3;
/** A file over this on either side is not diffed at all. */
export const MAX_DIFF_BYTES = 1_000_000;
/** A diff with more lines than this is not shown. */
export const MAX_DIFF_LINES = 2000;

const encoder = new TextEncoder();
const bytes = (text: string) => encoder.encode(text).length;

function patch(path: string, status: FileDiff['status'], before: string, after: string): FileDiff {
  if (bytes(before) > MAX_DIFF_BYTES || bytes(after) > MAX_DIFF_BYTES) {
    return { path, status, hunks: [], tooLarge: true };
  }
  const { hunks } = structuredPatch(path, path, before, after, undefined, undefined, {
    context: CONTEXT,
  });
  const lines = hunks.reduce((n, h) => n + 1 + h.lines.length, 0);
  return lines > MAX_DIFF_LINES
    ? { path, status, hunks: [], tooLarge: true }
    : { path, status, hunks };
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
      out.push(patch(path, 'added', '', text));
    } else if (old !== text) {
      out.push(patch(path, 'changed', old, text));
    }
  }
  for (const [path, text] of before) {
    if (!after.has(path)) {
      out.push(patch(path, 'removed', text, ''));
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
