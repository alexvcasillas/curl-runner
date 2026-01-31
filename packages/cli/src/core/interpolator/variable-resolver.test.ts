import { describe, expect, test } from 'bun:test';
import {
  extractVariables,
  formatDate,
  formatTime,
  interpolate,
  resolveDynamicVariable,
  resolveVariable,
} from './variable-resolver';

describe('interpolate', () => {
  describe('basic interpolation', () => {
    test('returns string unchanged when no variables', () => {
      expect(interpolate('hello world', {})).toBe('hello world');
    });

    test('interpolates single variable', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      expect(interpolate('${VAR}', { variables: { VAR: 'value' } })).toBe('value');
    });

    test('interpolates variable in string', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      expect(interpolate('hello ${NAME}!', { variables: { NAME: 'world' } })).toBe('hello world!');
    });

    test('interpolates multiple variables', () => {
      const result = interpolate(
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
        '${GREETING} ${NAME}!',
        { variables: { GREETING: 'Hello', NAME: 'World' } },
      );
      expect(result).toBe('Hello World!');
    });

    test('keeps unresolved variables as-is', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      expect(interpolate('${UNKNOWN}', {})).toBe('${UNKNOWN}');
    });
  });

  describe('objects and arrays', () => {
    test('interpolates nested objects', () => {
      const obj = {
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
        url: '${BASE_URL}/api',
        headers: {
          // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
          Authorization: 'Bearer ${TOKEN}',
        },
      };
      const result = interpolate(obj, {
        variables: { BASE_URL: 'https://api.example.com', TOKEN: 'secret' },
      });
      expect(result).toEqual({
        url: 'https://api.example.com/api',
        headers: { Authorization: 'Bearer secret' },
      });
    });

    test('interpolates arrays', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      const arr = ['${A}', '${B}', '${C}'];
      const result = interpolate(arr, { variables: { A: '1', B: '2', C: '3' } });
      expect(result).toEqual(['1', '2', '3']);
    });

    test('handles null and undefined', () => {
      expect(interpolate(null, {})).toBeNull();
      expect(interpolate(undefined, {})).toBeUndefined();
    });

    test('handles numbers and booleans', () => {
      expect(interpolate(42, {})).toBe(42);
      expect(interpolate(true, {})).toBe(true);
    });
  });

  describe('store context', () => {
    test('resolves store variables', () => {
      const result = interpolate(
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
        '${store.userId}',
        { storeContext: { userId: '123' } },
      );
      expect(result).toBe('123');
    });

    test('resolves multiple store variables', () => {
      const result = interpolate(
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
        '/users/${store.userId}/posts/${store.postId}',
        { storeContext: { userId: '123', postId: '456' } },
      );
      expect(result).toBe('/users/123/posts/456');
    });

    test('keeps unresolved store variables as-is', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      expect(interpolate('${store.missing}', { storeContext: {} })).toBe('${store.missing}');
    });

    test('mixes store and static variables', () => {
      const result = interpolate(
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
        '${BASE}/users/${store.userId}',
        { variables: { BASE: 'https://api.example.com' }, storeContext: { userId: '123' } },
      );
      expect(result).toBe('https://api.example.com/users/123');
    });
  });

  describe('string transforms', () => {
    test('transforms to uppercase', () => {
      const result = interpolate(
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
        '${ENV:upper}',
        { variables: { ENV: 'production' } },
      );
      expect(result).toBe('PRODUCTION');
    });

    test('transforms to lowercase', () => {
      const result = interpolate(
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
        '${RESOURCE:lower}',
        { variables: { RESOURCE: 'USERS' } },
      );
      expect(result).toBe('users');
    });

    test('keeps unresolved transform as-is', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      expect(interpolate('${MISSING:upper}', {})).toBe('${MISSING:upper}');
    });
  });

  describe('default values', () => {
    test('uses default when variable not set', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      expect(interpolate('${TIMEOUT:5000}', {})).toBe('5000');
    });

    test('uses variable value over default', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      expect(interpolate('${TIMEOUT:5000}', { variables: { TIMEOUT: '10000' } })).toBe('10000');
    });

    test('handles nested defaults', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      expect(interpolate('${DB_HOST:${FALLBACK:localhost}}', {})).toBe('localhost');
    });

    test('resolves first level of nested default', () => {
      const result = interpolate(
        // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
        '${DB_HOST:${FALLBACK:localhost}}',
        { variables: { FALLBACK: 'backup-host' } },
      );
      expect(result).toBe('backup-host');
    });

    test('handles empty default', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      expect(interpolate('${OPTIONAL:}', {})).toBe('');
    });

    test('handles default with special characters', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      expect(interpolate('${URL:https://api.example.com/v1}', {})).toBe(
        'https://api.example.com/v1',
      );
    });
  });

  describe('dynamic variables', () => {
    test('generates UUID', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      const result = interpolate('${UUID}', {}) as string;
      expect(result).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i);
    });

    test('generates short UUID', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      const result = interpolate('${UUID:short}', {}) as string;
      expect(result).toMatch(/^[0-9a-f]{8}$/i);
    });

    test('generates random number in range', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      const result = interpolate('${RANDOM:1-100}', {}) as string;
      const num = Number(result);
      expect(num).toBeGreaterThanOrEqual(1);
      expect(num).toBeLessThanOrEqual(100);
    });

    test('generates random string', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      const result = interpolate('${RANDOM:string:16}', {}) as string;
      expect(result).toHaveLength(16);
      expect(result).toMatch(/^[A-Za-z0-9]+$/);
    });

    test('generates timestamp', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      const result = interpolate('${TIMESTAMP}', {}) as string;
      expect(result).toMatch(/^\d+$/);
    });

    test('formats date', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      const result = interpolate('${DATE:YYYY-MM-DD}', {}) as string;
      expect(result).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    });

    test('formats time', () => {
      // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing interpolation
      const result = interpolate('${TIME:HH:mm:ss}', {}) as string;
      expect(result).toMatch(/^\d{2}:\d{2}:\d{2}$/);
    });
  });
});

describe('resolveVariable', () => {
  test('resolves store variable', () => {
    expect(resolveVariable('store.userId', {}, { userId: '123' })).toBe('123');
  });

  test('returns null for missing store variable', () => {
    expect(resolveVariable('store.missing', {}, {})).toBeNull();
  });

  test('resolves static variable', () => {
    expect(resolveVariable('BASE_URL', { BASE_URL: 'https://example.com' })).toBe(
      'https://example.com',
    );
  });

  test('returns null for unknown variable', () => {
    expect(resolveVariable('UNKNOWN', {})).toBeNull();
  });

  test('prioritizes store context over static variables', () => {
    const result = resolveVariable(
      'store.userId',
      { 'store.userId': 'static' },
      { userId: 'store' },
    );
    expect(result).toBe('store');
  });

  test('resolves dynamic variable', () => {
    const result = resolveVariable('UUID', {});
    expect(result).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i);
  });

  test('applies uppercase transform', () => {
    expect(resolveVariable('ENV:upper', { ENV: 'production' })).toBe('PRODUCTION');
  });

  test('applies lowercase transform', () => {
    expect(resolveVariable('RESOURCE:lower', { RESOURCE: 'USERS' })).toBe('users');
  });

  test('uses default value', () => {
    expect(resolveVariable('TIMEOUT:5000', {})).toBe('5000');
  });

  test('variable value overrides default', () => {
    expect(resolveVariable('TIMEOUT:5000', { TIMEOUT: '10000' })).toBe('10000');
  });
});

describe('resolveDynamicVariable', () => {
  test('resolves UUID', () => {
    const result = resolveDynamicVariable('UUID');
    expect(result).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i);
  });

  test('resolves UUID:short', () => {
    const result = resolveDynamicVariable('UUID:short');
    expect(result).toMatch(/^[0-9a-f]{8}$/i);
  });

  test('resolves RANDOM:min-max', () => {
    const result = resolveDynamicVariable('RANDOM:1-100');
    const num = Number(result);
    expect(num).toBeGreaterThanOrEqual(1);
    expect(num).toBeLessThanOrEqual(100);
  });

  test('resolves RANDOM:string:length', () => {
    const result = resolveDynamicVariable('RANDOM:string:10');
    expect(result).toHaveLength(10);
    expect(result).toMatch(/^[A-Za-z0-9]+$/);
  });

  test('resolves TIMESTAMP', () => {
    const result = resolveDynamicVariable('TIMESTAMP');
    expect(result).toMatch(/^\d+$/);
  });

  test('resolves CURRENT_TIME', () => {
    const result = resolveDynamicVariable('CURRENT_TIME');
    expect(result).toMatch(/^\d+$/);
  });

  test('resolves DATE:format', () => {
    const result = resolveDynamicVariable('DATE:YYYY-MM-DD');
    expect(result).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });

  test('resolves TIME:format', () => {
    const result = resolveDynamicVariable('TIME:HH:mm:ss');
    expect(result).toMatch(/^\d{2}:\d{2}:\d{2}$/);
  });

  test('returns null for unknown pattern', () => {
    expect(resolveDynamicVariable('UNKNOWN')).toBeNull();
  });
});

describe('extractVariables', () => {
  test('extracts single variable', () => {
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing extraction
    const refs = extractVariables('${VAR}');
    expect(refs).toHaveLength(1);
    expect(refs[0]).toEqual({ start: 0, end: 6, name: 'VAR' });
  });

  test('extracts multiple variables', () => {
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing extraction
    const refs = extractVariables('${A} and ${B}');
    expect(refs).toHaveLength(2);
    expect(refs[0].name).toBe('A');
    expect(refs[1].name).toBe('B');
  });

  test('extracts nested braces', () => {
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing extraction
    const refs = extractVariables('${VAR:${DEFAULT}}');
    expect(refs).toHaveLength(1);
    // biome-ignore lint/suspicious/noTemplateCurlyInString: Testing extraction
    expect(refs[0].name).toBe('VAR:${DEFAULT}');
  });

  test('returns empty array for no variables', () => {
    expect(extractVariables('no variables here')).toEqual([]);
  });

  test('handles unmatched braces', () => {
    const refs = extractVariables('${incomplete');
    expect(refs).toEqual([]);
  });
});

describe('formatDate', () => {
  test('formats YYYY-MM-DD', () => {
    const date = new Date(2024, 0, 15); // Jan 15, 2024
    expect(formatDate(date, 'YYYY-MM-DD')).toBe('2024-01-15');
  });

  test('formats DD/MM/YYYY', () => {
    const date = new Date(2024, 11, 25); // Dec 25, 2024
    expect(formatDate(date, 'DD/MM/YYYY')).toBe('25/12/2024');
  });

  test('formats partial patterns', () => {
    const date = new Date(2024, 5, 10); // Jun 10, 2024
    expect(formatDate(date, 'YYYY')).toBe('2024');
    expect(formatDate(date, 'MM-DD')).toBe('06-10');
  });
});

describe('formatTime', () => {
  test('formats HH:mm:ss', () => {
    const date = new Date(2024, 0, 1, 14, 30, 45);
    expect(formatTime(date, 'HH:mm:ss')).toBe('14:30:45');
  });

  test('formats HH:mm', () => {
    const date = new Date(2024, 0, 1, 9, 5, 0);
    expect(formatTime(date, 'HH:mm')).toBe('09:05');
  });

  test('pads single digits', () => {
    const date = new Date(2024, 0, 1, 1, 2, 3);
    expect(formatTime(date, 'HH:mm:ss')).toBe('01:02:03');
  });
});
