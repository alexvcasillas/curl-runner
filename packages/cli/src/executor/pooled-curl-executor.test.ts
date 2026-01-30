import { describe, expect, test } from 'bun:test';
import type { RequestConfig } from '../types/config';
import { PooledCurlExecutor } from './pooled-curl-executor';

describe('PooledCurlExecutor', () => {
  describe('groupRequestsByHost', () => {
    test('should group requests by host', () => {
      const executor = new PooledCurlExecutor();
      const requests: RequestConfig[] = [
        { url: 'https://api.example.com/users', method: 'GET' },
        { url: 'https://api.example.com/items', method: 'GET' },
        { url: 'https://other.com/data', method: 'GET' },
        { url: 'https://api.example.com/orders', method: 'POST' },
      ];

      const groups = executor.groupRequestsByHost(requests);

      expect(groups).toHaveLength(2);

      const exampleGroup = groups.find((g) => g.host === 'https://api.example.com');
      expect(exampleGroup).toBeDefined();
      expect(exampleGroup!.requests).toHaveLength(3);
      expect(exampleGroup!.requests.map((r) => r.index)).toEqual([0, 1, 3]);

      const otherGroup = groups.find((g) => g.host === 'https://other.com');
      expect(otherGroup).toBeDefined();
      expect(otherGroup!.requests).toHaveLength(1);
      expect(otherGroup!.requests[0].index).toBe(2);
    });

    test('should handle different ports as different hosts', () => {
      const executor = new PooledCurlExecutor();
      const requests: RequestConfig[] = [
        { url: 'https://api.example.com:443/users', method: 'GET' },
        { url: 'https://api.example.com:8443/users', method: 'GET' },
      ];

      const groups = executor.groupRequestsByHost(requests);

      expect(groups).toHaveLength(2);
    });

    test('should handle http vs https as different hosts', () => {
      const executor = new PooledCurlExecutor();
      const requests: RequestConfig[] = [
        { url: 'https://api.example.com/users', method: 'GET' },
        { url: 'http://api.example.com/users', method: 'GET' },
      ];

      const groups = executor.groupRequestsByHost(requests);

      expect(groups).toHaveLength(2);
    });

    test('should handle single request', () => {
      const executor = new PooledCurlExecutor();
      const requests: RequestConfig[] = [{ url: 'https://api.example.com/users', method: 'GET' }];

      const groups = executor.groupRequestsByHost(requests);

      expect(groups).toHaveLength(1);
      expect(groups[0].requests).toHaveLength(1);
    });

    test('should handle empty requests', () => {
      const executor = new PooledCurlExecutor();
      const groups = executor.groupRequestsByHost([]);

      expect(groups).toHaveLength(0);
    });
  });

  describe('buildBatchedCommand', () => {
    test('should build batched command with parallel flags', () => {
      const executor = new PooledCurlExecutor({ maxStreamsPerHost: 5 });
      const group = {
        host: 'https://api.example.com',
        requests: [
          { index: 0, config: { url: 'https://api.example.com/users', method: 'GET' as const } },
        ],
      };

      const args = executor.buildBatchedCommand(group);

      expect(args).toContain('-Z');
      expect(args).toContain('--parallel-max');
      expect(args).toContain('5');
      expect(args).toContain('--http2');
      expect(args).toContain('--keepalive-time');
      expect(args).toContain('-s');
      expect(args).toContain('-S');
    });

    test('should use --next to separate multiple requests', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          { index: 0, config: { url: 'https://api.example.com/users', method: 'GET' as const } },
          {
            index: 1,
            config: {
              url: 'https://api.example.com/items',
              method: 'POST' as const,
              body: { name: 'test' },
            },
          },
        ],
      };

      const args = executor.buildBatchedCommand(group);

      expect(args).toContain('--next');
      expect(args.indexOf('--next')).toBeGreaterThan(0);
    });

    test('should include unique markers per request', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          { index: 0, config: { url: 'https://api.example.com/users', method: 'GET' as const } },
          { index: 1, config: { url: 'https://api.example.com/items', method: 'GET' as const } },
        ],
      };

      const args = executor.buildBatchedCommand(group);
      const argsStr = args.join(' ');

      expect(argsStr).toContain('__CURL_BATCH_0_START__');
      expect(argsStr).toContain('__CURL_BATCH_0_END__');
      expect(argsStr).toContain('__CURL_BATCH_1_START__');
      expect(argsStr).toContain('__CURL_BATCH_1_END__');
    });

    test('should handle request with headers', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          {
            index: 0,
            config: {
              url: 'https://api.example.com/users',
              method: 'GET' as const,
              headers: { Authorization: 'Bearer token123' },
            },
          },
        ],
      };

      const args = executor.buildBatchedCommand(group);

      expect(args).toContain('-H');
      expect(args).toContain('Authorization: Bearer token123');
    });

    test('should handle request with JSON body', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          {
            index: 0,
            config: {
              url: 'https://api.example.com/users',
              method: 'POST' as const,
              body: { name: 'John', age: 30 },
            },
          },
        ],
      };

      const args = executor.buildBatchedCommand(group);

      expect(args).toContain('-d');
      expect(args).toContain('{"name":"John","age":30}');
      expect(args).toContain('Content-Type: application/json');
    });

    test('should handle request with query params', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          {
            index: 0,
            config: {
              url: 'https://api.example.com/users',
              method: 'GET' as const,
              params: { page: '1', limit: '10' },
            },
          },
        ],
      };

      const args = executor.buildBatchedCommand(group);

      expect(args).toContain('https://api.example.com/users?page=1&limit=10');
    });

    test('should handle basic auth', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          {
            index: 0,
            config: {
              url: 'https://api.example.com/users',
              method: 'GET' as const,
              auth: { type: 'basic' as const, username: 'user', password: 'pass' },
            },
          },
        ],
      };

      const args = executor.buildBatchedCommand(group);

      expect(args).toContain('-u');
      expect(args).toContain('user:pass');
    });

    test('should handle bearer auth', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          {
            index: 0,
            config: {
              url: 'https://api.example.com/users',
              method: 'GET' as const,
              auth: { type: 'bearer' as const, token: 'mytoken' },
            },
          },
        ],
      };

      const args = executor.buildBatchedCommand(group);

      expect(args).toContain('-H');
      expect(args).toContain('Authorization: Bearer mytoken');
    });

    test('should handle SSL config', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          {
            index: 0,
            config: {
              url: 'https://api.example.com/users',
              method: 'GET' as const,
              ssl: { verify: false, ca: '/path/to/ca.pem' },
            },
          },
        ],
      };

      const args = executor.buildBatchedCommand(group);

      expect(args).toContain('-k');
      expect(args).toContain('--cacert');
      expect(args).toContain('/path/to/ca.pem');
    });

    test('should handle timeout', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          {
            index: 0,
            config: {
              url: 'https://api.example.com/users',
              method: 'GET' as const,
              timeout: 30,
            },
          },
        ],
      };

      const args = executor.buildBatchedCommand(group);

      expect(args).toContain('--max-time');
      expect(args).toContain('30');
    });

    test('should handle follow redirects', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          {
            index: 0,
            config: {
              url: 'https://api.example.com/users',
              method: 'GET' as const,
              followRedirects: true,
              maxRedirects: 5,
            },
          },
        ],
      };

      const args = executor.buildBatchedCommand(group);

      expect(args).toContain('-L');
      expect(args).toContain('--max-redirs');
      expect(args).toContain('5');
    });

    test('should handle proxy', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          {
            index: 0,
            config: {
              url: 'https://api.example.com/users',
              method: 'GET' as const,
              proxy: 'http://proxy:8080',
            },
          },
        ],
      };

      const args = executor.buildBatchedCommand(group);

      expect(args).toContain('-x');
      expect(args).toContain('http://proxy:8080');
    });
  });

  describe('parseBatchedOutput', () => {
    test('should parse single request output', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          { index: 0, config: { url: 'https://api.example.com/users', method: 'GET' as const } },
        ],
      };

      const stdout = `{"id":1,"name":"John"}
__CURL_BATCH_0_START__{"response_code":200,"time_total":0.5}__CURL_BATCH_0_END__
`;
      const stderr = '';

      const results = executor.parseBatchedOutput(stdout, stderr, group, 0);

      expect(results.size).toBe(1);
      const result = results.get(0)!;
      expect(result.success).toBe(true);
      expect(result.status).toBe(200);
      expect(result.metrics?.duration).toBe(500);
    });

    test('should parse multiple request outputs', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          { index: 0, config: { url: 'https://api.example.com/users', method: 'GET' as const } },
          { index: 1, config: { url: 'https://api.example.com/items', method: 'GET' as const } },
        ],
      };

      const stdout = `{"users":[]}
__CURL_BATCH_0_START__{"response_code":200,"time_total":0.3}__CURL_BATCH_0_END__
{"items":[]}
__CURL_BATCH_1_START__{"response_code":200,"time_total":0.4}__CURL_BATCH_1_END__
`;
      const stderr = '';

      const results = executor.parseBatchedOutput(stdout, stderr, group, 0);

      expect(results.size).toBe(2);
      expect(results.get(0)!.success).toBe(true);
      expect(results.get(0)!.status).toBe(200);
      expect(results.get(1)!.success).toBe(true);
      expect(results.get(1)!.status).toBe(200);
    });

    test('should handle 4xx status as failure', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          { index: 0, config: { url: 'https://api.example.com/users', method: 'GET' as const } },
        ],
      };

      const stdout = `{"error":"Not found"}
__CURL_BATCH_0_START__{"response_code":404,"time_total":0.2}__CURL_BATCH_0_END__
`;
      const stderr = '';

      const results = executor.parseBatchedOutput(stdout, stderr, group, 0);

      expect(results.get(0)!.success).toBe(false);
      expect(results.get(0)!.status).toBe(404);
    });

    test('should handle missing markers as failure', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          { index: 0, config: { url: 'https://api.example.com/users', method: 'GET' as const } },
        ],
      };

      const stdout = '';
      const stderr = 'Connection refused';

      const results = executor.parseBatchedOutput(stdout, stderr, group, 7);

      expect(results.get(0)!.success).toBe(false);
      expect(results.get(0)!.error).toContain('Connection refused');
    });

    test('should parse timing metrics', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          { index: 0, config: { url: 'https://api.example.com/users', method: 'GET' as const } },
        ],
      };

      const stdout = `{}
__CURL_BATCH_0_START__{"response_code":200,"time_total":0.5,"time_namelookup":0.01,"time_connect":0.05,"time_appconnect":0.1,"time_starttransfer":0.3,"size_download":1024}__CURL_BATCH_0_END__
`;
      const stderr = '';

      const results = executor.parseBatchedOutput(stdout, stderr, group, 0);
      const metrics = results.get(0)!.metrics!;

      expect(metrics.duration).toBe(500);
      expect(metrics.dnsLookup).toBe(10);
      expect(metrics.tcpConnection).toBe(50);
      expect(metrics.tlsHandshake).toBe(100);
      expect(metrics.firstByte).toBe(300);
      expect(metrics.size).toBe(1024);
    });
  });

  describe('formatBatchedCommandForDisplay', () => {
    test('should format batched command for display', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          { index: 0, config: { url: 'https://api.example.com/users', method: 'GET' as const } },
        ],
      };

      const display = executor.formatBatchedCommandForDisplay(group);

      expect(display).toContain('curl');
      expect(display).toContain('-Z');
      expect(display).toContain('--http2');
      expect(display).toContain('https://api.example.com/users');
    });
  });

  describe('constructor defaults', () => {
    test('should use default pool config values', () => {
      const executor = new PooledCurlExecutor();
      const group = {
        host: 'https://api.example.com',
        requests: [
          { index: 0, config: { url: 'https://api.example.com/users', method: 'GET' as const } },
        ],
      };

      const args = executor.buildBatchedCommand(group);

      expect(args).toContain('--parallel-max');
      expect(args).toContain('10'); // default maxStreamsPerHost
      expect(args).toContain('--keepalive-time');
      expect(args).toContain('60'); // default keepaliveTime
      expect(args).toContain('--connect-timeout');
      expect(args).toContain('30'); // default connectTimeout
    });

    test('should allow custom pool config', () => {
      const executor = new PooledCurlExecutor({
        maxStreamsPerHost: 20,
        keepaliveTime: 120,
        connectTimeout: 60,
      });
      const group = {
        host: 'https://api.example.com',
        requests: [
          { index: 0, config: { url: 'https://api.example.com/users', method: 'GET' as const } },
        ],
      };

      const args = executor.buildBatchedCommand(group);

      expect(args).toContain('20');
      expect(args).toContain('120');
      expect(args).toContain('60');
    });
  });
});
