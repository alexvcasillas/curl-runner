import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { join } from 'node:path';
import { getEnvFilePaths, loadEnvFiles } from './env-file-loader';

describe('env-file-loader', () => {
  const testDir = join(import.meta.dir, '__test-env-files__');

  beforeEach(async () => {
    await Bun.write(join(testDir, '.gitkeep'), '');
  });

  afterEach(async () => {
    // Clean up test files
    const { rmSync } = await import('node:fs');
    try {
      rmSync(testDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  describe('loadEnvFiles', () => {
    test('returns empty when no .env files exist', async () => {
      const result = await loadEnvFiles(testDir);
      expect(result.variables).toEqual({});
      expect(result.secretKeys).toEqual([]);
    });

    test('loads .env file', async () => {
      await Bun.write(join(testDir, '.env'), 'API_URL=https://api.example.com\nAPI_KEY=abc123');

      const result = await loadEnvFiles(testDir);
      expect(result.variables).toEqual({
        API_URL: 'https://api.example.com',
        API_KEY: 'abc123',
      });
    });

    test('loads .env.local over .env', async () => {
      await Bun.write(join(testDir, '.env'), 'API_URL=https://prod.api.com\nFOO=bar');
      await Bun.write(join(testDir, '.env.local'), 'API_URL=https://local.api.com');

      const result = await loadEnvFiles(testDir);
      expect(result.variables.API_URL).toBe('https://local.api.com');
      expect(result.variables.FOO).toBe('bar');
    });

    test('loads environment-specific files', async () => {
      await Bun.write(join(testDir, '.env'), 'API_URL=https://default.api.com');
      await Bun.write(join(testDir, '.env.production'), 'API_URL=https://prod.api.com');

      const result = await loadEnvFiles(testDir, 'production');
      expect(result.variables.API_URL).toBe('https://prod.api.com');
    });

    test('loads .env.{env}.local over .env.{env}', async () => {
      await Bun.write(join(testDir, '.env.staging'), 'API_URL=https://staging.api.com');
      await Bun.write(join(testDir, '.env.staging.local'), 'API_URL=https://my-staging.api.com');

      const result = await loadEnvFiles(testDir, 'staging');
      expect(result.variables.API_URL).toBe('https://my-staging.api.com');
    });

    test('full priority chain: .env < .env.local < .env.{env} < .env.{env}.local', async () => {
      await Bun.write(join(testDir, '.env'), 'A=1\nB=1\nC=1\nD=1');
      await Bun.write(join(testDir, '.env.local'), 'B=2\nC=2\nD=2');
      await Bun.write(join(testDir, '.env.test'), 'C=3\nD=3');
      await Bun.write(join(testDir, '.env.test.local'), 'D=4');

      const result = await loadEnvFiles(testDir, 'test');
      expect(result.variables).toEqual({ A: '1', B: '2', C: '3', D: '4' });
    });

    test('extracts SECRET_ prefixed keys', async () => {
      await Bun.write(
        join(testDir, '.env'),
        'API_URL=https://api.com\nSECRET_API_KEY=supersecret\nSECRET_TOKEN=mytoken\nPUBLIC_KEY=pub123',
      );

      const result = await loadEnvFiles(testDir);
      expect(result.secretKeys).toContain('SECRET_API_KEY');
      expect(result.secretKeys).toContain('SECRET_TOKEN');
      expect(result.secretKeys).not.toContain('API_URL');
      expect(result.secretKeys).not.toContain('PUBLIC_KEY');
    });

    test('handles quoted values with double quotes', async () => {
      await Bun.write(join(testDir, '.env'), 'MESSAGE="Hello World"');

      const result = await loadEnvFiles(testDir);
      expect(result.variables.MESSAGE).toBe('Hello World');
    });

    test('handles quoted values with single quotes', async () => {
      await Bun.write(join(testDir, '.env'), "MESSAGE='Hello World'");

      const result = await loadEnvFiles(testDir);
      expect(result.variables.MESSAGE).toBe('Hello World');
    });

    test('handles escape sequences in double-quoted strings', async () => {
      await Bun.write(join(testDir, '.env'), 'MULTILINE="line1\\nline2\\tindented"');

      const result = await loadEnvFiles(testDir);
      expect(result.variables.MULTILINE).toBe('line1\nline2\tindented');
    });

    test('ignores comments', async () => {
      await Bun.write(
        join(testDir, '.env'),
        '# This is a comment\nAPI_URL=https://api.com\n# Another comment',
      );

      const result = await loadEnvFiles(testDir);
      expect(result.variables).toEqual({ API_URL: 'https://api.com' });
    });

    test('ignores empty lines', async () => {
      await Bun.write(join(testDir, '.env'), '\n\nAPI_URL=https://api.com\n\n\nAPI_KEY=key\n');

      const result = await loadEnvFiles(testDir);
      expect(result.variables).toEqual({
        API_URL: 'https://api.com',
        API_KEY: 'key',
      });
    });

    test('ignores lines without equals sign', async () => {
      await Bun.write(join(testDir, '.env'), 'VALID=value\nINVALID_LINE\nANOTHER=value2');

      const result = await loadEnvFiles(testDir);
      expect(result.variables).toEqual({
        VALID: 'value',
        ANOTHER: 'value2',
      });
    });

    test('handles values with equals signs', async () => {
      await Bun.write(join(testDir, '.env'), 'URL=https://api.com?key=value&foo=bar');

      const result = await loadEnvFiles(testDir);
      expect(result.variables.URL).toBe('https://api.com?key=value&foo=bar');
    });

    test('handles empty values', async () => {
      await Bun.write(join(testDir, '.env'), 'EMPTY=\nANOTHER=value');

      const result = await loadEnvFiles(testDir);
      expect(result.variables.EMPTY).toBe('');
      expect(result.variables.ANOTHER).toBe('value');
    });

    test('trims whitespace around keys and values', async () => {
      await Bun.write(join(testDir, '.env'), '  KEY  =  value  ');

      const result = await loadEnvFiles(testDir);
      expect(result.variables.KEY).toBe('value');
    });
  });

  describe('getEnvFilePaths', () => {
    test('returns empty when no files exist', async () => {
      const paths = await getEnvFilePaths(testDir);
      expect(paths).toEqual([]);
    });

    test('returns .env when it exists', async () => {
      await Bun.write(join(testDir, '.env'), 'KEY=value');

      const paths = await getEnvFilePaths(testDir);
      expect(paths).toEqual([join(testDir, '.env')]);
    });

    test('returns multiple existing files', async () => {
      await Bun.write(join(testDir, '.env'), 'KEY=value');
      await Bun.write(join(testDir, '.env.local'), 'KEY=local');

      const paths = await getEnvFilePaths(testDir);
      expect(paths).toContain(join(testDir, '.env'));
      expect(paths).toContain(join(testDir, '.env.local'));
    });

    test('includes environment-specific files when env specified', async () => {
      await Bun.write(join(testDir, '.env'), 'KEY=value');
      await Bun.write(join(testDir, '.env.production'), 'KEY=prod');

      const paths = await getEnvFilePaths(testDir, 'production');
      expect(paths).toContain(join(testDir, '.env'));
      expect(paths).toContain(join(testDir, '.env.production'));
    });

    test('does not include environment-specific files when env not specified', async () => {
      await Bun.write(join(testDir, '.env'), 'KEY=value');
      await Bun.write(join(testDir, '.env.production'), 'KEY=prod');

      const paths = await getEnvFilePaths(testDir);
      expect(paths).toContain(join(testDir, '.env'));
      expect(paths).not.toContain(join(testDir, '.env.production'));
    });
  });
});
