import { describe, expect, test } from 'bun:test';
import {
  extractBatchMetrics,
  extractMetricsFromOutput,
  extractResponseFromOutput,
  getStatusCode,
  isSuccessStatus,
  parseHeaderBlocks,
  parseMetrics,
} from './response-parser';
import type { CurlMetrics } from './types';

describe('parseMetrics', () => {
  test('converts seconds to milliseconds', () => {
    const raw: CurlMetrics = {
      time_total: 1.5,
      time_namelookup: 0.1,
      time_connect: 0.2,
      time_appconnect: 0.3,
      time_starttransfer: 0.5,
    };
    const result = parseMetrics(raw);

    expect(result.duration).toBe(1500);
    expect(result.dnsLookup).toBe(100);
    expect(result.tcpConnection).toBe(200);
    expect(result.tlsHandshake).toBe(300);
    expect(result.firstByte).toBe(500);
    expect(result.download).toBe(1500);
  });

  test('includes size_download', () => {
    const raw: CurlMetrics = {
      time_total: 0.5,
      size_download: 1024,
    };
    const result = parseMetrics(raw);

    expect(result.size).toBe(1024);
  });

  test('handles missing fields', () => {
    const raw: CurlMetrics = {};
    const result = parseMetrics(raw);

    expect(result.duration).toBe(0);
    expect(result.dnsLookup).toBe(0);
    expect(result.tcpConnection).toBe(0);
    expect(result.tlsHandshake).toBe(0);
    expect(result.firstByte).toBe(0);
    expect(result.download).toBe(0);
  });

  test('handles zero values', () => {
    const raw: CurlMetrics = {
      time_total: 0,
      size_download: 0,
    };
    const result = parseMetrics(raw);

    expect(result.duration).toBe(0);
    expect(result.size).toBe(0);
  });
});

describe('extractMetricsFromOutput', () => {
  test('extracts metrics and body from output', () => {
    const stdout = `{"data":"test"}
__CURL_METRICS_START__{"response_code":200,"time_total":0.5}__CURL_METRICS_END__`;

    const { body, metrics } = extractMetricsFromOutput(stdout);

    expect(body).toBe('{"data":"test"}');
    expect(metrics.response_code).toBe(200);
    expect(metrics.time_total).toBe(0.5);
  });

  test('returns original body when no markers', () => {
    const stdout = '{"data":"test"}';
    const { body, metrics } = extractMetricsFromOutput(stdout);

    expect(body).toBe('{"data":"test"}');
    expect(metrics).toEqual({});
  });

  test('handles empty metrics JSON', () => {
    const stdout = `data
__CURL_METRICS_START__{}__CURL_METRICS_END__`;

    const { body, metrics } = extractMetricsFromOutput(stdout);

    expect(body).toBe('data');
    expect(metrics).toEqual({});
  });

  test('handles invalid JSON in markers gracefully', () => {
    const stdout = `data
__CURL_METRICS_START__invalid-json__CURL_METRICS_END__`;

    const { body, metrics } = extractMetricsFromOutput(stdout);

    expect(body).toBe('data');
    expect(metrics).toEqual({});
  });

  test('trims whitespace from body', () => {
    const stdout = `  {"data":"test"}
__CURL_METRICS_START__{"response_code":200}__CURL_METRICS_END__`;

    const { body } = extractMetricsFromOutput(stdout);

    expect(body).toBe('{"data":"test"}');
  });

  test('handles multiline body', () => {
    const stdout = `line1
line2
line3
__CURL_METRICS_START__{"response_code":200}__CURL_METRICS_END__`;

    const { body } = extractMetricsFromOutput(stdout);

    expect(body).toBe('line1\nline2\nline3');
  });
});

describe('extractBatchMetrics', () => {
  test('extracts metrics for specific index', () => {
    const stdout = `body0
__CURL_BATCH_0_START__{"response_code":200}__CURL_BATCH_0_END__
body1
__CURL_BATCH_1_START__{"response_code":201}__CURL_BATCH_1_END__`;

    const result0 = extractBatchMetrics(stdout, 0);
    expect(result0.found).toBe(true);
    expect(result0.metricsJson).toBe('{"response_code":200}');

    const result1 = extractBatchMetrics(stdout, 1);
    expect(result1.found).toBe(true);
    expect(result1.metricsJson).toBe('{"response_code":201}');
  });

  test('returns found=false when markers not found', () => {
    const stdout = 'no markers here';
    const result = extractBatchMetrics(stdout, 0);

    expect(result.found).toBe(false);
    expect(result.metricsJson).toBe('');
    expect(result.startIdx).toBe(-1);
    expect(result.endIdx).toBe(-1);
  });

  test('returns found=false when only start marker found', () => {
    const stdout = '__CURL_BATCH_0_START__{"data":"incomplete"}';
    const result = extractBatchMetrics(stdout, 0);

    expect(result.found).toBe(false);
  });

  test('returns found=false when only end marker found', () => {
    const stdout = '{"data":"incomplete"}__CURL_BATCH_0_END__';
    const result = extractBatchMetrics(stdout, 0);

    expect(result.found).toBe(false);
  });

  test('provides correct indices', () => {
    const stdout = 'prefix__CURL_BATCH_5_START__json__CURL_BATCH_5_END__suffix';
    const result = extractBatchMetrics(stdout, 5);

    expect(result.found).toBe(true);
    expect(result.startIdx).toBe(6); // Start of __CURL_BATCH_5_START__
    expect(result.endIdx).toBe(stdout.indexOf('__CURL_BATCH_5_END__'));
    expect(result.metricsJson).toBe('json');
  });

  test('does not match wrong index', () => {
    const stdout = '__CURL_BATCH_0_START__data__CURL_BATCH_0_END__';
    const result = extractBatchMetrics(stdout, 1);

    expect(result.found).toBe(false);
  });
});

describe('parseHeaderBlocks', () => {
  test('parses HTTP/1.1 block with CRLF', () => {
    const text = 'HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Custom: value\r\n\r\nbody';
    const { blocks, bodyOffset } = parseHeaderBlocks(text);

    expect(blocks).toHaveLength(1);
    expect(blocks[0].status).toBe(200);
    expect(blocks[0].headers['content-type']).toBe('application/json');
    expect(blocks[0].headers['x-custom']).toBe('value');
    expect(text.slice(bodyOffset)).toBe('body');
  });

  test('parses HTTP/2 status line', () => {
    const text = 'HTTP/2 200\r\nContent-Type: application/json\r\n\r\n{}';
    const { blocks } = parseHeaderBlocks(text);

    expect(blocks).toHaveLength(1);
    expect(blocks[0].status).toBe(200);
    expect(blocks[0].headers['content-type']).toBe('application/json');
  });

  test('handles LF-only line endings', () => {
    const text = 'HTTP/1.1 200 OK\nContent-Type: application/json\n\nbody';
    const { blocks, bodyOffset } = parseHeaderBlocks(text);

    expect(blocks[0].headers['content-type']).toBe('application/json');
    expect(text.slice(bodyOffset)).toBe('body');
  });

  test('parses redirect chain → returns all blocks', () => {
    const text =
      'HTTP/1.1 301 Moved Permanently\r\nLocation: /next\r\n\r\nHTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nfinal';
    const { blocks, bodyOffset } = parseHeaderBlocks(text);

    expect(blocks).toHaveLength(2);
    expect(blocks[0].status).toBe(301);
    expect(blocks[1].status).toBe(200);
    expect(text.slice(bodyOffset)).toBe('final');
  });

  test('handles 1xx informational preamble', () => {
    const text =
      'HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\nbody';
    const { blocks } = parseHeaderBlocks(text);

    expect(blocks).toHaveLength(2);
    expect(blocks[0].status).toBe(100);
    expect(blocks[1].status).toBe(200);
  });

  test('handles HEAD-style empty body', () => {
    const text = 'HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n';
    const { blocks, bodyOffset } = parseHeaderBlocks(text);

    expect(blocks[0].status).toBe(200);
    expect(text.slice(bodyOffset)).toBe('');
  });

  test('does not consume body containing \\r\\n\\r\\n', () => {
    const text = 'HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nfirst\r\n\r\nsecond';
    const { blocks, bodyOffset } = parseHeaderBlocks(text);

    expect(blocks).toHaveLength(1);
    expect(text.slice(bodyOffset)).toBe('first\r\n\r\nsecond');
  });

  test('returns no blocks when no status line', () => {
    const { blocks, bodyOffset } = parseHeaderBlocks('just a body');
    expect(blocks).toEqual([]);
    expect(bodyOffset).toBe(0);
  });

  test('consumes CONNECT proxy preamble', () => {
    const text =
      'HTTP/1.1 200 Connection established\r\n\r\nHTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\nbody';
    const { blocks } = parseHeaderBlocks(text);

    expect(blocks).toHaveLength(2);
  });

  test('repeated headers become string[]', () => {
    const text = 'HTTP/1.1 200 OK\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\n\r\nbody';
    const { blocks } = parseHeaderBlocks(text);

    expect(blocks[0].headers['set-cookie']).toEqual(['a=1', 'b=2']);
  });

  test('lowercases header keys', () => {
    const text = 'HTTP/1.1 200 OK\r\nX-CUSTOM: value\r\n\r\n';
    const { blocks } = parseHeaderBlocks(text);

    expect(blocks[0].headers['x-custom']).toBe('value');
  });
});

describe('extractResponseFromOutput', () => {
  test('extracts headers, body, and metrics in one call', () => {
    const stdout =
      'HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{"x":1}\n__CURL_METRICS_START__{"http_code":200,"time_total":0.1}__CURL_METRICS_END__';
    const { headers, status, body, metrics, headerHistory } = extractResponseFromOutput(stdout);

    expect(headers['content-type']).toBe('application/json');
    expect(status).toBe(200);
    expect(body).toBe('{"x":1}');
    expect(metrics.http_code).toBe(200);
    expect(headerHistory).toHaveLength(1);
  });

  test('picks last non-informational block on redirect chain', () => {
    const stdout =
      'HTTP/1.1 301 Moved\r\nLocation: /next\r\n\r\nHTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\nfinal\n__CURL_METRICS_START__{"http_code":200}__CURL_METRICS_END__';
    const { headers, status, headerHistory } = extractResponseFromOutput(stdout);

    expect(status).toBe(200);
    expect(headers['content-type']).toBe('text/html');
    expect(headerHistory).toHaveLength(2);
    expect(headerHistory[0].status).toBe(301);
  });

  test('skips 1xx informational when picking final headers', () => {
    const stdout =
      'HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 204 No Content\r\nX-Marker: end\r\n\r\n__CURL_METRICS_START__{"http_code":204}__CURL_METRICS_END__';
    const { headers, status } = extractResponseFromOutput(stdout);

    expect(status).toBe(204);
    expect(headers['x-marker']).toBe('end');
  });

  test('returns empty headers when no header block', () => {
    const stdout = 'plain\n__CURL_METRICS_START__{"http_code":200}__CURL_METRICS_END__';
    const { headers, status, body } = extractResponseFromOutput(stdout);

    expect(headers).toEqual({});
    expect(status).toBeUndefined();
    expect(body).toBe('plain');
  });
});

describe('getStatusCode', () => {
  test('returns response_code', () => {
    const metrics: CurlMetrics = { response_code: 200 };
    expect(getStatusCode(metrics)).toBe(200);
  });

  test('returns http_code as fallback', () => {
    const metrics: CurlMetrics = { http_code: 201 };
    expect(getStatusCode(metrics)).toBe(201);
  });

  test('prefers response_code over http_code', () => {
    const metrics: CurlMetrics = { response_code: 200, http_code: 201 };
    expect(getStatusCode(metrics)).toBe(200);
  });

  test('returns undefined when no status codes', () => {
    const metrics: CurlMetrics = {};
    expect(getStatusCode(metrics)).toBeUndefined();
  });
});

describe('isSuccessStatus', () => {
  test('returns true for 2xx status codes', () => {
    expect(isSuccessStatus(200)).toBe(true);
    expect(isSuccessStatus(201)).toBe(true);
    expect(isSuccessStatus(204)).toBe(true);
    expect(isSuccessStatus(299)).toBe(true);
  });

  test('returns true for 3xx status codes', () => {
    expect(isSuccessStatus(300)).toBe(true);
    expect(isSuccessStatus(301)).toBe(true);
    expect(isSuccessStatus(302)).toBe(true);
    expect(isSuccessStatus(399)).toBe(true);
  });

  test('returns false for 4xx status codes', () => {
    expect(isSuccessStatus(400)).toBe(false);
    expect(isSuccessStatus(401)).toBe(false);
    expect(isSuccessStatus(404)).toBe(false);
    expect(isSuccessStatus(499)).toBe(false);
  });

  test('returns false for 5xx status codes', () => {
    expect(isSuccessStatus(500)).toBe(false);
    expect(isSuccessStatus(502)).toBe(false);
    expect(isSuccessStatus(503)).toBe(false);
    expect(isSuccessStatus(599)).toBe(false);
  });

  test('returns false for 1xx status codes', () => {
    expect(isSuccessStatus(100)).toBe(false);
    expect(isSuccessStatus(101)).toBe(false);
  });

  test('returns false for undefined', () => {
    expect(isSuccessStatus(undefined)).toBe(false);
  });
});
