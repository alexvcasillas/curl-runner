import { describe, expect, test } from 'bun:test';
import type { ConditionExpression, ResponseStoreContext, WhenCondition } from '../types/config';
import { evaluateCondition, evaluateExpression, parseStringCondition } from './condition-evaluator';

describe('parseStringCondition', () => {
  test('should parse equality condition', () => {
    const result = parseStringCondition('store.status == 200');
    expect(result).toEqual({
      left: 'store.status',
      operator: '==',
      right: 200,
    });
  });

  test('should parse inequality condition', () => {
    const result = parseStringCondition('store.type != admin');
    expect(result).toEqual({
      left: 'store.type',
      operator: '!=',
      right: 'admin',
    });
  });

  test('should parse greater than condition', () => {
    const result = parseStringCondition('store.count > 10');
    expect(result).toEqual({
      left: 'store.count',
      operator: '>',
      right: 10,
    });
  });

  test('should parse less than condition', () => {
    const result = parseStringCondition('store.price < 100.5');
    expect(result).toEqual({
      left: 'store.price',
      operator: '<',
      right: 100.5,
    });
  });

  test('should parse greater than or equal condition', () => {
    const result = parseStringCondition('store.version >= 2');
    expect(result).toEqual({
      left: 'store.version',
      operator: '>=',
      right: 2,
    });
  });

  test('should parse less than or equal condition', () => {
    const result = parseStringCondition('store.age <= 65');
    expect(result).toEqual({
      left: 'store.age',
      operator: '<=',
      right: 65,
    });
  });

  test('should parse contains condition', () => {
    const result = parseStringCondition('store.message contains error');
    expect(result).toEqual({
      left: 'store.message',
      operator: 'contains',
      right: 'error',
    });
  });

  test('should parse matches condition', () => {
    const result = parseStringCondition('store.email matches ^[a-z]+@');
    expect(result).toEqual({
      left: 'store.email',
      operator: 'matches',
      right: '^[a-z]+@',
    });
  });

  test('should parse exists condition', () => {
    const result = parseStringCondition('store.userId exists');
    expect(result).toEqual({
      left: 'store.userId',
      operator: 'exists',
    });
  });

  test('should parse not-exists condition', () => {
    const result = parseStringCondition('store.error not-exists');
    expect(result).toEqual({
      left: 'store.error',
      operator: 'not-exists',
    });
  });

  test('should parse boolean true', () => {
    const result = parseStringCondition('store.enabled == true');
    expect(result).toEqual({
      left: 'store.enabled',
      operator: '==',
      right: true,
    });
  });

  test('should parse boolean false', () => {
    const result = parseStringCondition('store.disabled == false');
    expect(result).toEqual({
      left: 'store.disabled',
      operator: '==',
      right: false,
    });
  });

  test('should parse quoted strings', () => {
    const result = parseStringCondition('store.name == "John Doe"');
    expect(result).toEqual({
      left: 'store.name',
      operator: '==',
      right: 'John Doe',
    });
  });

  test('should return null for invalid syntax', () => {
    expect(parseStringCondition('invalid')).toBeNull();
    expect(parseStringCondition('')).toBeNull();
    expect(parseStringCondition('store.x')).toBeNull();
  });
});

describe('evaluateExpression', () => {
  const context: ResponseStoreContext = {
    status: '200',
    userId: '123',
    name: 'John',
    empty: '',
    enabled: 'true',
    count: '42',
    body: '{"type":"user","nested":{"id":1}}',
  };

  describe('exists operator', () => {
    test('should return true for existing value', () => {
      const expr: ConditionExpression = { left: 'store.userId', operator: 'exists' };
      expect(evaluateExpression(expr, context).passed).toBe(true);
    });

    test('should return false for empty string', () => {
      const expr: ConditionExpression = { left: 'store.empty', operator: 'exists' };
      expect(evaluateExpression(expr, context).passed).toBe(false);
    });

    test('should return false for missing value', () => {
      const expr: ConditionExpression = { left: 'store.missing', operator: 'exists' };
      expect(evaluateExpression(expr, context).passed).toBe(false);
    });
  });

  describe('not-exists operator', () => {
    test('should return true for missing value', () => {
      const expr: ConditionExpression = { left: 'store.missing', operator: 'not-exists' };
      expect(evaluateExpression(expr, context).passed).toBe(true);
    });

    test('should return true for empty string', () => {
      const expr: ConditionExpression = { left: 'store.empty', operator: 'not-exists' };
      expect(evaluateExpression(expr, context).passed).toBe(true);
    });

    test('should return false for existing value', () => {
      const expr: ConditionExpression = { left: 'store.userId', operator: 'not-exists' };
      expect(evaluateExpression(expr, context).passed).toBe(false);
    });
  });

  describe('== operator', () => {
    test('should compare numbers', () => {
      const expr: ConditionExpression = { left: 'store.status', operator: '==', right: 200 };
      expect(evaluateExpression(expr, context).passed).toBe(true);
    });

    test('should compare strings case-insensitively by default', () => {
      const expr: ConditionExpression = { left: 'store.name', operator: '==', right: 'john' };
      expect(evaluateExpression(expr, context).passed).toBe(true);
    });

    test('should compare strings case-sensitively when specified', () => {
      const expr: ConditionExpression = {
        left: 'store.name',
        operator: '==',
        right: 'john',
        caseSensitive: true,
      };
      expect(evaluateExpression(expr, context).passed).toBe(false);
    });

    test('should compare booleans', () => {
      const expr: ConditionExpression = { left: 'store.enabled', operator: '==', right: true };
      expect(evaluateExpression(expr, context).passed).toBe(true);
    });
  });

  describe('!= operator', () => {
    test('should return true for different values', () => {
      const expr: ConditionExpression = { left: 'store.status', operator: '!=', right: 404 };
      expect(evaluateExpression(expr, context).passed).toBe(true);
    });

    test('should return false for equal values', () => {
      const expr: ConditionExpression = { left: 'store.status', operator: '!=', right: 200 };
      expect(evaluateExpression(expr, context).passed).toBe(false);
    });
  });

  describe('comparison operators', () => {
    test('should compare with >', () => {
      const expr: ConditionExpression = { left: 'store.count', operator: '>', right: 40 };
      expect(evaluateExpression(expr, context).passed).toBe(true);
    });

    test('should compare with <', () => {
      const expr: ConditionExpression = { left: 'store.count', operator: '<', right: 50 };
      expect(evaluateExpression(expr, context).passed).toBe(true);
    });

    test('should compare with >=', () => {
      const expr: ConditionExpression = { left: 'store.count', operator: '>=', right: 42 };
      expect(evaluateExpression(expr, context).passed).toBe(true);
    });

    test('should compare with <=', () => {
      const expr: ConditionExpression = { left: 'store.count', operator: '<=', right: 42 };
      expect(evaluateExpression(expr, context).passed).toBe(true);
    });
  });

  describe('contains operator', () => {
    test('should find substring case-insensitively', () => {
      const expr: ConditionExpression = { left: 'store.name', operator: 'contains', right: 'OH' };
      expect(evaluateExpression(expr, context).passed).toBe(true);
    });

    test('should find substring case-sensitively when specified', () => {
      const expr: ConditionExpression = {
        left: 'store.name',
        operator: 'contains',
        right: 'OH',
        caseSensitive: true,
      };
      expect(evaluateExpression(expr, context).passed).toBe(false);
    });

    test('should return false when substring not found', () => {
      const expr: ConditionExpression = { left: 'store.name', operator: 'contains', right: 'xyz' };
      expect(evaluateExpression(expr, context).passed).toBe(false);
    });
  });

  describe('matches operator', () => {
    test('should match regex pattern', () => {
      const expr: ConditionExpression = {
        left: 'store.name',
        operator: 'matches',
        right: '^[A-Z]',
      };
      expect(evaluateExpression(expr, context).passed).toBe(true);
    });

    test('should be case-insensitive by default', () => {
      const expr: ConditionExpression = {
        left: 'store.name',
        operator: 'matches',
        right: '^john$',
      };
      expect(evaluateExpression(expr, context).passed).toBe(true);
    });

    test('should respect caseSensitive flag', () => {
      const expr: ConditionExpression = {
        left: 'store.name',
        operator: 'matches',
        right: '^john$',
        caseSensitive: true,
      };
      expect(evaluateExpression(expr, context).passed).toBe(false);
    });

    test('should handle invalid regex', () => {
      const expr: ConditionExpression = { left: 'store.name', operator: 'matches', right: '[' };
      expect(evaluateExpression(expr, context).passed).toBe(false);
    });
  });
});

describe('evaluateCondition', () => {
  const context: ResponseStoreContext = {
    status: '200',
    userId: '123',
    type: 'admin',
    enabled: 'true',
  };

  describe('string shorthand', () => {
    test('should evaluate valid string condition', () => {
      const result = evaluateCondition('store.status == 200', context);
      expect(result.shouldRun).toBe(true);
      expect(result.reason).toBeUndefined();
    });

    test('should return false with reason for failing condition', () => {
      const result = evaluateCondition('store.status == 404', context);
      expect(result.shouldRun).toBe(false);
      expect(result.reason).toContain('condition not met');
    });

    test('should return false for invalid syntax', () => {
      const result = evaluateCondition('invalid syntax', context);
      expect(result.shouldRun).toBe(false);
      expect(result.reason).toContain('invalid condition syntax');
    });
  });

  describe('single condition object', () => {
    test('should evaluate single condition', () => {
      const condition: WhenCondition = {
        left: 'store.userId',
        operator: 'exists',
      };
      expect(evaluateCondition(condition, context).shouldRun).toBe(true);
    });

    test('should fail single condition', () => {
      const condition: WhenCondition = {
        left: 'store.missing',
        operator: 'exists',
      };
      const result = evaluateCondition(condition, context);
      expect(result.shouldRun).toBe(false);
      expect(result.reason).toContain('condition not met');
    });
  });

  describe('all (AND) conditions', () => {
    test('should pass when all conditions true', () => {
      const condition: WhenCondition = {
        all: [
          { left: 'store.status', operator: '==', right: 200 },
          { left: 'store.userId', operator: 'exists' },
          { left: 'store.type', operator: '==', right: 'admin' },
        ],
      };
      expect(evaluateCondition(condition, context).shouldRun).toBe(true);
    });

    test('should fail when any condition false (short-circuit)', () => {
      const condition: WhenCondition = {
        all: [
          { left: 'store.status', operator: '==', right: 200 },
          { left: 'store.status', operator: '==', right: 404 }, // fails
          { left: 'store.userId', operator: 'exists' },
        ],
      };
      const result = evaluateCondition(condition, context);
      expect(result.shouldRun).toBe(false);
      expect(result.reason).toContain('store.status == 404');
    });
  });

  describe('any (OR) conditions', () => {
    test('should pass when any condition true (short-circuit)', () => {
      const condition: WhenCondition = {
        any: [
          { left: 'store.status', operator: '==', right: 404 }, // fails
          { left: 'store.status', operator: '==', right: 200 }, // passes
          { left: 'store.status', operator: '==', right: 500 }, // skipped
        ],
      };
      expect(evaluateCondition(condition, context).shouldRun).toBe(true);
    });

    test('should fail when all conditions false', () => {
      const condition: WhenCondition = {
        any: [
          { left: 'store.status', operator: '==', right: 404 },
          { left: 'store.status', operator: '==', right: 500 },
        ],
      };
      const result = evaluateCondition(condition, context);
      expect(result.shouldRun).toBe(false);
      expect(result.reason).toContain('no conditions met');
    });
  });

  describe('edge cases', () => {
    test('should run when no valid condition specified', () => {
      const condition: WhenCondition = {};
      expect(evaluateCondition(condition, context).shouldRun).toBe(true);
    });

    test('should handle empty all array', () => {
      const condition: WhenCondition = { all: [] };
      expect(evaluateCondition(condition, context).shouldRun).toBe(true);
    });

    test('should handle empty any array', () => {
      const condition: WhenCondition = { any: [] };
      expect(evaluateCondition(condition, context).shouldRun).toBe(true);
    });

    test('should handle nested JSON in store', () => {
      const nestedContext: ResponseStoreContext = {
        body: '{"user":{"id":123,"role":"admin"}}',
      };
      const result = evaluateCondition('store.body.user.role == admin', nestedContext);
      expect(result.shouldRun).toBe(true);
    });
  });
});
