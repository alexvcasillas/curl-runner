import { describe, expect, test } from 'bun:test';
import {
  extractBatchMetrics,
  extractMetricsFromOutput,
  getStatusCode,
  isSuccessStatus,
  parseHeadersFromStderr,
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

describe('parseHeadersFromStderr', () => {
  test('parses headers from stderr', () => {
    const stderr = `< HTTP/1.1 200 OK
< Content-Type: application/json
< X-Custom-Header: value`;

    const headers = parseHeadersFromStderr(stderr);

    expect(headers['< Content-Type']).toBe('application/json');
    expect(headers['< X-Custom-Header']).toBe('value');
  });

  test('handles headers with colons in value', () => {
    const stderr = '< Date: Mon, 01 Jan 2024 12:00:00 GMT';
    const headers = parseHeadersFromStderr(stderr);

    expect(headers['< Date']).toBe('Mon, 01 Jan 2024 12:00:00 GMT');
  });

  test('returns empty object for no headers', () => {
    const stderr = 'no headers here';
    const headers = parseHeadersFromStderr(stderr);

    expect(headers).toEqual({});
  });

  test('handles empty stderr', () => {
    const headers = parseHeadersFromStderr('');
    expect(headers).toEqual({});
  });

  test('trims whitespace', () => {
    const stderr = '<   Content-Type  :   application/json  ';
    const headers = parseHeadersFromStderr(stderr);

    expect(headers['<   Content-Type']).toBe('application/json');
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
