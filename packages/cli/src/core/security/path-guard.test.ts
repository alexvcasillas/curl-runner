import { describe, expect, test } from 'bun:test';
import { resolve, sep } from 'node:path';
import { resolveSafePath } from './path-guard';

describe('resolveSafePath', () => {
  const cwd = process.cwd();

  describe('in-cwd paths pass', () => {
    test('relative file in cwd is valid', () => {
      const result = resolveSafePath('output.json');
      expect(result.valid).toBe(true);
      expect(result.resolved).toBe(resolve(cwd, 'output.json'));
      expect(result.error).toBeUndefined();
    });

    test('nested in-cwd subdir is valid', () => {
      const result = resolveSafePath('subdir/nested/output.json');
      expect(result.valid).toBe(true);
      expect(result.resolved).toBe(resolve(cwd, 'subdir/nested/output.json'));
    });

    test('baseDir itself is valid (edge case)', () => {
      const result = resolveSafePath('.', { baseDir: cwd });
      expect(result.valid).toBe(true);
      expect(result.resolved).toBe(cwd);
    });

    test('absolute path inside baseDir is valid', () => {
      const inside = resolve(cwd, 'somefile.txt');
      const result = resolveSafePath(inside);
      expect(result.valid).toBe(true);
      expect(result.resolved).toBe(inside);
    });
  });

  describe('escaping paths are rejected', () => {
    test('../ traversal is rejected', () => {
      const result = resolveSafePath('../escape.json');
      expect(result.valid).toBe(false);
      expect(result.error).toContain('escapes the working directory');
      expect(result.error).toContain('--allow-path');
    });

    test('deep traversal is rejected', () => {
      const result = resolveSafePath('../../etc/passwd');
      expect(result.valid).toBe(false);
      expect(result.error).toContain('escapes the working directory');
    });

    test('absolute /etc/passwd is rejected', () => {
      const result = resolveSafePath('/etc/passwd');
      expect(result.valid).toBe(false);
      expect(result.error).toContain('escapes the working directory');
      expect(result.error).toContain('--allow-path');
    });

    test('absolute path outside cwd is rejected', () => {
      const result = resolveSafePath('/tmp/output.json');
      expect(result.valid).toBe(false);
      expect(result.error).toContain('escapes the working directory');
    });

    test('path that looks inside but resolves outside is rejected', () => {
      // starts inside but uses .. to escape
      const result = resolveSafePath('subdir/../../escape.json');
      expect(result.valid).toBe(false);
      expect(result.error).toContain('escapes the working directory');
    });

    test('error message includes the input path', () => {
      const result = resolveSafePath('../secret.key');
      expect(result.error).toContain('../secret.key');
    });
  });

  describe('allow:true bypasses confinement', () => {
    test('../escape passes when allow:true', () => {
      const result = resolveSafePath('../escape.json', { allow: true });
      expect(result.valid).toBe(true);
      expect(result.resolved).toBe('../escape.json');
      expect(result.error).toBeUndefined();
    });

    test('/etc/passwd passes when allow:true', () => {
      const result = resolveSafePath('/etc/passwd', { allow: true });
      expect(result.valid).toBe(true);
      expect(result.resolved).toBe('/etc/passwd');
    });

    test('absolute path outside cwd passes when allow:true', () => {
      const result = resolveSafePath('/tmp/output.json', { allow: true });
      expect(result.valid).toBe(true);
    });
  });

  describe('baseDir override', () => {
    test('path inside custom baseDir is valid', () => {
      const baseDir = '/tmp';
      const result = resolveSafePath('data.json', { baseDir });
      expect(result.valid).toBe(true);
      expect(result.resolved).toBe(`/tmp${sep}data.json`);
    });

    test('path outside custom baseDir is rejected', () => {
      const baseDir = '/tmp/myapp';
      const result = resolveSafePath('../escape.json', { baseDir });
      expect(result.valid).toBe(false);
      expect(result.error).toContain('escapes the working directory');
    });

    test('absolute path matching custom baseDir is valid', () => {
      const baseDir = '/tmp/myapp';
      const result = resolveSafePath('/tmp/myapp/out.json', { baseDir });
      expect(result.valid).toBe(true);
    });
  });
});
