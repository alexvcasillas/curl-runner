import { describe, expect, test } from 'bun:test';
import type { ExecutionResult } from '../types/config';
import { filterSnapshotBody, SnapshotManager } from './snapshot-manager';

describe('SnapshotManager', () => {
  describe('getSnapshotPath', () => {
    test('should generate correct snapshot path', () => {
      const manager = new SnapshotManager({});
      expect(manager.getSnapshotPath('tests/api.yaml')).toBe('tests/__snapshots__/api.snap.json');
    });

    test('should use custom directory', () => {
      const manager = new SnapshotManager({ dir: '.snapshots' });
      expect(manager.getSnapshotPath('api.yaml')).toBe('.snapshots/api.snap.json');
    });

    test('should handle nested paths', () => {
      const manager = new SnapshotManager({});
      expect(manager.getSnapshotPath('tests/integration/users.yaml')).toBe(
        'tests/integration/__snapshots__/users.snap.json',
      );
    });
  });

  describe('createSnapshot', () => {
    const mockResult: ExecutionResult = {
      request: { url: 'https://api.example.com' },
      success: true,
      status: 200,
      headers: {
        'Content-Type': 'application/json',
        'X-Request-Id': 'abc123',
      },
      body: {
        id: 1,
        name: 'Test',
        timestamp: '2024-01-01',
      },
    };

    test('should create snapshot with body only by default', () => {
      const manager = new SnapshotManager({});
      const snapshot = manager.createSnapshot(mockResult, { include: ['body'] });

      expect(snapshot.body).toEqual(mockResult.body);
      expect(snapshot.status).toBeUndefined();
      expect(snapshot.headers).toBeUndefined();
      expect(snapshot.hash).toBeDefined();
      expect(snapshot.updatedAt).toBeDefined();
    });

    test('should create snapshot with status', () => {
      const manager = new SnapshotManager({});
      const snapshot = manager.createSnapshot(mockResult, { include: ['status'] });

      expect(snapshot.status).toBe(200);
      expect(snapshot.body).toBeUndefined();
    });

    test('should create snapshot with headers (normalized)', () => {
      const manager = new SnapshotManager({});
      const snapshot = manager.createSnapshot(mockResult, { include: ['headers'] });

      expect(snapshot.headers).toEqual({
        'content-type': 'application/json',
        'x-request-id': 'abc123',
      });
    });

    test('should create snapshot with all components', () => {
      const manager = new SnapshotManager({});
      const snapshot = manager.createSnapshot(mockResult, {
        include: ['status', 'headers', 'body'],
      });

      expect(snapshot.status).toBe(200);
      expect(snapshot.headers).toBeDefined();
      expect(snapshot.body).toBeDefined();
    });
  });

  describe('hash', () => {
    test('should generate consistent hashes', () => {
      const manager = new SnapshotManager({});
      const content = { id: 1, name: 'test' };

      const hash1 = manager.hash(content);
      const hash2 = manager.hash(content);

      expect(hash1).toBe(hash2);
    });

    test('should generate different hashes for different content', () => {
      const manager = new SnapshotManager({});

      const hash1 = manager.hash({ id: 1 });
      const hash2 = manager.hash({ id: 2 });

      expect(hash1).not.toBe(hash2);
    });
  });

  describe('mergeConfig', () => {
    test('should return null if not enabled', () => {
      const config = SnapshotManager.mergeConfig({}, undefined);
      expect(config).toBeNull();
    });

    test('should handle boolean true', () => {
      const config = SnapshotManager.mergeConfig({}, true);
      expect(config).toEqual({
        enabled: true,
        include: ['body'],
        exclude: [],
        match: {},
      });
    });

    test('should merge global and request excludes', () => {
      const config = SnapshotManager.mergeConfig(
        { exclude: ['*.timestamp'] },
        { enabled: true, exclude: ['body.id'] },
      );
      expect(config?.exclude).toEqual(['*.timestamp', 'body.id']);
    });

    test('should override include from request config', () => {
      const config = SnapshotManager.mergeConfig(
        { include: ['body'] },
        { enabled: true, include: ['status', 'body'] },
      );
      expect(config?.include).toEqual(['status', 'body']);
    });

    test('should use global enabled', () => {
      const config = SnapshotManager.mergeConfig({ enabled: true }, undefined);
      expect(config?.enabled).toBe(true);
    });
  });
});

describe('filterSnapshotBody', () => {
  test('should return primitive values unchanged', () => {
    expect(filterSnapshotBody('string', [])).toBe('string');
    expect(filterSnapshotBody(123, [])).toBe(123);
    expect(filterSnapshotBody(null, [])).toBe(null);
  });

  test('should return body unchanged if no excludes', () => {
    const body = { id: 1, name: 'test' };
    expect(filterSnapshotBody(body, [])).toEqual(body);
  });

  test('should filter exact paths', () => {
    const body = { id: 1, timestamp: '2024-01-01', name: 'test' };
    const result = filterSnapshotBody(body, ['body.timestamp']);
    expect(result).toEqual({ id: 1, name: 'test' });
  });

  test('should filter wildcard paths', () => {
    const body = {
      user: { id: 1, createdAt: '2024-01-01' },
      post: { id: 2, createdAt: '2024-01-02' },
    };
    const result = filterSnapshotBody(body, ['body.*.createdAt']);
    expect(result).toEqual({
      user: { id: 1 },
      post: { id: 2 },
    });
  });

  test('should filter array wildcards', () => {
    const body = {
      items: [
        { id: 1, updatedAt: '2024-01-01' },
        { id: 2, updatedAt: '2024-01-02' },
      ],
    };
    const result = filterSnapshotBody(body, ['body.items[*].updatedAt']);
    expect(result).toEqual({
      items: [{ id: 1 }, { id: 2 }],
    });
  });

  test('should handle nested objects', () => {
    const body = {
      data: {
        user: {
          id: 1,
          meta: { lastLogin: '2024-01-01' },
        },
      },
    };
    const result = filterSnapshotBody(body, ['body.data.user.meta.lastLogin']);
    expect(result).toEqual({
      data: {
        user: {
          id: 1,
          meta: {},
        },
      },
    });
  });
});
