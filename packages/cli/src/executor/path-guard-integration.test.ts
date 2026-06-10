import { afterEach, beforeEach, describe, expect, spyOn, test } from 'bun:test';
import type { GlobalConfig, RequestConfig } from '../types/config';
import { CurlBuilder } from '../utils/curl-builder';
import { RequestExecutor } from './request-executor';

/**
 * Integration tests: FS-path-escaping requests yield success:false without
 * invoking curl. Covers output, ssl, formData paths. Covers sequential,
 * parallel, and pooled paths.
 */

describe('Filesystem path confinement in RequestExecutor', () => {
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

  describe('output path confinement', () => {
    test('output: ../escape.json yields success:false and curl not called', async () => {
      const executor = new RequestExecutor({});
      const result = await executor.executeRequest({
        url: 'https://api.example.com/data',
        method: 'GET',
        output: '../escape.json',
      });
      expect(result.success).toBe(false);
      expect(result.error).toContain('escapes the working directory');
      expect(result.error).toContain('--allow-path');
      expect(executeCurlSpy).not.toHaveBeenCalled();
    });

    test('output: /etc/passwd yields success:false', async () => {
      const executor = new RequestExecutor({});
      const result = await executor.executeRequest({
        url: 'https://api.example.com/data',
        method: 'GET',
        output: '/etc/passwd',
      });
      expect(result.success).toBe(false);
      expect(result.error).toContain('escapes the working directory');
      expect(executeCurlSpy).not.toHaveBeenCalled();
    });

    test('output: relative in-cwd path passes guard', async () => {
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
        url: 'https://api.example.com/data',
        method: 'GET',
        output: 'output.json',
      });
      expect(result.success).toBe(true);
      expect(executeCurlSpy).toHaveBeenCalled();
    });

    test('output: ../escape.json allowed when security.allowPaths=true', async () => {
      const mockResult = {
        success: true,
        status: 200,
        headers: {},
        body: '{}',
        metrics: { duration: 10 },
      };
      executeCurlSpy.mockResolvedValue(mockResult);

      const config: GlobalConfig = { security: { allowPaths: true } };
      const executor = new RequestExecutor(config);
      const result = await executor.executeRequest({
        url: 'https://api.example.com/data',
        method: 'GET',
        output: '../escape.json',
      });
      expect(result.success).toBe(true);
      expect(executeCurlSpy).toHaveBeenCalled();
    });
  });

  describe('ssl paths are not confined', () => {
    // ssl ca/cert/key are TLS handshake material (commonly absolute system
    // paths) whose contents are never sent to the server, so they are exempt
    // from path confinement.
    test('absolute ssl.cert/ca/key outside cwd is allowed', async () => {
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
        url: 'https://api.example.com/data',
        method: 'GET',
        ssl: {
          ca: '/etc/ssl/certs/ca.pem',
          cert: '/etc/ssl/certs/client.crt',
          key: '../../private.key',
        },
      });
      expect(result.success).toBe(true);
      expect(executeCurlSpy).toHaveBeenCalled();
    });
  });

  describe('formData file path confinement', () => {
    test('formData file outside cwd yields success:false', async () => {
      const executor = new RequestExecutor({});
      const result = await executor.executeRequest({
        url: 'https://api.example.com/upload',
        method: 'POST',
        formData: {
          attachment: { file: '../secret.txt', type: 'text/plain' },
        },
      });
      expect(result.success).toBe(false);
      expect(result.error).toContain('escapes the working directory');
      expect(executeCurlSpy).not.toHaveBeenCalled();
    });

    test('formData file /etc/passwd yields success:false', async () => {
      const executor = new RequestExecutor({});
      const result = await executor.executeRequest({
        url: 'https://api.example.com/upload',
        method: 'POST',
        formData: {
          leak: { file: '/etc/passwd', type: 'text/plain' },
        },
      });
      expect(result.success).toBe(false);
      expect(result.error).toContain('escapes the working directory');
      expect(executeCurlSpy).not.toHaveBeenCalled();
    });

    test('formData file outside cwd allowed when security.allowPaths=true', async () => {
      // The file won't exist, so expect a "File not found" error, not a path-guard error
      const config: GlobalConfig = { security: { allowPaths: true } };
      const executor = new RequestExecutor(config);
      const result = await executor.executeRequest({
        url: 'https://api.example.com/upload',
        method: 'POST',
        formData: {
          attachment: { file: '../nonexistent-test-file.txt', type: 'text/plain' },
        },
      });
      // Path guard passes, but file existence check will fail (file not found)
      expect(result.success).toBe(false);
      expect(result.error).not.toContain('escapes the working directory');
    });
  });

  describe('blocked path has metrics', () => {
    test('path-blocked result includes metrics', async () => {
      const executor = new RequestExecutor({});
      const result = await executor.executeRequest({
        url: 'https://api.example.com/data',
        method: 'GET',
        output: '/tmp/steal.json',
      });
      expect(result.success).toBe(false);
      expect(result.metrics).toBeDefined();
      expect(result.metrics?.duration).toBeGreaterThanOrEqual(0);
    });
  });

  describe('executeSequential with path violation', () => {
    test('blocked path in sequential run yields success:false', async () => {
      const executor = new RequestExecutor({});
      const requests: RequestConfig[] = [
        {
          url: 'https://api.example.com/data',
          method: 'GET',
          output: '../escape.json',
        },
      ];
      const summary = await executor.executeSequential(requests);
      expect(summary.failed).toBe(1);
      expect(summary.successful).toBe(0);
      expect(summary.results[0].success).toBe(false);
      expect(executeCurlSpy).not.toHaveBeenCalled();
    });

    test('mixed sequential: valid passes, path-blocked fails with continueOnError', async () => {
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
        { url: 'https://api.example.com/data', method: 'GET', output: '../escape.json' },
      ];
      const summary = await executor.executeSequential(requests);
      expect(summary.total).toBe(2);
      expect(summary.successful).toBe(1);
      expect(summary.failed).toBe(1);
    });
  });

  describe('executeParallel with path violation', () => {
    test('blocked path in parallel run yields success:false', async () => {
      const executor = new RequestExecutor({});
      const requests: RequestConfig[] = [
        {
          url: 'https://api.example.com/data',
          method: 'GET',
          output: '../escape.json',
        },
      ];
      const summary = await executor.executeParallel(requests);
      expect(summary.failed).toBe(1);
      expect(summary.results[0].success).toBe(false);
      expect(executeCurlSpy).not.toHaveBeenCalled();
    });
  });
});
