import { describe, expect, test } from 'bun:test';
import { YamlParser } from './yaml';

describe('YamlParser.interpolateVariables', () => {
  describe('store context', () => {
    test('resolves store variables', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
      const obj = { url: 'https://api.example.com/users/${store.userId}' };
      const result = YamlParser.interpolateVariables(obj, {}, { userId: '123' });
      expect(result).toEqual({ url: 'https://api.example.com/users/123' });
    });

    test('resolves multiple store variables in one string', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
      const obj = { url: 'https://api.example.com/users/${store.userId}/posts/${store.postId}' };
      const result = YamlParser.interpolateVariables(obj, {}, { userId: '123', postId: '456' });
      expect(result).toEqual({ url: 'https://api.example.com/users/123/posts/456' });
    });

    test('resolves store variables in nested objects', () => {
      const obj = {
        headers: {
          // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
          Authorization: 'Bearer ${store.token}',
          // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
          'X-User-Id': '${store.userId}',
        },
      };
      const result = YamlParser.interpolateVariables(
        obj,
        {},
        { token: 'jwt-token', userId: '123' },
      );
      expect(result).toEqual({
        headers: {
          Authorization: 'Bearer jwt-token',
          'X-User-Id': '123',
        },
      });
    });

    test('resolves store variables in arrays', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
      const obj = { ids: ['${store.id1}', '${store.id2}'] };
      const result = YamlParser.interpolateVariables(obj, {}, { id1: '1', id2: '2' });
      expect(result).toEqual({ ids: ['1', '2'] });
    });

    test('keeps unresolved store variables as-is', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
      const obj = { url: 'https://api.example.com/users/${store.missing}' };
      const result = YamlParser.interpolateVariables(obj, {}, {});
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
      expect(result).toEqual({ url: 'https://api.example.com/users/${store.missing}' });
    });

    test('mixes store variables with static variables', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
      const obj = { url: '${BASE_URL}/users/${store.userId}' };
      const result = YamlParser.interpolateVariables(
        obj,
        { BASE_URL: 'https://api.example.com' },
        { userId: '123' },
      );
      expect(result).toEqual({ url: 'https://api.example.com/users/123' });
    });

    test('works without store context (backwards compatibility)', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing variable interpolation
      const obj = { url: '${BASE_URL}/users' };
      const result = YamlParser.interpolateVariables(obj, { BASE_URL: 'https://api.example.com' });
      expect(result).toEqual({ url: 'https://api.example.com/users' });
    });

    test('resolves single store variable as exact value', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
      const obj = { userId: '${store.userId}' };
      const result = YamlParser.interpolateVariables(obj, {}, { userId: '123' });
      expect(result).toEqual({ userId: '123' });
    });

    test('mixes store variables with dynamic variables', () => {
      const obj = {
        headers: {
          // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing variable interpolation
          'X-Request-ID': '${UUID}',
          // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
          Authorization: 'Bearer ${store.token}',
        },
      };
      const result = YamlParser.interpolateVariables(obj, {}, { token: 'my-token' }) as typeof obj;
      expect(result.headers['X-Request-ID']).toMatch(
        /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i,
      );
      expect(result.headers.Authorization).toBe('Bearer my-token');
    });

    test('resolves store variables in request body', () => {
      const obj = {
        body: {
          // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
          userId: '${store.userId}',
          name: 'Test User',
          // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
          parentId: '${store.parentId}',
        },
      };
      const result = YamlParser.interpolateVariables(obj, {}, { userId: '123', parentId: '456' });
      expect(result).toEqual({
        body: {
          userId: '123',
          name: 'Test User',
          parentId: '456',
        },
      });
    });
  });

  describe('string transforms', () => {
    test('transforms to uppercase with :upper', () => {
      const obj = {
        headers: {
          // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing string transform
          'X-Environment': '${ENV:upper}',
        },
      };
      const result = YamlParser.interpolateVariables(obj, { ENV: 'production' });
      expect(result).toEqual({ headers: { 'X-Environment': 'PRODUCTION' } });
    });

    test('transforms to lowercase with :lower', () => {
      const obj = {
        headers: {
          // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing string transform
          'X-Resource': '${RESOURCE:lower}',
        },
      };
      const result = YamlParser.interpolateVariables(obj, { RESOURCE: 'USERS' });
      expect(result).toEqual({ headers: { 'X-Resource': 'users' } });
    });

    test('mixes transforms with regular variables', () => {
      const obj = {
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing string transform
        url: '${BASE_URL}/${RESOURCE:lower}',
        headers: {
          // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing string transform
          'X-Environment': '${ENV:upper}',
        },
      };
      const result = YamlParser.interpolateVariables(obj, {
        BASE_URL: 'https://api.example.com',
        RESOURCE: 'Users',
        ENV: 'staging',
      });
      expect(result).toEqual({
        url: 'https://api.example.com/users',
        headers: { 'X-Environment': 'STAGING' },
      });
    });

    test('keeps unresolved transforms as-is', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing string transform
      const obj = { value: '${MISSING:upper}' };
      const result = YamlParser.interpolateVariables(obj, {});
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing string transform
      expect(result).toEqual({ value: '${MISSING:upper}' });
    });
  });

  describe('default values', () => {
    test('uses default when variable not set', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing variable interpolation
      const obj = { timeout: '${API_TIMEOUT:5000}' };
      const result = YamlParser.interpolateVariables(obj, {});
      expect(result).toEqual({ timeout: '5000' });
    });

    test('uses variable value over default', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing variable interpolation
      const obj = { timeout: '${API_TIMEOUT:5000}' };
      const result = YamlParser.interpolateVariables(obj, { API_TIMEOUT: '10000' });
      expect(result).toEqual({ timeout: '10000' });
    });

    test('handles multiple variables with defaults', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing variable interpolation
      const obj = { url: '${BASE_URL:https://api.example.com}/${VERSION:v1}/users' };
      const result = YamlParser.interpolateVariables(obj, {});
      expect(result).toEqual({ url: 'https://api.example.com/v1/users' });
    });

    test('mixes defaults with regular variables', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing variable interpolation
      const obj = { url: '${BASE_URL}/api/${VERSION:v1}/users' };
      const result = YamlParser.interpolateVariables(obj, { BASE_URL: 'https://prod.example.com' });
      expect(result).toEqual({ url: 'https://prod.example.com/api/v1/users' });
    });

    test('handles nested defaults', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing variable interpolation
      const obj = { host: '${DATABASE_HOST:${DB_HOST:localhost}}' };
      const result = YamlParser.interpolateVariables(obj, {});
      expect(result).toEqual({ host: 'localhost' });
    });
  });

  describe('dynamic variables', () => {
    test('generates UUID', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing variable interpolation
      const obj = { id: '${UUID}' };
      const result = YamlParser.interpolateVariables(obj, {}) as { id: string };
      expect(result.id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i);
    });

    test('generates short UUID', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${UUID:short} interpolation
      const obj = { id: '${UUID:short}' };
      const result = YamlParser.interpolateVariables(obj, {}) as { id: string };
      expect(result.id).toMatch(/^[0-9a-f]{8}$/i);
    });

    test('generates random number in range', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${RANDOM:x-y} interpolation
      const obj = { value: '${RANDOM:1-100}' };
      const result = YamlParser.interpolateVariables(obj, {}) as { value: string };
      const num = Number(result.value);
      expect(num).toBeGreaterThanOrEqual(1);
      expect(num).toBeLessThanOrEqual(100);
    });

    test('generates random string', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${RANDOM:string:n} interpolation
      const obj = { token: '${RANDOM:string:16}' };
      const result = YamlParser.interpolateVariables(obj, {}) as { token: string };
      expect(result.token).toHaveLength(16);
      expect(result.token).toMatch(/^[A-Za-z0-9]+$/);
    });

    test('generates timestamp', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing variable interpolation
      const obj = { ts: '${TIMESTAMP}' };
      const result = YamlParser.interpolateVariables(obj, {}) as { ts: string };
      expect(result.ts).toMatch(/^\d+$/);
    });

    test('formats date', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing variable interpolation
      const obj = { date: '${DATE:YYYY-MM-DD}' };
      const result = YamlParser.interpolateVariables(obj, {}) as { date: string };
      expect(result.date).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    });

    test('formats time', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing variable interpolation
      const obj = { time: '${TIME:HH:mm:ss}' };
      const result = YamlParser.interpolateVariables(obj, {}) as { time: string };
      expect(result.time).toMatch(/^\d{2}:\d{2}:\d{2}$/);
    });

    test('interpolates multiple dynamic variables', () => {
      const obj = {
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing dynamic variable interpolation
        sessionId: '${UUID:short}',
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing dynamic variable interpolation
        randomNum: '${RANDOM:1-1000}',
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing dynamic variable interpolation
        randomStr: '${RANDOM:string:10}',
      };
      const result = YamlParser.interpolateVariables(obj, {}) as typeof obj;

      expect(result.sessionId).toMatch(/^[0-9a-f]{8}$/i);
      const num = Number(result.randomNum);
      expect(num).toBeGreaterThanOrEqual(1);
      expect(num).toBeLessThanOrEqual(1000);
      expect(result.randomStr).toHaveLength(10);
      expect(result.randomStr).toMatch(/^[A-Za-z0-9]+$/);
    });
  });
});

describe('YamlParser.mergeConfigs', () => {
  test('merges base and override configs', () => {
    const base = { headers: { 'Content-Type': 'application/json' } };
    const override = { url: 'https://api.example.com', method: 'GET' as const };
    const result = YamlParser.mergeConfigs(base, override);
    expect(result.url).toBe('https://api.example.com');
    expect(result.headers).toEqual({ 'Content-Type': 'application/json' });
  });

  test('override takes precedence for same keys', () => {
    const base = { headers: { 'X-Custom': 'base' }, timeout: 5000 };
    const override = {
      url: 'https://api.example.com',
      method: 'GET' as const,
      headers: { 'X-Custom': 'override' },
    };
    const result = YamlParser.mergeConfigs(base, override);
    expect(result.headers?.['X-Custom']).toBe('override');
  });

  test('merges nested objects', () => {
    const base = { params: { a: '1' }, variables: { x: 'base' } };
    const override = {
      url: 'https://api.example.com',
      method: 'GET' as const,
      params: { b: '2' },
      variables: { y: 'override' },
    };
    const result = YamlParser.mergeConfigs(base, override);
    expect(result.params).toEqual({ a: '1', b: '2' });
    expect(result.variables).toEqual({ x: 'base', y: 'override' });
  });
});
