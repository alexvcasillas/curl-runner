/**
 * Shared curl response and metrics parsing logic.
 */

import { METRICS_MARKER_END, METRICS_MARKER_START } from './args-builder';
import type { CurlMetrics, ProcessedMetrics } from './types';

/**
 * Parses raw curl metrics JSON into processed metrics (in milliseconds).
 */
export function parseMetrics(raw: CurlMetrics): ProcessedMetrics {
  return {
    duration: (raw.time_total || 0) * 1000,
    size: raw.size_download,
    dnsLookup: (raw.time_namelookup || 0) * 1000,
    tcpConnection: (raw.time_connect || 0) * 1000,
    tlsHandshake: (raw.time_appconnect || 0) * 1000,
    firstByte: (raw.time_starttransfer || 0) * 1000,
    download: (raw.time_total || 0) * 1000,
  };
}

/**
 * Extracts metrics JSON from curl output using standard markers.
 * Returns the raw metrics and the response body (with markers removed).
 */
export function extractMetricsFromOutput(stdout: string): {
  body: string;
  metrics: CurlMetrics;
} {
  const metricsMatch = stdout.match(
    new RegExp(`${METRICS_MARKER_START}(.+?)${METRICS_MARKER_END}`),
  );

  if (!metricsMatch) {
    return { body: stdout, metrics: {} };
  }

  const body = stdout
    .replace(new RegExp(`${METRICS_MARKER_START}.+?${METRICS_MARKER_END}`), '')
    .trim();

  let metrics: CurlMetrics = {};
  try {
    metrics = JSON.parse(metricsMatch[1]);
  } catch {
    // Ignore parse errors, return empty metrics
  }

  return { body, metrics };
}

/**
 * Extracts metrics from batched curl output using index-specific markers.
 */
export function extractBatchMetrics(
  stdout: string,
  requestIndex: number,
): {
  found: boolean;
  metricsJson: string;
  startIdx: number;
  endIdx: number;
} {
  const markerStart = `__CURL_BATCH_${requestIndex}_START__`;
  const markerEnd = `__CURL_BATCH_${requestIndex}_END__`;

  const startIdx = stdout.indexOf(markerStart);
  const endIdx = stdout.indexOf(markerEnd);

  if (startIdx === -1 || endIdx === -1) {
    return { found: false, metricsJson: '', startIdx: -1, endIdx: -1 };
  }

  const metricsJson = stdout.substring(startIdx + markerStart.length, endIdx);
  return { found: true, metricsJson, startIdx, endIdx };
}

/**
 * Parses headers from curl stderr output.
 */
export function parseHeadersFromStderr(stderr: string): Record<string, string> {
  const headers: Record<string, string> = {};
  const headerLines = stderr.split('\n').filter((line) => line.includes(':'));

  for (const line of headerLines) {
    const [key, ...valueParts] = line.split(':');
    if (key && valueParts.length > 0) {
      headers[key.trim()] = valueParts.join(':').trim();
    }
  }

  return headers;
}

/**
 * Gets the status code from curl metrics.
 */
export function getStatusCode(metrics: CurlMetrics): number | undefined {
  return metrics.response_code || metrics.http_code;
}

/**
 * Determines if a response is successful based on status code.
 */
export function isSuccessStatus(statusCode: number | undefined): boolean {
  if (statusCode === undefined) {
    return false;
  }
  return statusCode >= 200 && statusCode < 400;
}
