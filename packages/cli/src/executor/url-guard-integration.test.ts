import { afterEach, beforeEach, describe, expect, spyOn, test } from 'bun:test';
import { CurlBuilder } from '../utils/curl-builder';
import { RequestExecutor } from './request-executor';
import type { GlobalConfig, RequestConfig } from '../types/config';

/**
 * Integration tests: blocked-protocol requests yield success:false without
 * invoking curl. Covers sequential, parallel, and pooled paths.
 */

describe('URL protocol enforcement in RequestExecutor', () => {
  let executeCurlSpy: ReturnType<typeof spyOn>;
  let consoleLogSpy: ReturnType<typeof spyOn>;
  let consoleErrorSpy: ReturnType<typeof spyOn>;

  beforeEach(() => {
    executeCurlSpy = spyOn(CurlBuilder, 'executeCurl');
    consoleLogSpy = spyOn(console, 'log').mockImplementation(() => {});
    consoleErrorSpy = spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    executeCurlSpy.mockRestore();
    consoleLogSpy.mockRestore();
    consoleErrorSpy.mockRestore();
  });

  describe('executeRequest', () => {
    test('file:// URL is blocked by default', async () => {
      const executor = new RequestExecutor({});
      const result = await executor.executeRequest({
        url: 'file:///etc/passwd',
        method: 'GET',
      });
      expect(result.success).toBe(false);
      expect(result.error).toContain('Blocked protocol "file"');
      expect(executeCurlSpy).not.toHaveBeenCalled();
    });

    test('gopher:// URL is blocked by default', async () => {
      const executor = new RequestExecutor({});
      const result = await executor.executeRequest({
        url: 'gopher://internal.example.com/',
        method: 'GET',
      });
      expect(result.success).toBe(false);
      expect(result.error).toContain('Blocked protocol "gopher"');
      expect(executeCurlSpy).not.toHaveBeenCalled();
    });

    test('ftp:// URL is blocked by default', async () => {
      const executor = new RequestExecutor({});
      const result = await executor.executeRequest({
        url: 'ftp://files.example.com/data.csv',
        method: 'GET',
      });
      expect(result.success).toBe(false);
      expect(result.error).toContain('Blocked protocol "ftp"');
      expect(executeCurlSpy).not.toHaveBeenCalled();
    });

    test('http:// URL passes by default', async () => {
      const mockResult = {
        success: true,
        status: 200,
        headers: {},
        body: '{}',
        metrics: { duration: 10 },
      };
      executeCurlSpy.mockResolvedValue(mockResult);

      const executor = new RequestExecutor({});
      const result = await executor.executeRequest({
        url: 'http://api.example.com/data',
        method: 'GET',
      });
      expect(result.success).toBe(true);
      expect(executeCurlSpy).toHaveBeenCalled();
    });

    test('ftp:// passes when explicitly allowed via security config', async () => {
      const mockResult = {
        success: true,
        status: 200,
        headers: {},
        body: 'data',
        metrics: { duration: 10 },
      };
      executeCurlSpy.mockResolvedValue(mockResult);

      const config: GlobalConfig = {
        security: { allowedProtocols: ['http', 'https', 'ftp'] },
      };
      const executor = new RequestExecutor(config);
      const result = await executor.executeRequest({
        url: 'ftp://files.example.com/data.csv',
        method: 'GET',
      });
      expect(result.success).toBe(true);
      expect(executeCurlSpy).toHaveBeenCalled();
    });

    test('blocked URL has metrics', async () => {
      const executor = new RequestExecutor({});
      const result = await executor.executeRequest({
        url: 'file:///etc/shadow',
        method: 'GET',
      });
      expect(result.success).toBe(false);
      expect(result.metrics).toBeDefined();
      expect(result.metrics?.duration).toBeGreaterThanOrEqual(0);
    });

    test('malformed URL is blocked', async () => {
      const executor = new RequestExecutor({});
      const result = await executor.executeRequest({
        url: 'not-a-url',
        method: 'GET',
      });
      expect(result.success).toBe(false);
      expect(result.error).toContain('Invalid URL');
      expect(executeCurlSpy).not.toHaveBeenCalled();
    });
  });

  describe('executeSequential', () => {
    test('blocked request in sequential run yields success:false', async () => {
      const executor = new RequestExecutor({});
      const requests: RequestConfig[] = [
        { url: 'file:///etc/passwd', method: 'GET' },
      ];
      const summary = await executor.executeSequential(requests);
      expect(summary.failed).toBe(1);
      expect(summary.successful).toBe(0);
      expect(summary.results[0].success).toBe(false);
      expect(executeCurlSpy).not.toHaveBeenCalled();
    });

    test('mixed requests: valid passes, blocked fails', async () => {
      const mockResult = {
        success: true,
        status: 200,
        headers: {},
        body: '{}',
        metrics: { duration: 10 },
      };
      executeCurlSpy.mockResolvedValue(mockResult);

      const config: GlobalConfig = { continueOnError: true };
      const executor = new RequestExecutor(config);
      const requests: RequestConfig[] = [
        { url: 'https://api.example.com/ok', method: 'GET' },
        { url: 'file:///etc/passwd', method: 'GET' },
      ];
      const summary = await executor.executeSequential(requests);
      expect(summary.total).toBe(2);
      expect(summary.successful).toBe(1);
      expect(summary.failed).toBe(1);
    });
  });

  describe('executeParallel (non-pooled)', () => {
    test('blocked request in parallel run yields success:false', async () => {
      const executor = new RequestExecutor({});
      const requests: RequestConfig[] = [
        { url: 'gopher://evil.example.com/', method: 'GET' },
      ];
      const summary = await executor.executeParallel(requests);
      expect(summary.failed).toBe(1);
      expect(summary.results[0].success).toBe(false);
      expect(executeCurlSpy).not.toHaveBeenCalled();
    });
  });
});
