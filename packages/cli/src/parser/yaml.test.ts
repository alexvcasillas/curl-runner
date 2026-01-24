import { describe, expect, test } from 'bun:test';
import { YamlParser } from './yaml';

describe('YamlParser.interpolateVariables with store context', () => {
  test('should resolve store variables', () => {
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
    const obj = { url: 'https://api.example.com/users/${store.userId}' };
    const variables = {};
    const storeContext = { userId: '123' };

    const result = YamlParser.interpolateVariables(obj, variables, storeContext);
    expect(result).toEqual({ url: 'https://api.example.com/users/123' });
  });

  test('should resolve multiple store variables in one string', () => {
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
    const obj = { url: 'https://api.example.com/users/${store.userId}/posts/${store.postId}' };
    const variables = {};
    const storeContext = { userId: '123', postId: '456' };

    const result = YamlParser.interpolateVariables(obj, variables, storeContext);
    expect(result).toEqual({ url: 'https://api.example.com/users/123/posts/456' });
  });

  test('should resolve store variables in nested objects', () => {
    const obj = {
      headers: {
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
        Authorization: 'Bearer ${store.token}',
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
        'X-User-Id': '${store.userId}',
      },
    };
    const variables = {};
    const storeContext = { token: 'jwt-token', userId: '123' };

    const result = YamlParser.interpolateVariables(obj, variables, storeContext);
    expect(result).toEqual({
      headers: {
        Authorization: 'Bearer jwt-token',
        'X-User-Id': '123',
      },
    });
  });

  test('should resolve store variables in arrays', () => {
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
    const obj = { ids: ['${store.id1}', '${store.id2}'] };
    const variables = {};
    const storeContext = { id1: '1', id2: '2' };

    const result = YamlParser.interpolateVariables(obj, variables, storeContext);
    expect(result).toEqual({ ids: ['1', '2'] });
  });

  test('should keep unresolved store variables as-is', () => {
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
    const obj = { url: 'https://api.example.com/users/${store.missing}' };
    const variables = {};
    const storeContext = {};

    const result = YamlParser.interpolateVariables(obj, variables, storeContext);
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
    expect(result).toEqual({ url: 'https://api.example.com/users/${store.missing}' });
  });

  test('should mix store variables with static variables', () => {
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
    const obj = { url: '${BASE_URL}/users/${store.userId}' };
    const variables = { BASE_URL: 'https://api.example.com' };
    const storeContext = { userId: '123' };

    const result = YamlParser.interpolateVariables(obj, variables, storeContext);
    expect(result).toEqual({ url: 'https://api.example.com/users/123' });
  });

  test('should work without store context (backwards compatibility)', () => {
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing variable interpolation
    const obj = { url: '${BASE_URL}/users' };
    const variables = { BASE_URL: 'https://api.example.com' };

    const result = YamlParser.interpolateVariables(obj, variables);
    expect(result).toEqual({ url: 'https://api.example.com/users' });
  });

  test('should resolve single store variable as exact value', () => {
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
    const obj = { userId: '${store.userId}' };
    const variables = {};
    const storeContext = { userId: '123' };

    const result = YamlParser.interpolateVariables(obj, variables, storeContext);
    expect(result).toEqual({ userId: '123' });
  });

  test('should mix store variables with dynamic variables', () => {
    const obj = {
      headers: {
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing variable interpolation
        'X-Request-ID': '${UUID}',
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
        Authorization: 'Bearer ${store.token}',
      },
    };
    const variables = {};
    const storeContext = { token: 'my-token' };

    const result = YamlParser.interpolateVariables(obj, variables, storeContext) as typeof obj;
    // UUID should be a valid UUID string
    expect(result.headers['X-Request-ID']).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i,
    );
    expect(result.headers.Authorization).toBe('Bearer my-token');
  });

  test('should resolve store variables in request body', () => {
    const obj = {
      body: {
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
        userId: '${store.userId}',
        name: 'Test User',
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${store.xxx} interpolation
        parentId: '${store.parentId}',
      },
    };
    const variables = {};
    const storeContext = { userId: '123', parentId: '456' };

    const result = YamlParser.interpolateVariables(obj, variables, storeContext);
    expect(result).toEqual({
      body: {
        userId: '123',
        name: 'Test User',
        parentId: '456',
      },
    });
  });
});

describe('YamlParser.resolveVariable', () => {
  test('should resolve store variable', () => {
    const storeContext = { userId: '123' };
    const result = YamlParser.resolveVariable('store.userId', {}, storeContext);
    expect(result).toBe('123');
  });

  test('should return null for missing store variable', () => {
    const storeContext = { other: 'value' };
    const result = YamlParser.resolveVariable('store.missing', {}, storeContext);
    expect(result).toBeNull();
  });

  test('should resolve dynamic variable', () => {
    const result = YamlParser.resolveVariable('UUID', {}, {});
    expect(result).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i);
  });

  test('should resolve static variable', () => {
    const variables = { BASE_URL: 'https://api.example.com' };
    const result = YamlParser.resolveVariable('BASE_URL', variables, {});
    expect(result).toBe('https://api.example.com');
  });

  test('should return null for unknown variable', () => {
    const result = YamlParser.resolveVariable('UNKNOWN', {}, {});
    expect(result).toBeNull();
  });

  test('should prioritize store context over static variables', () => {
    // This test ensures store.X prefix is properly handled
    const variables = { 'store.userId': 'static-value' };
    const storeContext = { userId: 'store-value' };
    const result = YamlParser.resolveVariable('store.userId', variables, storeContext);
    expect(result).toBe('store-value');
  });
});

describe('YamlParser.resolveDynamicVariable', () => {
  test('should resolve UUID to a valid full UUID', () => {
    const result = YamlParser.resolveDynamicVariable('UUID');
    expect(result).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i);
  });

  test('should resolve UUID:short to first 8 characters of a UUID', () => {
    const result = YamlParser.resolveDynamicVariable('UUID:short');
    expect(result).toMatch(/^[0-9a-f]{8}$/i);
    expect(result).toHaveLength(8);
  });

  test('should resolve RANDOM:min-max to a number within range', () => {
    const result = YamlParser.resolveDynamicVariable('RANDOM:1-100');
    expect(result).not.toBeNull();
    const num = Number(result);
    expect(num).toBeGreaterThanOrEqual(1);
    expect(num).toBeLessThanOrEqual(100);
  });

  test('should resolve RANDOM:min-max with large range', () => {
    const result = YamlParser.resolveDynamicVariable('RANDOM:1-1000');
    expect(result).not.toBeNull();
    const num = Number(result);
    expect(num).toBeGreaterThanOrEqual(1);
    expect(num).toBeLessThanOrEqual(1000);
  });

  test('should resolve RANDOM:min-max with same min and max', () => {
    const result = YamlParser.resolveDynamicVariable('RANDOM:5-5');
    expect(result).toBe('5');
  });

  test('should resolve RANDOM:string:length to alphanumeric string of correct length', () => {
    const result = YamlParser.resolveDynamicVariable('RANDOM:string:10');
    expect(result).not.toBeNull();
    expect(result).toHaveLength(10);
    expect(result).toMatch(/^[A-Za-z0-9]+$/);
  });

  test('should resolve RANDOM:string:length with different lengths', () => {
    const result5 = YamlParser.resolveDynamicVariable('RANDOM:string:5');
    const result20 = YamlParser.resolveDynamicVariable('RANDOM:string:20');
    expect(result5).toHaveLength(5);
    expect(result20).toHaveLength(20);
    expect(result5).toMatch(/^[A-Za-z0-9]+$/);
    expect(result20).toMatch(/^[A-Za-z0-9]+$/);
  });

  test('should resolve TIMESTAMP to a numeric string', () => {
    const result = YamlParser.resolveDynamicVariable('TIMESTAMP');
    expect(result).toMatch(/^\d+$/);
  });

  test('should resolve CURRENT_TIME to a numeric string', () => {
    const result = YamlParser.resolveDynamicVariable('CURRENT_TIME');
    expect(result).toMatch(/^\d+$/);
  });

  test('should return null for unknown dynamic variable', () => {
    const result = YamlParser.resolveDynamicVariable('UNKNOWN');
    expect(result).toBeNull();
  });
});

describe('YamlParser.interpolateVariables with new dynamic variables', () => {
  test('should interpolate UUID:short in objects', () => {
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${UUID:short} interpolation
    const obj = { id: '${UUID:short}' };
    const result = YamlParser.interpolateVariables(obj, {}) as typeof obj;
    expect(result.id).toMatch(/^[0-9a-f]{8}$/i);
  });

  test('should interpolate RANDOM:min-max in objects', () => {
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${RANDOM:x-y} interpolation
    const obj = { value: '${RANDOM:1-100}' };
    const result = YamlParser.interpolateVariables(obj, {}) as typeof obj;
    const num = Number(result.value);
    expect(num).toBeGreaterThanOrEqual(1);
    expect(num).toBeLessThanOrEqual(100);
  });

  test('should interpolate RANDOM:string:length in objects', () => {
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing ${RANDOM:string:n} interpolation
    const obj = { token: '${RANDOM:string:16}' };
    const result = YamlParser.interpolateVariables(obj, {}) as typeof obj;
    expect(result.token).toHaveLength(16);
    expect(result.token).toMatch(/^[A-Za-z0-9]+$/);
  });

  test('should interpolate multiple new dynamic variables in one object', () => {
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
