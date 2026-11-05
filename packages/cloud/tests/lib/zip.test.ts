import { unzipSync } from 'fflate';
import { describe, expect, it } from 'vitest';
import { isSafeEntryPath, UnsafePathError, zipFiles } from '@/lib/zip';

describe('isSafeEntryPath', () => {
  it('accepts relative forward-slash paths', () => {
    for (const p of ['a.yaml', 'requests/charges/create.yaml', '.hidden/a.yaml', 'a..b.yaml']) {
      expect(isSafeEntryPath(p), p).toBe(true);
    }
  });

  it('rejects empty, absolute, parent, backslash and NUL paths', () => {
    for (const p of [
      '',
      '/etc/passwd',
      '../evil.yaml',
      'a/../../evil.yaml',
      'a/..',
      '..',
      'a\\b.yaml',
      '..\\evil.yaml',
      'a\0.yaml',
    ]) {
      expect(isSafeEntryPath(p), JSON.stringify(p)).toBe(false);
    }
  });
});

describe('zipFiles', () => {
  it('puts each file at its path', () => {
    const out = unzipSync(zipFiles([{ path: 'a/b.yaml', text: 'x: 1\n' }]));
    expect(Object.keys(out)).toEqual(['a/b.yaml']);
    expect(new TextDecoder().decode(out['a/b.yaml'])).toBe('x: 1\n');
  });

  it('fails the whole archive on an unsafe path', () => {
    expect(() =>
      zipFiles([
        { path: 'ok.yaml', text: '' },
        { path: '../evil.yaml', text: '' },
      ]),
    ).toThrow(UnsafePathError);
  });
});
