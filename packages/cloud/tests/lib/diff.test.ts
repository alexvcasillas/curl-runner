import { describe, expect, it } from 'vitest';
import { diffVersions, hunkHeader, lineCounts, MAX_DIFF_BYTES, MAX_DIFF_LINES } from '@/lib/diff';

const f = (path: string, text: string) => ({ path, text });

describe('diffVersions', () => {
  it('reports added, removed and changed files, sorted by path', () => {
    const prev = [f('b.yaml', 'a: 1\n'), f('z.yaml', 'old\n'), f('same.yaml', 'x\n')];
    const next = [f('same.yaml', 'x\n'), f('b.yaml', 'a: 2\n'), f('a.yaml', 'new\n')];
    const out = diffVersions(prev, next);
    expect(out.map((d) => [d.path, d.status])).toEqual([
      ['a.yaml', 'added'],
      ['b.yaml', 'changed'],
      ['z.yaml', 'removed'],
    ]);
  });

  it('gives an identical set no diffs', () => {
    const files = [f('a.yaml', 'a\n'), f('b.yaml', 'b\n')];
    expect(diffVersions(files, [...files].reverse())).toEqual([]);
  });

  it('shows every file as added against nothing', () => {
    const out = diffVersions([], [f('b.yaml', 'one\ntwo\n'), f('a.yaml', 'x\n')]);
    expect(out.map((d) => [d.path, d.status])).toEqual([
      ['a.yaml', 'added'],
      ['b.yaml', 'added'],
    ]);
    expect(out[1].hunks[0].lines).toEqual(['+one', '+two']);
    expect(out[1].hunks[0]).toMatchObject({ oldLines: 0, newStart: 1, newLines: 2 });
    expect(hunkHeader(out[1].hunks[0])).toBe('@@ -0,0 +1,2 @@');
  });

  it('shows a removed file as all deleted lines', () => {
    const [d] = diffVersions([f('a.yaml', 'one\ntwo\n')], []);
    expect(d.status).toBe('removed');
    expect(d.hunks[0].lines).toEqual(['-one', '-two']);
    expect(hunkHeader(d.hunks[0])).toBe('@@ -1,2 +0,0 @@');
  });

  it('keeps 3 lines of context around a change', () => {
    const lines = Array.from({ length: 20 }, (_, i) => `line ${i + 1}`);
    const next = [...lines];
    next[9] = 'changed';
    const [d] = diffVersions(
      [f('a.yaml', `${lines.join('\n')}\n`)],
      [f('a.yaml', `${next.join('\n')}\n`)],
    );
    expect(d.status).toBe('changed');
    expect(d.hunks).toHaveLength(1);
    expect(d.hunks[0].lines).toEqual([
      ' line 7',
      ' line 8',
      ' line 9',
      '-line 10',
      '+changed',
      ' line 11',
      ' line 12',
      ' line 13',
    ]);
  });

  it('counts added and removed lines', () => {
    const [d] = diffVersions([f('a.yaml', 'a\nb\n')], [f('a.yaml', 'a\nc\nd\n')]);
    expect(lineCounts(d)).toEqual({ plus: 2, minus: 1 });
  });

  it('does not diff a file over 1 MB on either side', () => {
    const big = `${'x'.repeat(MAX_DIFF_BYTES)}y\n`;
    const out = diffVersions(
      [f('a.yaml', 'small\n'), f('b.yaml', big)],
      [f('a.yaml', big), f('b.yaml', 'small\n'), f('c.yaml', big)],
    );
    expect(out.map((d) => [d.path, d.status, d.tooLarge, d.hunks.length])).toEqual([
      ['a.yaml', 'changed', true, 0],
      ['b.yaml', 'changed', true, 0],
      ['c.yaml', 'added', true, 0],
    ]);
  });

  it('counts bytes, not characters', () => {
    const wide = '€'.repeat(Math.ceil(MAX_DIFF_BYTES / 3) + 1);
    expect(diffVersions([], [f('a.yaml', wide)])[0].tooLarge).toBe(true);
  });

  it('does not show a diff past 2,000 lines', () => {
    const many = (n: number) => `${Array.from({ length: n }, (_, i) => `l${i}`).join('\n')}\n`;
    const [atLimit] = diffVersions([], [f('a.yaml', many(MAX_DIFF_LINES - 1))]);
    expect(atLimit.tooLarge).toBeUndefined();
    const [over] = diffVersions([], [f('a.yaml', many(MAX_DIFF_LINES))]);
    expect(over.tooLarge).toBe(true);
    expect(over.hunks).toEqual([]);
    expect(over.status).toBe('added');
  });
});
