import { describe, expect, test } from 'bun:test';
import type { ExecutionResult } from '../types/config';
import { BaselineManager } from './baseline-manager';

describe('BaselineManager', () => {
  describe('getBaselinePath', () => {
    test('should generate correct baseline path', () => {
      const manager = new BaselineManager({});
      expect(manager.getBaselinePath('tests/api.yaml', 'staging')).toBe(
        'tests/__baselines__/api.staging.baseline.json',
      );
    });

    test('should use custom directory', () => {
      const manager = new BaselineManager({ dir: '.baselines' });
      expect(manager.getBaselinePath('api.yaml', 'prod')).toBe('.baselines/api.prod.baseline.json');
    });

    test('should handle nested paths', () => {
      const manager = new BaselineManager({});
      expect(manager.getBaselinePath('tests/integration/users.yaml', 'staging')).toBe(
        'tests/integration/__baselines__/users.staging.baseline.json',
      );
    });

    test('should handle labels with special characters', () => {
      const manager = new BaselineManager({});
      expect(manager.getBaselinePath('api.yaml', 'v1.0.0')).toBe(
        '__baselines__/api.v1.0.0.baseline.json',
      );
    });
  });

  describe('getBaselineDir', () => {
    test('should return correct directory', () => {
      const manager = new BaselineManager({});
      expect(manager.getBaselineDir('tests/api.yaml')).toBe('tests/__baselines__');
    });

    test('should use custom directory', () => {
      const manager = new BaselineManager({ dir: 'snapshots' });
      expect(manager.getBaselineDir('api.yaml')).toBe('snapshots');
    });
  });

  describe('createBaseline', () => {
    const mockResult: ExecutionResult = {
      request: { url: 'https://api.example.com', name: 'Get Users' },
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
      metrics: {
        duration: 150,
      },
    };

    test('should create baseline with all fields', () => {
      const manager = new BaselineManager({});
      const baseline = manager.createBaseline(mockResult, {});

      expect(baseline.status).toBe(200);
      expect(baseline.body).toEqual(mockResult.body);
      expect(baseline.headers).toBeDefined();
      expect(baseline.hash).toBeDefined();
      expect(baseline.capturedAt).toBeDefined();
      expect(baseline.timing).toBeUndefined();
    });

    test('should include timing when configured', () => {
      const manager = new BaselineManager({});
      const baseline = manager.createBaseline(mockResult, { includeTimings: true });

      expect(baseline.timing).toBe(150);
    });

    test('should normalize headers (lowercase, sorted)', () => {
      const manager = new BaselineManager({});
      const baseline = manager.createBaseline(mockResult, {});

      expect(baseline.headers).toEqual({
        'content-type': 'application/json',
        'x-request-id': 'abc123',
      });
    });
  });

  describe('hash', () => {
    test('should generate consistent hashes', () => {
      const manager = new BaselineManager({});
      const content = { id: 1, name: 'test' };

      const hash1 = manager.hash(content);
      const hash2 = manager.hash(content);

      expect(hash1).toBe(hash2);
    });

    test('should generate different hashes for different content', () => {
      const manager = new BaselineManager({});

      const hash1 = manager.hash({ id: 1 });
      const hash2 = manager.hash({ id: 2 });

      expect(hash1).not.toBe(hash2);
    });

    test('should produce 8 character hash', () => {
      const manager = new BaselineManager({});
      const hash = manager.hash({ data: 'test' });

      expect(hash).toHaveLength(8);
    });
  });

  describe('mergeConfig', () => {
    test('should return null if not enabled', () => {
      const config = BaselineManager.mergeConfig({}, undefined);
      expect(config).toBeNull();
    });

    test('should return null if explicitly disabled', () => {
      const config = BaselineManager.mergeConfig({}, { enabled: false });
      expect(config).toBeNull();
    });

    test('should handle boolean true', () => {
      const config = BaselineManager.mergeConfig({}, true);
      expect(config).toEqual({
        enabled: true,
        exclude: [],
        match: {},
        includeTimings: false,
      });
    });

    test('should merge global and request excludes', () => {
      const config = BaselineManager.mergeConfig(
        { exclude: ['*.timestamp'] },
        { enabled: true, exclude: ['body.id'] },
      );
      expect(config?.exclude).toEqual(['*.timestamp', 'body.id']);
    });

    test('should merge match rules', () => {
      const config = BaselineManager.mergeConfig(
        { match: { 'body.id': '*' } },
        { enabled: true, match: { 'body.token': 'regex:^[a-z]+$' } },
      );
      expect(config?.match).toEqual({
        'body.id': '*',
        'body.token': 'regex:^[a-z]+$',
      });
    });

    test('should use global enabled', () => {
      const config = BaselineManager.mergeConfig({ enabled: true }, undefined);
      expect(config?.enabled).toBe(true);
    });

    test('should inherit includeTimings from global', () => {
      const config = BaselineManager.mergeConfig({ enabled: true, includeTimings: true }, true);
      expect(config?.includeTimings).toBe(true);
    });

    test('should allow request to override includeTimings', () => {
      const config = BaselineManager.mergeConfig(
        { enabled: true, includeTimings: false },
        { enabled: true, includeTimings: true },
      );
      expect(config?.includeTimings).toBe(true);
    });
  });
});
