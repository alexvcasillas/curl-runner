/**
 * Shared curl response and metrics parsing logic.
 */

import { METRICS_MARKER_END, METRICS_MARKER_START } from './args-builder';
import type { CurlMetrics, ProcessedMetrics, ResponseHeaderBlock } from './types';

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

const STATUS_LINE_RE = /^HTTP\/[\d.]+\s+(\d{3})/;

/**
 * Parses sequential HTTP header blocks emitted by `curl -D -`.
 * Handles redirects, 1xx informational (empty blocks), and CONNECT proxy preambles.
 * Header keys are lowercased; repeated keys produce string[] values.
 */
export function parseHeaderBlocks(text: string): {
  blocks: ResponseHeaderBlock[];
  bodyOffset: number;
} {
  const blocks: ResponseHeaderBlock[] = [];
  let i = 0;

  while (i < text.length) {
    // Skip leading whitespace between blocks (handles batched-output preamble)
    while (i < text.length) {
      const ch = text.charCodeAt(i);
      if (ch === 13 || ch === 10 || ch === 32 || ch === 9) {
        i++;
      } else {
        break;
      }
    }
    if (i >= text.length) {
      break;
    }

    const statusMatch = text.slice(i).match(STATUS_LINE_RE);
    if (!statusMatch) {
      break;
    }

    const status = Number(statusMatch[1]);

    const crlfIdx = text.indexOf('\r\n\r\n', i);
    const lfIdx = text.indexOf('\n\n', i);

    let endIdx: number;
    let sepLen: number;
    if (crlfIdx !== -1 && (lfIdx === -1 || crlfIdx <= lfIdx)) {
      endIdx = crlfIdx;
      sepLen = 4;
    } else if (lfIdx !== -1) {
      endIdx = lfIdx;
      sepLen = 2;
    } else {
      break;
    }

    const statusLineNl = text.indexOf('\n', i);
    const sectionStart = statusLineNl + 1;

    const headers: Record<string, string | string[]> = {};
    if (sectionStart > 0 && sectionStart < endIdx) {
      for (const line of text.slice(sectionStart, endIdx).split(/\r?\n/)) {
        if (!line) {
          continue;
        }
        const colonIdx = line.indexOf(':');
        if (colonIdx === -1) {
          continue;
        }
        const key = line.slice(0, colonIdx).trim().toLowerCase();
        if (!key) {
          continue;
        }
        const value = line.slice(colonIdx + 1).trim();
        const existing = headers[key];
        if (existing === undefined) {
          headers[key] = value;
        } else if (Array.isArray(existing)) {
          existing.push(value);
        } else {
          headers[key] = [existing, value];
        }
      }
    }

    blocks.push({ status, headers });
    i = endIdx + sepLen;
  }

  return { blocks, bodyOffset: i };
}

/**
 * Extracts headers, status, body and metrics from curl stdout when invoked
 * with both `-D -` (header dump) and `-w` (metrics marker).
 */
export function extractResponseFromOutput(stdout: string): {
  headers: Record<string, string | string[]>;
  status?: number;
  headerHistory: ResponseHeaderBlock[];
  body: string;
  metrics: CurlMetrics;
} {
  const { blocks, bodyOffset } = parseHeaderBlocks(stdout);

  let finalBlock: ResponseHeaderBlock | undefined;
  for (let k = blocks.length - 1; k >= 0; k--) {
    if (blocks[k].status >= 200) {
      finalBlock = blocks[k];
      break;
    }
  }

  const { body, metrics } = extractMetricsFromOutput(stdout.slice(bodyOffset));

  return {
    headers: finalBlock?.headers ?? {},
    status: finalBlock?.status,
    headerHistory: blocks,
    body,
    metrics,
  };
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
