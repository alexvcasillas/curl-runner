import { describe, expect, test } from 'bun:test';
import {
  getArrayValue,
  isArraySelector,
  isRangePattern,
  isRegexPattern,
  validateBodyProperties,
  validateHeaders,
  validateRangePattern,
  validateRegexPattern,
  validateResponse,
  validateStatus,
  validateValue,
} from './validators';

describe('validateResponse', () => {
  test('returns success when no expect config', () => {
    // biome-ignore lint/suspicious/noExplicitAny: Testing undefined expect config
    const result = validateResponse({ status: 200 }, undefined as any);
    expect(result.success).toBe(true);
  });

  test('validates status code', () => {
    const result = validateResponse({ status: 200 }, { status: 200 });
    expect(result.success).toBe(true);
  });

  test('fails on status mismatch', () => {
    const result = validateResponse({ status: 404 }, { status: 200 });
    expect(result.success).toBe(false);
    expect(result.error).toContain('Expected status 200');
  });

  test('validates headers', () => {
    const result = validateResponse(
      { status: 200, headers: { 'content-type': 'application/json' } },
      { headers: { 'content-type': 'application/json' } },
    );
    expect(result.success).toBe(true);
  });

  test('validates body', () => {
    const result = validateResponse(
      { status: 200, body: { name: 'test' } },
      { body: { name: 'test' } },
    );
    expect(result.success).toBe(true);
  });

  test('validates response time', () => {
    const result = validateResponse(
      { status: 200, metrics: { duration: 50 } },
      { responseTime: '<= 100' },
    );
    expect(result.success).toBe(true);
  });

  test('fails response time validation', () => {
    const result = validateResponse(
      { status: 200, metrics: { duration: 150 } },
      { responseTime: '<= 100' },
    );
    expect(result.success).toBe(false);
    expect(result.error).toContain('response time');
  });

  describe('failure expectation (negative testing)', () => {
    test('succeeds when expecting failure and status is 4xx', () => {
      const result = validateResponse({ status: 404 }, { failure: true, status: 404 });
      expect(result.success).toBe(true);
    });

    test('succeeds when expecting failure and status is 5xx', () => {
      const result = validateResponse({ status: 500 }, { failure: true, status: 500 });
      expect(result.success).toBe(true);
    });

    test('fails when expecting failure but got success status', () => {
      const result = validateResponse({ status: 200 }, { failure: true });
      expect(result.success).toBe(false);
      expect(result.error).toContain('Expected request to fail');
    });
  });
});

describe('validateStatus', () => {
  test('validates single status', () => {
    expect(validateStatus(200, 200)).toEqual([]);
    expect(validateStatus(404, 200)).toHaveLength(1);
  });

  test('validates multiple statuses', () => {
    expect(validateStatus(200, [200, 201])).toEqual([]);
    expect(validateStatus(201, [200, 201])).toEqual([]);
    expect(validateStatus(404, [200, 201])).toHaveLength(1);
  });

  test('handles undefined actual status', () => {
    expect(validateStatus(undefined, 200)).toHaveLength(1);
  });
});

describe('validateHeaders', () => {
  test('validates matching headers', () => {
    const errors = validateHeaders(
      { 'content-type': 'application/json' },
      { 'content-type': 'application/json' },
    );
    expect(errors).toEqual([]);
  });

  test('validates case-insensitive headers (lowercase actual)', () => {
    const errors = validateHeaders(
      { 'content-type': 'application/json' },
      { 'Content-Type': 'application/json' },
    );
    expect(errors).toEqual([]);
  });

  test('returns error for mismatched headers', () => {
    const errors = validateHeaders(
      { 'content-type': 'text/html' },
      { 'content-type': 'application/json' },
    );
    expect(errors).toHaveLength(1);
    expect(errors[0]).toContain('content-type');
  });

  test('returns error for missing headers', () => {
    const errors = validateHeaders({}, { 'x-custom': 'value' });
    expect(errors).toHaveLength(1);
  });
});

describe('validateBodyProperties', () => {
  test('validates simple object', () => {
    const errors = validateBodyProperties({ name: 'test' }, { name: 'test' }, '');
    expect(errors).toEqual([]);
  });

  test('validates nested object', () => {
    const errors = validateBodyProperties(
      { user: { name: 'test' } },
      { user: { name: 'test' } },
      '',
    );
    expect(errors).toEqual([]);
  });

  test('returns error for mismatched value', () => {
    const errors = validateBodyProperties({ name: 'actual' }, { name: 'expected' }, '');
    expect(errors).toHaveLength(1);
    expect(errors[0]).toContain('name');
  });

  test('validates value against array of acceptable values', () => {
    // Scalar actual, array expected → "one of" matching
    const errors = validateBodyProperties(1, [1, 2, 3], '');
    expect(errors).toEqual([]);
  });

  test('validates primitive body', () => {
    const errors = validateBodyProperties('hello', 'hello', '');
    expect(errors).toEqual([]);
  });

  test('array containment with objects (partial fields)', () => {
    const actual = [
      { type: 'paragraph', text: 'Hello' },
      { type: 'heading', text: 'World' },
    ];
    const expected = [{ type: 'paragraph' }];
    const errors = validateBodyProperties(actual, expected, '');
    expect(errors).toEqual([]);
  });

  test('array containment with full match', () => {
    const actual = [
      { type: 'paragraph', text: 'Hello' },
      { type: 'heading', text: 'World' },
    ];
    const expected = [{ type: 'paragraph' }, { type: 'heading' }];
    const errors = validateBodyProperties(actual, expected, '');
    expect(errors).toEqual([]);
  });

  test('array mismatch when expected item not found', () => {
    const actual = [{ type: 'paragraph', text: 'Hello' }];
    const expected = [{ type: 'heading' }];
    const errors = validateBodyProperties(actual, expected, '');
    expect(errors).toHaveLength(1);
    expect(errors[0]).toContain('no match found');
  });

  test('primitive array containment', () => {
    const errors = validateBodyProperties([1, 2, 3], [1, 2], '');
    expect(errors).toEqual([]);
  });

  test('nested array field validation', () => {
    const actual = { content: [{ type: 'paragraph' }, { type: 'heading' }] };
    const expected = { content: [{ type: 'paragraph' }] };
    const errors = validateBodyProperties(actual, expected, '');
    expect(errors).toEqual([]);
  });

  test('scalar vs array backward compat (one of)', () => {
    const errors = validateBodyProperties(2, [1, 2, 3], '');
    expect(errors).toEqual([]);
    const errorsNoMatch = validateBodyProperties(5, [1, 2, 3], '');
    expect(errorsNoMatch).toHaveLength(1);
  });
});

describe('validateValue', () => {
  test('accepts wildcard', () => {
    expect(validateValue('anything', '*', 'path').isValid).toBe(true);
    expect(validateValue(123, '*', 'path').isValid).toBe(true);
    expect(validateValue(null, '*', 'path').isValid).toBe(true);
  });

  test('validates exact match', () => {
    expect(validateValue('test', 'test', 'path').isValid).toBe(true);
    expect(validateValue(42, 42, 'path').isValid).toBe(true);
    expect(validateValue(true, true, 'path').isValid).toBe(true);
  });

  test('returns error for mismatch', () => {
    const result = validateValue('actual', 'expected', 'myField');
    expect(result.isValid).toBe(false);
    expect(result.error).toContain('myField');
    expect(result.error).toContain('expected');
    expect(result.error).toContain('actual');
  });

  test('validates null', () => {
    expect(validateValue(null, null, 'path').isValid).toBe(true);
    expect(validateValue(null, 'null', 'path').isValid).toBe(true);
    expect(validateValue('not null', null, 'path').isValid).toBe(false);
  });

  test('validates array of acceptable values', () => {
    expect(validateValue('a', ['a', 'b', 'c'], 'path').isValid).toBe(true);
    expect(validateValue('b', ['a', 'b', 'c'], 'path').isValid).toBe(true);
    expect(validateValue('d', ['a', 'b', 'c'], 'path').isValid).toBe(false);
  });

  test('validates wildcard in array', () => {
    expect(validateValue('anything', ['*'], 'path').isValid).toBe(true);
  });

  test('validates regex in array', () => {
    expect(validateValue('test123', ['^test\\d+$'], 'path').isValid).toBe(true);
  });

  test('validates range in array', () => {
    expect(validateValue(50, ['>= 0', '<= 100'], 'path').isValid).toBe(true);
  });

  test('validates regex pattern', () => {
    expect(validateValue('test123', '^test\\d+$', 'path').isValid).toBe(true);
    expect(validateValue('other', '^test\\d+$', 'path').isValid).toBe(false);
  });

  test('validates range pattern', () => {
    expect(validateValue(50, '>= 0', 'path').isValid).toBe(true);
    expect(validateValue(-1, '>= 0', 'path').isValid).toBe(false);
  });
});

describe('isRegexPattern', () => {
  test('detects start anchor', () => {
    expect(isRegexPattern('^test')).toBe(true);
  });

  test('detects end anchor', () => {
    expect(isRegexPattern('test$')).toBe(true);
  });

  test('detects escape sequences', () => {
    expect(isRegexPattern('\\d+')).toBe(true);
    expect(isRegexPattern('\\w+')).toBe(true);
    expect(isRegexPattern('\\s+')).toBe(true);
  });

  test('returns false for plain strings', () => {
    expect(isRegexPattern('hello')).toBe(false);
    expect(isRegexPattern('C++')).toBe(false);
    expect(isRegexPattern('really?')).toBe(false);
    expect(isRegexPattern('*-')).toBe(false);
    expect(isRegexPattern('[]')).toBe(false);
    expect(isRegexPattern('[a-z]')).toBe(false);
  });
});

describe('validateRegexPattern', () => {
  test('validates matching pattern', () => {
    expect(validateRegexPattern('test123', '^test\\d+$')).toBe(true);
    expect(validateRegexPattern('hello world', 'world')).toBe(true);
  });

  test('returns false for non-matching pattern', () => {
    expect(validateRegexPattern('test', '^\\d+$')).toBe(false);
  });

  test('handles invalid regex', () => {
    expect(validateRegexPattern('test', '[invalid')).toBe(false);
  });

  test('converts non-string values', () => {
    expect(validateRegexPattern(123, '\\d+')).toBe(true);
    expect(validateRegexPattern(true, 'true')).toBe(true);
  });
});

describe('isRangePattern', () => {
  test('detects comparison operators', () => {
    expect(isRangePattern('>= 0')).toBe(true);
    expect(isRangePattern('<= 100')).toBe(true);
    expect(isRangePattern('> 10')).toBe(true);
    expect(isRangePattern('< 50')).toBe(true);
  });

  test('detects combined conditions', () => {
    expect(isRangePattern('>= 0, <= 100')).toBe(true);
  });

  test('returns false for plain numbers', () => {
    expect(isRangePattern('100')).toBe(false);
  });

  test('returns false for plain strings', () => {
    expect(isRangePattern('hello')).toBe(false);
  });

  test('returns false for zip codes', () => {
    expect(isRangePattern('92998-3874')).toBe(false);
  });
});

describe('validateRangePattern', () => {
  test('validates greater than', () => {
    expect(validateRangePattern(50, '> 10')).toBe(true);
    expect(validateRangePattern(10, '> 10')).toBe(false);
  });

  test('validates greater than or equal', () => {
    expect(validateRangePattern(10, '>= 10')).toBe(true);
    expect(validateRangePattern(9, '>= 10')).toBe(false);
  });

  test('validates less than', () => {
    expect(validateRangePattern(5, '< 10')).toBe(true);
    expect(validateRangePattern(10, '< 10')).toBe(false);
  });

  test('validates less than or equal', () => {
    expect(validateRangePattern(10, '<= 10')).toBe(true);
    expect(validateRangePattern(11, '<= 10')).toBe(false);
  });

  test('validates combined conditions', () => {
    expect(validateRangePattern(50, '>= 0, <= 100')).toBe(true);
    expect(validateRangePattern(-1, '>= 0, <= 100')).toBe(false);
    expect(validateRangePattern(101, '>= 0, <= 100')).toBe(false);
  });

  test('returns false for non-numeric values', () => {
    expect(validateRangePattern('hello', '>= 0')).toBe(false);
  });

  test('handles negative numbers', () => {
    expect(validateRangePattern(-5, '>= -10')).toBe(true);
    expect(validateRangePattern(-15, '>= -10')).toBe(false);
  });

  test('handles decimal numbers', () => {
    expect(validateRangePattern(3.14, '>= 3.0, <= 4.0')).toBe(true);
  });
});

describe('isArraySelector', () => {
  test('detects bracket notation', () => {
    expect(isArraySelector('[0]')).toBe(true);
    expect(isArraySelector('[-1]')).toBe(true);
    expect(isArraySelector('[*]')).toBe(true);
  });

  test('detects wildcard', () => {
    expect(isArraySelector('*')).toBe(true);
  });

  test('detects slice', () => {
    expect(isArraySelector('slice(0,3)')).toBe(true);
  });

  test('returns false for regular keys', () => {
    expect(isArraySelector('name')).toBe(false);
    expect(isArraySelector('user_id')).toBe(false);
  });
});

describe('getArrayValue', () => {
  const array = ['a', 'b', 'c', 'd', 'e'];

  test('returns entire array for wildcard', () => {
    expect(getArrayValue(array, '*')).toEqual(array);
  });

  test('returns element by positive index', () => {
    expect(getArrayValue(array, '[0]')).toBe('a');
    expect(getArrayValue(array, '[2]')).toBe('c');
  });

  test('returns element by negative index', () => {
    expect(getArrayValue(array, '[-1]')).toBe('e');
    expect(getArrayValue(array, '[-2]')).toBe('d');
  });

  test('returns entire array for [*]', () => {
    expect(getArrayValue(array, '[*]')).toEqual(array);
  });

  test('returns slice', () => {
    expect(getArrayValue(array, 'slice(0,2)')).toEqual(['a', 'b']);
    expect(getArrayValue(array, 'slice(1,3)')).toEqual(['b', 'c']);
  });

  test('returns slice to end', () => {
    expect(getArrayValue(array, 'slice(2)')).toEqual(['c', 'd', 'e']);
  });

  test('returns undefined for invalid selector', () => {
    expect(getArrayValue(array, 'invalid')).toBeUndefined();
  });
});
