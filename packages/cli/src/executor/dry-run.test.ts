import { afterEach, beforeEach, describe, expect, mock, spyOn, test } from 'bun:test';
import type { ExecutionResult, GlobalConfig, RequestConfig } from '../types/config';
import { CurlBuilder } from '../utils/curl-builder';
import { RequestExecutor } from './request-executor';

/**
 * Tests for dry-run mode functionality.
 * Dry-run mode shows curl commands without executing them.
 */

describe('RequestExecutor Dry Run Mode', () => {
  let executeCurlSpy: ReturnType<typeof spyOn>;

  beforeEach(() => {
    // Spy on CurlBuilder.executeCurl to verify it's not called in dry-run mode
    executeCurlSpy = spyOn(CurlBuilder, 'executeCurl');
  });

  afterEach(() => {
    executeCurlSpy.mockRestore();
  });

  describe('executeRequest with dryRun enabled', () => {
    test('returns success: true with dryRun flag', async () => {
      const config: GlobalConfig = { dryRun: true };
      const executor = new RequestExecutor(config);

      const request: RequestConfig = {
        url: 'https://api.example.com/users',
        method: 'GET',
      };

      const result = await executor.executeRequest(request);

      expect(result.success).toBe(true);
      expect(result.dryRun).toBe(true);
    });

    test('returns duration of 0 in dry-run mode', async () => {
      const config: GlobalConfig = { dryRun: true };
      const executor = new RequestExecutor(config);

      const request: RequestConfig = {
        url: 'https://api.example.com/users',
        method: 'POST',
        body: { name: 'Test' },
      };

      const result = await executor.executeRequest(request);

      expect(result.metrics?.duration).toBe(0);
    });

    test('does not execute actual curl command', async () => {
      const config: GlobalConfig = { dryRun: true };
      const executor = new RequestExecutor(config);

      const request: RequestConfig = {
        url: 'https://api.example.com/data',
        method: 'GET',
      };

      await executor.executeRequest(request);

      expect(executeCurlSpy).not.toHaveBeenCalled();
    });

    test('does not include status code in result', async () => {
      const config: GlobalConfig = { dryRun: true };
      const executor = new RequestExecutor(config);

      const request: RequestConfig = {
        url: 'https://api.example.com/users',
        method: 'GET',
      };

      const result = await executor.executeRequest(request);

      expect(result.status).toBeUndefined();
    });

    test('does not include body in result', async () => {
      const config: GlobalConfig = { dryRun: true };
      const executor = new RequestExecutor(config);

      const request: RequestConfig = {
        url: 'https://api.example.com/users',
        method: 'GET',
      };

      const result = await executor.executeRequest(request);

      expect(result.body).toBeUndefined();
    });

    test('does not include headers in result', async () => {
      const config: GlobalConfig = { dryRun: true };
      const executor = new RequestExecutor(config);

      const request: RequestConfig = {
        url: 'https://api.example.com/users',
        method: 'GET',
      };

      const result = await executor.executeRequest(request);

      expect(result.headers).toBeUndefined();
    });

    test('preserves request config in result', async () => {
      const config: GlobalConfig = { dryRun: true };
      const executor = new RequestExecutor(config);

      const request: RequestConfig = {
        name: 'Get Users',
        url: 'https://api.example.com/users',
        method: 'GET',
        headers: { Authorization: 'Bearer token123' },
      };

      const result = await executor.executeRequest(request);

      expect(result.request).toEqual(request);
    });

    test('works with POST request and body', async () => {
      const config: GlobalConfig = { dryRun: true };
      const executor = new RequestExecutor(config);

      const request: RequestConfig = {
        url: 'https://api.example.com/users',
        method: 'POST',
        body: { name: 'John', email: 'john@example.com' },
        headers: { 'Content-Type': 'application/json' },
      };

      const result = await executor.executeRequest(request);

      expect(result.success).toBe(true);
      expect(result.dryRun).toBe(true);
      expect(executeCurlSpy).not.toHaveBeenCalled();
    });

    test('works with authentication config', async () => {
      const config: GlobalConfig = { dryRun: true };
      const executor = new RequestExecutor(config);

      const request: RequestConfig = {
        url: 'https://api.example.com/protected',
        method: 'GET',
        auth: {
          type: 'bearer',
          token: 'secret-token',
        },
      };

      const result = await executor.executeRequest(request);

      expect(result.success).toBe(true);
      expect(result.dryRun).toBe(true);
    });

    test('works with query parameters', async () => {
      const config: GlobalConfig = { dryRun: true };
      const executor = new RequestExecutor(config);

      const request: RequestConfig = {
        url: 'https://api.example.com/search',
        method: 'GET',
        params: { q: 'test', limit: '10' },
      };

      const result = await executor.executeRequest(request);

      expect(result.success).toBe(true);
      expect(result.dryRun).toBe(true);
    });

    test('works with timeout config', async () => {
      const config: GlobalConfig = { dryRun: true };
      const executor = new RequestExecutor(config);

      const request: RequestConfig = {
        url: 'https://api.example.com/slow',
        method: 'GET',
        timeout: 5000,
      };

      const result = await executor.executeRequest(request);

      expect(result.success).toBe(true);
      expect(result.dryRun).toBe(true);
    });

    test('works with retry config (but does not retry)', async () => {
      const config: GlobalConfig = { dryRun: true };
      const executor = new RequestExecutor(config);

      const request: RequestConfig = {
        url: 'https://api.example.com/flaky',
        method: 'GET',
        retry: { count: 3, delay: 1000 },
      };

      const result = await executor.executeRequest(request);

      expect(result.success).toBe(true);
      expect(result.dryRun).toBe(true);
      // Should not have attempted any curl calls (no retries needed)
      expect(executeCurlSpy).not.toHaveBeenCalled();
    });
  });

  describe('executeSequential with dryRun enabled', () => {
    test('processes all requests without executing', async () => {
      const config: GlobalConfig = { dryRun: true };
      const executor = new RequestExecutor(config);

      const requests: RequestConfig[] = [
        { url: 'https://api.example.com/users', method: 'GET' },
        { url: 'https://api.example.com/posts', method: 'GET' },
        { url: 'https://api.example.com/comments', method: 'GET' },
      ];

      const summary = await executor.executeSequential(requests);

      expect(summary.total).toBe(3);
      expect(summary.successful).toBe(3);
      expect(summary.failed).toBe(0);
      expect(executeCurlSpy).not.toHaveBeenCalled();
    });

    test('all results have dryRun flag', async () => {
      const config: GlobalConfig = { dryRun: true };
      const executor = new RequestExecutor(config);

      const requests: RequestConfig[] = [
        { url: 'https://api.example.com/users', method: 'GET' },
        { url: 'https://api.example.com/posts', method: 'POST', body: { title: 'Test' } },
      ];

      const summary = await executor.executeSequential(requests);

      for (const result of summary.results) {
        expect(result.dryRun).toBe(true);
        expect(result.success).toBe(true);
      }
    });

    test('skips store extraction in dry-run mode', async () => {
      const config: GlobalConfig = { dryRun: true };
      const executor = new RequestExecutor(config);

      const requests: RequestConfig[] = [
        {
          url: 'https://api.example.com/login',
          method: 'POST',
          body: { username: 'test' },
          store: { token: 'body.token' },
        },
        {
          url: 'https://api.example.com/protected',
          method: 'GET',
          headers: { Authorization: 'Bearer ${store.token}' },
        },
      ];

      const summary = await executor.executeSequential(requests);

      // Both requests should succeed in dry-run mode
      expect(summary.successful).toBe(2);
      expect(summary.failed).toBe(0);
    });
  });

  describe('executeParallel with dryRun enabled', () => {
    test('processes all requests without executing', async () => {
      const config: GlobalConfig = { dryRun: true, execution: 'parallel' };
      const executor = new RequestExecutor(config);

      const requests: RequestConfig[] = [
        { url: 'https://api.example.com/users', method: 'GET' },
        { url: 'https://api.example.com/posts', method: 'GET' },
        { url: 'https://api.example.com/comments', method: 'GET' },
      ];

      const summary = await executor.executeParallel(requests);

      expect(summary.total).toBe(3);
      expect(summary.successful).toBe(3);
      expect(summary.failed).toBe(0);
      expect(executeCurlSpy).not.toHaveBeenCalled();
    });

    test('works with maxConcurrency', async () => {
      const config: GlobalConfig = { dryRun: true, maxConcurrency: 2 };
      const executor = new RequestExecutor(config);

      const requests: RequestConfig[] = [
        { url: 'https://api.example.com/1', method: 'GET' },
        { url: 'https://api.example.com/2', method: 'GET' },
        { url: 'https://api.example.com/3', method: 'GET' },
        { url: 'https://api.example.com/4', method: 'GET' },
      ];

      const summary = await executor.executeParallel(requests);

      expect(summary.total).toBe(4);
      expect(summary.successful).toBe(4);
      for (const result of summary.results) {
        expect(result.dryRun).toBe(true);
      }
    });
  });

  describe('execute method with dryRun enabled', () => {
    test('uses sequential mode by default', async () => {
      const config: GlobalConfig = { dryRun: true };
      const executor = new RequestExecutor(config);

      const requests: RequestConfig[] = [
        { url: 'https://api.example.com/users', method: 'GET' },
      ];

      const summary = await executor.execute(requests);

      expect(summary.successful).toBe(1);
      expect(summary.results[0].dryRun).toBe(true);
    });

    test('respects parallel execution mode', async () => {
      const config: GlobalConfig = { dryRun: true, execution: 'parallel' };
      const executor = new RequestExecutor(config);

      const requests: RequestConfig[] = [
        { url: 'https://api.example.com/users', method: 'GET' },
        { url: 'https://api.example.com/posts', method: 'GET' },
      ];

      const summary = await executor.execute(requests);

      expect(summary.successful).toBe(2);
      for (const result of summary.results) {
        expect(result.dryRun).toBe(true);
      }
    });
  });
});

describe('Dry Run with Special Configurations', () => {
  let executeCurlSpy: ReturnType<typeof spyOn>;

  beforeEach(() => {
    executeCurlSpy = spyOn(CurlBuilder, 'executeCurl');
  });

  afterEach(() => {
    executeCurlSpy.mockRestore();
  });

  test('works with insecure flag', async () => {
    const config: GlobalConfig = { dryRun: true };
    const executor = new RequestExecutor(config);

    const request: RequestConfig = {
      url: 'https://self-signed.example.com/api',
      method: 'GET',
      insecure: true,
    };

    const result = await executor.executeRequest(request);

    expect(result.success).toBe(true);
    expect(result.dryRun).toBe(true);
  });

  test('works with proxy config', async () => {
    const config: GlobalConfig = { dryRun: true };
    const executor = new RequestExecutor(config);

    const request: RequestConfig = {
      url: 'https://api.example.com/data',
      method: 'GET',
      proxy: 'http://proxy.example.com:8080',
    };

    const result = await executor.executeRequest(request);

    expect(result.success).toBe(true);
    expect(result.dryRun).toBe(true);
  });

  test('works with SSL config', async () => {
    const config: GlobalConfig = { dryRun: true };
    const executor = new RequestExecutor(config);

    const request: RequestConfig = {
      url: 'https://api.example.com/secure',
      method: 'GET',
      ssl: {
        verify: true,
        ca: '/path/to/ca.pem',
        cert: '/path/to/cert.pem',
        key: '/path/to/key.pem',
      },
    };

    const result = await executor.executeRequest(request);

    expect(result.success).toBe(true);
    expect(result.dryRun).toBe(true);
  });

  test('works with redirect config', async () => {
    const config: GlobalConfig = { dryRun: true };
    const executor = new RequestExecutor(config);

    const request: RequestConfig = {
      url: 'https://api.example.com/redirect',
      method: 'GET',
      followRedirects: true,
      maxRedirects: 5,
    };

    const result = await executor.executeRequest(request);

    expect(result.success).toBe(true);
    expect(result.dryRun).toBe(true);
  });

  test('works with basic auth', async () => {
    const config: GlobalConfig = { dryRun: true };
    const executor = new RequestExecutor(config);

    const request: RequestConfig = {
      url: 'https://api.example.com/protected',
      method: 'GET',
      auth: {
        type: 'basic',
        username: 'user',
        password: 'pass',
      },
    };

    const result = await executor.executeRequest(request);

    expect(result.success).toBe(true);
    expect(result.dryRun).toBe(true);
  });

  test('skips expect validation in dry-run mode', async () => {
    const config: GlobalConfig = { dryRun: true };
    const executor = new RequestExecutor(config);

    const request: RequestConfig = {
      url: 'https://api.example.com/users',
      method: 'GET',
      expect: {
        status: 200,
        body: { id: 1, name: 'Test' },
      },
    };

    const result = await executor.executeRequest(request);

    // Should succeed even though no actual response to validate
    expect(result.success).toBe(true);
    expect(result.dryRun).toBe(true);
    expect(result.error).toBeUndefined();
  });

  test('works with all HTTP methods', async () => {
    const config: GlobalConfig = { dryRun: true };
    const executor = new RequestExecutor(config);

    const methods: RequestConfig['method'][] = [
      'GET',
      'POST',
      'PUT',
      'DELETE',
      'PATCH',
      'HEAD',
      'OPTIONS',
    ];

    for (const method of methods) {
      const request: RequestConfig = {
        url: 'https://api.example.com/resource',
        method,
      };

      const result = await executor.executeRequest(request);

      expect(result.success).toBe(true);
      expect(result.dryRun).toBe(true);
    }
  });
});

describe('Dry Run Disabled (Normal Mode)', () => {
  test('dryRun flag is false by default', () => {
    const config: GlobalConfig = {};
    expect(config.dryRun).toBeUndefined();
  });

  test('result does not have dryRun flag in normal mode', async () => {
    // Mock a successful curl response
    const mockResult = {
      success: true,
      status: 200,
      headers: { 'content-type': 'application/json' },
      body: '{"id": 1}',
      metrics: { duration: 100 },
    };

    const executeCurlSpy = spyOn(CurlBuilder, 'executeCurl').mockResolvedValue(mockResult);

    const config: GlobalConfig = { dryRun: false };
    const executor = new RequestExecutor(config);

    const request: RequestConfig = {
      url: 'https://api.example.com/users',
      method: 'GET',
    };

    const result = await executor.executeRequest(request);

    expect(result.dryRun).toBeUndefined();
    expect(executeCurlSpy).toHaveBeenCalled();

    executeCurlSpy.mockRestore();
  });
});

describe('Dry Run Result Structure', () => {
  test('result matches ExecutionResult interface', async () => {
    const config: GlobalConfig = { dryRun: true };
    const executor = new RequestExecutor(config);

    const request: RequestConfig = {
      name: 'Test Request',
      url: 'https://api.example.com/test',
      method: 'GET',
    };

    const result: ExecutionResult = await executor.executeRequest(request);

    // Required fields
    expect(result).toHaveProperty('request');
    expect(result).toHaveProperty('success');

    // Dry-run specific
    expect(result.dryRun).toBe(true);
    expect(result.metrics).toBeDefined();
    expect(result.metrics?.duration).toBe(0);

    // Should not have real response data
    expect(result.status).toBeUndefined();
    expect(result.body).toBeUndefined();
    expect(result.headers).toBeUndefined();
    expect(result.error).toBeUndefined();
  });
});
