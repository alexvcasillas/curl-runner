import { describe, expect, mock, test } from 'bun:test';
import type { ExecutionResult, RequestConfig, SnapshotConfig } from '../../types/config';
import { postProcessResult } from './post-processor';

describe('postProcessResult', () => {
  const createResult = (overrides: Partial<ExecutionResult> = {}): ExecutionResult => ({
    request: { url: 'https://example.com' },
    success: true,
    status: 200,
    body: { data: 'test' },
    headers: { 'content-type': 'application/json' },
    metrics: { duration: 100 },
    ...overrides,
  });

  const createConfig = (overrides: Partial<RequestConfig> = {}): RequestConfig => ({
    url: 'https://example.com',
    ...overrides,
  });

  describe('validation', () => {
    test('passes when no expect config', async () => {
      const result = createResult();
      const config = createConfig();

      await postProcessResult(result, config);

      expect(result.success).toBe(true);
      expect(result.error).toBeUndefined();
    });

    test('passes when status matches expect', async () => {
      const result = createResult({ status: 200 });
      const config = createConfig({ expect: { status: 200 } });

      await postProcessResult(result, config);

      expect(result.success).toBe(true);
    });

    test('fails when status does not match expect', async () => {
      const result = createResult({ status: 404 });
      const config = createConfig({ expect: { status: 200 } });

      await postProcessResult(result, config);

      expect(result.success).toBe(false);
      expect(result.error).toContain('status');
    });

    test('skips validation when result already failed', async () => {
      const result = createResult({ success: false, error: 'Connection failed' });
      const config = createConfig({ expect: { status: 200 } });

      await postProcessResult(result, config);

      // Error should remain unchanged
      expect(result.error).toBe('Connection failed');
    });
  });

  describe('snapshot testing', () => {
    test('skips snapshot when no manager provided', async () => {
      const result = createResult();
      const config = createConfig({ sourceFile: 'test.yaml', name: 'Test' });

      await postProcessResult(result, config, {
        snapshotConfig: { enabled: true },
      });

      expect(result.snapshotResult).toBeUndefined();
    });

    test('skips snapshot when no config provided', async () => {
      const mockManager = {
        compareAndUpdate: mock(() =>
          Promise.resolve({ match: true, isNew: false, updated: false, differences: [] }),
        ),
      };
      const result = createResult();
      const config = createConfig({ sourceFile: 'test.yaml' });

      await postProcessResult(result, config, {
        snapshotManager: mockManager as any,
        snapshotConfig: null,
      });

      expect(mockManager.compareAndUpdate).not.toHaveBeenCalled();
    });

    test('skips snapshot when no sourceFile', async () => {
      const mockManager = {
        compareAndUpdate: mock(() =>
          Promise.resolve({ match: true, isNew: false, updated: false, differences: [] }),
        ),
      };
      const result = createResult();
      const config = createConfig({ name: 'Test' }); // no sourceFile

      await postProcessResult(result, config, {
        snapshotManager: mockManager as any,
        snapshotConfig: { enabled: true },
      });

      expect(mockManager.compareAndUpdate).not.toHaveBeenCalled();
    });

    test('calls snapshot manager when configured', async () => {
      const mockManager = {
        compareAndUpdate: mock(() =>
          Promise.resolve({ match: true, isNew: false, updated: false, differences: [] }),
        ),
      };
      const result = createResult();
      const config = createConfig({ sourceFile: 'test.yaml', name: 'Test Request' });
      const snapshotConfig: SnapshotConfig = { enabled: true };

      await postProcessResult(result, config, {
        snapshotManager: mockManager as any,
        snapshotConfig,
      });

      expect(mockManager.compareAndUpdate).toHaveBeenCalledWith(
        'test.yaml',
        'Test Request',
        result,
        snapshotConfig,
      );
    });

    test('uses "Request" as default name', async () => {
      const mockManager = {
        compareAndUpdate: mock(() =>
          Promise.resolve({ match: true, isNew: false, updated: false, differences: [] }),
        ),
      };
      const result = createResult();
      const config = createConfig({ sourceFile: 'test.yaml' }); // no name
      const snapshotConfig: SnapshotConfig = { enabled: true };

      await postProcessResult(result, config, {
        snapshotManager: mockManager as any,
        snapshotConfig,
      });

      expect(mockManager.compareAndUpdate).toHaveBeenCalledWith(
        'test.yaml',
        'Request',
        result,
        snapshotConfig,
      );
    });

    test('marks result as failed on snapshot mismatch', async () => {
      const mockManager = {
        compareAndUpdate: mock(() =>
          Promise.resolve({
            match: false,
            isNew: false,
            updated: false,
            differences: [{ path: 'body.data', expected: 'old', received: 'new', type: 'changed' }],
          }),
        ),
      };
      const result = createResult();
      const config = createConfig({ sourceFile: 'test.yaml', name: 'Test' });

      await postProcessResult(result, config, {
        snapshotManager: mockManager as any,
        snapshotConfig: { enabled: true },
      });

      expect(result.success).toBe(false);
      expect(result.error).toBe('Snapshot mismatch');
      expect(result.snapshotResult).toBeDefined();
      expect(result.snapshotResult?.match).toBe(false);
    });

    test('does not fail when snapshot was updated', async () => {
      const mockManager = {
        compareAndUpdate: mock(() =>
          Promise.resolve({
            match: false,
            isNew: false,
            updated: true, // was updated
            differences: [],
          }),
        ),
      };
      const result = createResult();
      const config = createConfig({ sourceFile: 'test.yaml', name: 'Test' });

      await postProcessResult(result, config, {
        snapshotManager: mockManager as any,
        snapshotConfig: { enabled: true },
      });

      expect(result.success).toBe(true);
      expect(result.error).toBeUndefined();
    });

    test('skips snapshot when result already failed', async () => {
      const mockManager = {
        compareAndUpdate: mock(() =>
          Promise.resolve({ match: true, isNew: false, updated: false, differences: [] }),
        ),
      };
      const result = createResult({ success: false, error: 'Validation failed' });
      const config = createConfig({ sourceFile: 'test.yaml', name: 'Test' });

      await postProcessResult(result, config, {
        snapshotManager: mockManager as any,
        snapshotConfig: { enabled: true },
      });

      expect(mockManager.compareAndUpdate).not.toHaveBeenCalled();
    });

    test('preserves existing error when snapshot fails', async () => {
      const mockManager = {
        compareAndUpdate: mock(() =>
          Promise.resolve({
            match: false,
            isNew: false,
            updated: false,
            differences: [],
          }),
        ),
      };
      // First fail validation, then check snapshot doesn't overwrite error
      const result = createResult({ status: 404 });
      const config = createConfig({
        sourceFile: 'test.yaml',
        name: 'Test',
        expect: { status: 200 },
      });

      await postProcessResult(result, config, {
        snapshotManager: mockManager as any,
        snapshotConfig: { enabled: true },
      });

      // Should have validation error, not snapshot error
      expect(result.error).toContain('status');
    });
  });

  describe('returns result for chaining', () => {
    test('returns the mutated result', async () => {
      const result = createResult();
      const config = createConfig();

      const returned = await postProcessResult(result, config);

      expect(returned).toBe(result);
    });
  });
});
