import { describe, expect, test } from 'bun:test';
import type { Snapshot } from '../types/config';
import { SnapshotDiffer } from './snapshot-differ';

describe('SnapshotDiffer', () => {
  describe('basic comparison', () => {
    test('should match identical snapshots', () => {
      const differ = new SnapshotDiffer({});
      const snapshot: Snapshot = {
        status: 200,
        body: { id: 1, name: 'test' },
        hash: 'abc123',
        updatedAt: '2024-01-01',
      };

      const result = differ.compare(snapshot, snapshot);
      expect(result.match).toBe(true);
      expect(result.differences).toHaveLength(0);
    });

    test('should detect status changes', () => {
      const differ = new SnapshotDiffer({});
      const expected: Snapshot = { status: 200, hash: 'a', updatedAt: '' };
      const received: Snapshot = { status: 201, hash: 'b', updatedAt: '' };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(false);
      expect(result.differences).toHaveLength(1);
      expect(result.differences[0]).toEqual({
        path: 'status',
        expected: 200,
        received: 201,
        type: 'changed',
      });
    });

    test('should detect body value changes', () => {
      const differ = new SnapshotDiffer({});
      const expected: Snapshot = {
        body: { name: 'old' },
        hash: 'a',
        updatedAt: '',
      };
      const received: Snapshot = {
        body: { name: 'new' },
        hash: 'b',
        updatedAt: '',
      };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(false);
      expect(result.differences[0].path).toBe('body.name');
      expect(result.differences[0].type).toBe('changed');
    });

    test('should detect added fields', () => {
      const differ = new SnapshotDiffer({});
      const expected: Snapshot = {
        body: { id: 1 },
        hash: 'a',
        updatedAt: '',
      };
      const received: Snapshot = {
        body: { id: 1, newField: 'value' },
        hash: 'b',
        updatedAt: '',
      };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(false);
      expect(result.differences[0].path).toBe('body.newField');
      expect(result.differences[0].type).toBe('added');
    });

    test('should detect removed fields', () => {
      const differ = new SnapshotDiffer({});
      const expected: Snapshot = {
        body: { id: 1, oldField: 'value' },
        hash: 'a',
        updatedAt: '',
      };
      const received: Snapshot = {
        body: { id: 1 },
        hash: 'b',
        updatedAt: '',
      };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(false);
      expect(result.differences[0].path).toBe('body.oldField');
      expect(result.differences[0].type).toBe('removed');
    });
  });

  describe('array comparison', () => {
    test('should compare array elements', () => {
      const differ = new SnapshotDiffer({});
      const expected: Snapshot = {
        body: { items: [1, 2, 3] },
        hash: 'a',
        updatedAt: '',
      };
      const received: Snapshot = {
        body: { items: [1, 2, 4] },
        hash: 'b',
        updatedAt: '',
      };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(false);
      expect(result.differences[0].path).toBe('body.items[2]');
    });

    test('should detect added array elements', () => {
      const differ = new SnapshotDiffer({});
      const expected: Snapshot = {
        body: { items: [1, 2] },
        hash: 'a',
        updatedAt: '',
      };
      const received: Snapshot = {
        body: { items: [1, 2, 3] },
        hash: 'b',
        updatedAt: '',
      };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(false);
      expect(result.differences[0].path).toBe('body.items[2]');
      expect(result.differences[0].type).toBe('added');
    });
  });

  describe('exclusions', () => {
    test('should exclude exact paths', () => {
      const differ = new SnapshotDiffer({
        exclude: ['body.timestamp'],
      });
      const expected: Snapshot = {
        body: { id: 1, timestamp: '2024-01-01' },
        hash: 'a',
        updatedAt: '',
      };
      const received: Snapshot = {
        body: { id: 1, timestamp: '2024-12-31' },
        hash: 'b',
        updatedAt: '',
      };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(true);
    });

    test('should exclude wildcard paths (*.field)', () => {
      const differ = new SnapshotDiffer({
        exclude: ['*.createdAt'],
      });
      const expected: Snapshot = {
        body: { user: { createdAt: '2024-01-01' }, post: { createdAt: '2024-01-01' } },
        hash: 'a',
        updatedAt: '',
      };
      const received: Snapshot = {
        body: { user: { createdAt: '2024-12-31' }, post: { createdAt: '2024-12-31' } },
        hash: 'b',
        updatedAt: '',
      };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(true);
    });

    test('should exclude array wildcard paths (body[*].id)', () => {
      const differ = new SnapshotDiffer({
        exclude: ['body.items[*].id'],
      });
      const expected: Snapshot = {
        body: {
          items: [
            { id: 1, name: 'a' },
            { id: 2, name: 'b' },
          ],
        },
        hash: 'a',
        updatedAt: '',
      };
      const received: Snapshot = {
        body: {
          items: [
            { id: 99, name: 'a' },
            { id: 100, name: 'b' },
          ],
        },
        hash: 'b',
        updatedAt: '',
      };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(true);
    });
  });

  describe('match rules', () => {
    test('should accept any value with wildcard (*)', () => {
      const differ = new SnapshotDiffer({
        match: { 'body.id': '*' },
      });
      const expected: Snapshot = {
        body: { id: 1 },
        hash: 'a',
        updatedAt: '',
      };
      const received: Snapshot = {
        body: { id: 999 },
        hash: 'b',
        updatedAt: '',
      };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(true);
    });

    test('should match regex patterns', () => {
      const differ = new SnapshotDiffer({
        match: { 'body.version': 'regex:^v\\d+\\.\\d+' },
      });
      const expected: Snapshot = {
        body: { version: 'v1.0.0' },
        hash: 'a',
        updatedAt: '',
      };
      const received: Snapshot = {
        body: { version: 'v2.5.3' },
        hash: 'b',
        updatedAt: '',
      };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(true);
    });

    test('should fail on non-matching regex', () => {
      const differ = new SnapshotDiffer({
        match: { 'body.version': 'regex:^v\\d+\\.\\d+' },
      });
      const expected: Snapshot = {
        body: { version: 'v1.0' },
        hash: 'a',
        updatedAt: '',
      };
      const received: Snapshot = {
        body: { version: 'invalid' },
        hash: 'b',
        updatedAt: '',
      };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(false);
    });
  });

  describe('type mismatches', () => {
    test('should detect type changes', () => {
      const differ = new SnapshotDiffer({});
      const expected: Snapshot = {
        body: { count: 42 },
        hash: 'a',
        updatedAt: '',
      };
      const received: Snapshot = {
        body: { count: '42' },
        hash: 'b',
        updatedAt: '',
      };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(false);
      expect(result.differences[0].type).toBe('type_mismatch');
    });

    test('should detect object to array change', () => {
      const differ = new SnapshotDiffer({});
      const expected: Snapshot = {
        body: { data: { key: 'value' } },
        hash: 'a',
        updatedAt: '',
      };
      const received: Snapshot = {
        body: { data: ['value'] },
        hash: 'b',
        updatedAt: '',
      };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(false);
      expect(result.differences[0].type).toBe('type_mismatch');
    });
  });

  describe('nested objects', () => {
    test('should compare deeply nested values', () => {
      const differ = new SnapshotDiffer({});
      const expected: Snapshot = {
        body: { level1: { level2: { level3: { value: 'old' } } } },
        hash: 'a',
        updatedAt: '',
      };
      const received: Snapshot = {
        body: { level1: { level2: { level3: { value: 'new' } } } },
        hash: 'b',
        updatedAt: '',
      };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(false);
      expect(result.differences[0].path).toBe('body.level1.level2.level3.value');
    });
  });

  describe('header comparison', () => {
    test('should compare headers', () => {
      const differ = new SnapshotDiffer({});
      const expected: Snapshot = {
        headers: { 'content-type': 'application/json' },
        hash: 'a',
        updatedAt: '',
      };
      const received: Snapshot = {
        headers: { 'content-type': 'text/html' },
        hash: 'b',
        updatedAt: '',
      };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(false);
      expect(result.differences[0].path).toBe('headers.content-type');
    });

    test('should exclude headers', () => {
      const differ = new SnapshotDiffer({
        exclude: ['headers.date', 'headers.x-request-id'],
      });
      const expected: Snapshot = {
        headers: { 'content-type': 'application/json', date: '2024-01-01' },
        hash: 'a',
        updatedAt: '',
      };
      const received: Snapshot = {
        headers: { 'content-type': 'application/json', date: '2024-12-31' },
        hash: 'b',
        updatedAt: '',
      };

      const result = differ.compare(expected, received);
      expect(result.match).toBe(true);
    });
  });
});
