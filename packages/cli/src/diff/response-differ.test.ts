import { describe, expect, test } from 'bun:test';
import type { Baseline } from '../types/config';
import { ResponseDiffer } from './response-differ';

describe('ResponseDiffer', () => {
  describe('compare', () => {
    test('should detect no differences for identical baselines', () => {
      const differ = new ResponseDiffer({});
      const baseline: Baseline = {
        status: 200,
        body: { id: 1, name: 'Test' },
        hash: 'abc123',
        capturedAt: '2024-01-01',
      };

      const result = differ.compare(baseline, baseline, 'staging', 'production', 'Get User');

      expect(result.hasDifferences).toBe(false);
      expect(result.differences).toHaveLength(0);
      expect(result.requestName).toBe('Get User');
      expect(result.baselineLabel).toBe('staging');
      expect(result.currentLabel).toBe('production');
    });

    test('should detect status change', () => {
      const differ = new ResponseDiffer({});
      const baseline: Baseline = { status: 200, hash: 'a', capturedAt: '' };
      const current: Baseline = { status: 201, hash: 'b', capturedAt: '' };

      const result = differ.compare(baseline, current, 'v1', 'v2', 'Create User');

      expect(result.hasDifferences).toBe(true);
      expect(result.differences).toHaveLength(1);
      expect(result.differences[0]).toEqual({
        path: 'status',
        baseline: 200,
        current: 201,
        type: 'changed',
      });
    });

    test('should detect body field changes', () => {
      const differ = new ResponseDiffer({});
      const baseline: Baseline = {
        body: { id: 1, name: 'Old' },
        hash: 'a',
        capturedAt: '',
      };
      const current: Baseline = {
        body: { id: 1, name: 'New' },
        hash: 'b',
        capturedAt: '',
      };

      const result = differ.compare(baseline, current, 'v1', 'v2', 'Get');

      expect(result.hasDifferences).toBe(true);
      expect(result.differences).toContainEqual({
        path: 'body.name',
        baseline: 'Old',
        current: 'New',
        type: 'changed',
      });
    });

    test('should detect added fields', () => {
      const differ = new ResponseDiffer({});
      const baseline: Baseline = {
        body: { id: 1 },
        hash: 'a',
        capturedAt: '',
      };
      const current: Baseline = {
        body: { id: 1, newField: 'value' },
        hash: 'b',
        capturedAt: '',
      };

      const result = differ.compare(baseline, current, 'v1', 'v2', 'Get');

      expect(result.hasDifferences).toBe(true);
      expect(result.differences).toContainEqual({
        path: 'body.newField',
        baseline: undefined,
        current: 'value',
        type: 'added',
      });
    });

    test('should detect removed fields', () => {
      const differ = new ResponseDiffer({});
      const baseline: Baseline = {
        body: { id: 1, oldField: 'value' },
        hash: 'a',
        capturedAt: '',
      };
      const current: Baseline = {
        body: { id: 1 },
        hash: 'b',
        capturedAt: '',
      };

      const result = differ.compare(baseline, current, 'v1', 'v2', 'Get');

      expect(result.hasDifferences).toBe(true);
      expect(result.differences).toContainEqual({
        path: 'body.oldField',
        baseline: 'value',
        current: undefined,
        type: 'removed',
      });
    });

    test('should detect type mismatch', () => {
      const differ = new ResponseDiffer({});
      const baseline: Baseline = {
        body: { count: '10' },
        hash: 'a',
        capturedAt: '',
      };
      const current: Baseline = {
        body: { count: 10 },
        hash: 'b',
        capturedAt: '',
      };

      const result = differ.compare(baseline, current, 'v1', 'v2', 'Get');

      expect(result.hasDifferences).toBe(true);
      expect(result.differences[0].type).toBe('type_mismatch');
    });

    test('should include timing diff when configured', () => {
      const differ = new ResponseDiffer({ includeTimings: true });
      const baseline: Baseline = {
        body: {},
        timing: 100,
        hash: 'a',
        capturedAt: '',
      };
      const current: Baseline = {
        body: {},
        timing: 150,
        hash: 'b',
        capturedAt: '',
      };

      const result = differ.compare(baseline, current, 'v1', 'v2', 'Get');

      expect(result.timingDiff).toBeDefined();
      expect(result.timingDiff?.baseline).toBe(100);
      expect(result.timingDiff?.current).toBe(150);
      expect(result.timingDiff?.changePercent).toBe(50);
    });
  });

  describe('deepCompare', () => {
    test('should handle nested objects', () => {
      const differ = new ResponseDiffer({});
      const baseline = { user: { profile: { age: 25 } } };
      const current = { user: { profile: { age: 26 } } };

      const diffs = differ.deepCompare(baseline, current, 'body');

      expect(diffs).toHaveLength(1);
      expect(diffs[0].path).toBe('body.user.profile.age');
    });

    test('should handle arrays', () => {
      const differ = new ResponseDiffer({});
      const baseline = { items: [1, 2, 3] };
      const current = { items: [1, 2, 4] };

      const diffs = differ.deepCompare(baseline, current, 'body');

      expect(diffs).toHaveLength(1);
      expect(diffs[0].path).toBe('body.items[2]');
    });

    test('should detect array length changes', () => {
      const differ = new ResponseDiffer({});
      const baseline = { items: [1, 2] };
      const current = { items: [1, 2, 3] };

      const diffs = differ.deepCompare(baseline, current, 'body');

      expect(diffs).toHaveLength(1);
      expect(diffs[0].type).toBe('added');
      expect(diffs[0].path).toBe('body.items[2]');
    });

    test('should handle null values', () => {
      const differ = new ResponseDiffer({});
      const baseline = { value: null };
      const current = { value: null };

      const diffs = differ.deepCompare(baseline, current, 'body');

      expect(diffs).toHaveLength(0);
    });
  });

  describe('isExcluded', () => {
    test('should exclude exact paths', () => {
      const differ = new ResponseDiffer({ exclude: ['body.timestamp'] });

      expect(differ.isExcluded('body.timestamp')).toBe(true);
      expect(differ.isExcluded('body.id')).toBe(false);
    });

    test('should exclude wildcard paths', () => {
      const differ = new ResponseDiffer({ exclude: ['*.createdAt'] });

      expect(differ.isExcluded('body.createdAt')).toBe(true);
      expect(differ.isExcluded('body.user.createdAt')).toBe(true);
      expect(differ.isExcluded('body.id')).toBe(false);
    });

    test('should exclude array wildcards', () => {
      const differ = new ResponseDiffer({ exclude: ['body.items[*].id'] });

      expect(differ.isExcluded('body.items[0].id')).toBe(true);
      expect(differ.isExcluded('body.items[99].id')).toBe(true);
      expect(differ.isExcluded('body.items[0].name')).toBe(false);
    });
  });

  describe('matchesRule', () => {
    test('should match wildcard rule', () => {
      const differ = new ResponseDiffer({ match: { 'body.requestId': '*' } });

      expect(differ.matchesRule('body.requestId', 'any-value')).toBe(true);
      expect(differ.matchesRule('body.requestId', 12345)).toBe(true);
      expect(differ.matchesRule('body.otherId', 'value')).toBe(false);
    });

    test('should match regex rule', () => {
      const differ = new ResponseDiffer({
        match: { 'body.uuid': 'regex:^[a-f0-9-]{36}$' },
      });

      expect(differ.matchesRule('body.uuid', '550e8400-e29b-41d4-a716-446655440000')).toBe(true);
      expect(differ.matchesRule('body.uuid', 'invalid')).toBe(false);
    });

    test('should return false for unmatched paths', () => {
      const differ = new ResponseDiffer({ match: { 'body.id': '*' } });

      expect(differ.matchesRule('body.name', 'value')).toBe(false);
    });
  });

  describe('compare with exclusions', () => {
    test('should ignore excluded paths in comparison', () => {
      const differ = new ResponseDiffer({ exclude: ['body.timestamp', 'body.requestId'] });
      const baseline: Baseline = {
        body: { id: 1, timestamp: '2024-01-01', requestId: 'abc' },
        hash: 'a',
        capturedAt: '',
      };
      const current: Baseline = {
        body: { id: 1, timestamp: '2024-01-02', requestId: 'xyz' },
        hash: 'b',
        capturedAt: '',
      };

      const result = differ.compare(baseline, current, 'v1', 'v2', 'Get');

      expect(result.hasDifferences).toBe(false);
    });

    test('should ignore matched paths with wildcard', () => {
      const differ = new ResponseDiffer({ match: { 'body.token': '*' } });
      const baseline: Baseline = {
        body: { id: 1, token: 'old-token' },
        hash: 'a',
        capturedAt: '',
      };
      const current: Baseline = {
        body: { id: 1, token: 'new-token' },
        hash: 'b',
        capturedAt: '',
      };

      const result = differ.compare(baseline, current, 'v1', 'v2', 'Get');

      expect(result.hasDifferences).toBe(false);
    });
  });

  describe('header comparison', () => {
    test('should detect header changes', () => {
      const differ = new ResponseDiffer({});
      const baseline: Baseline = {
        headers: { 'content-type': 'application/json' },
        hash: 'a',
        capturedAt: '',
      };
      const current: Baseline = {
        headers: { 'content-type': 'text/plain' },
        hash: 'b',
        capturedAt: '',
      };

      const result = differ.compare(baseline, current, 'v1', 'v2', 'Get');

      expect(result.hasDifferences).toBe(true);
      expect(result.differences[0].path).toBe('headers.content-type');
    });

    test('should detect added headers', () => {
      const differ = new ResponseDiffer({});
      const baseline: Baseline = {
        headers: {},
        hash: 'a',
        capturedAt: '',
      };
      const current: Baseline = {
        headers: { 'x-new-header': 'value' },
        hash: 'b',
        capturedAt: '',
      };

      const result = differ.compare(baseline, current, 'v1', 'v2', 'Get');

      expect(result.hasDifferences).toBe(true);
      expect(result.differences[0].type).toBe('added');
    });
  });
});
