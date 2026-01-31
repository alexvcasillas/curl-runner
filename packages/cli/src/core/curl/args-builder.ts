/**
 * Shared curl argument building logic.
 * Used by both CurlBuilder (single requests) and PooledCurlExecutor (batched requests).
 */

import type { RequestConfig } from '../../types/config';
import { type CurlArgsOptions, type CurlArgsResult, isFileAttachment } from './types';

const DEFAULT_WRITE_OUT = '\n__CURL_METRICS_START__%{json}__CURL_METRICS_END__';

/**
 * Builds curl command-line arguments from a request config.
 * This is the core shared logic used by all curl execution paths.
 */
export function buildCurlArgs(
  config: RequestConfig,
  options: CurlArgsOptions = {},
): CurlArgsResult {
  const {
    writeOutMarker = DEFAULT_WRITE_OUT,
    includeSilentFlags = true,
    includeHttp2Flag = true,
    includeOutputFlag = true,
  } = options;

  const args: string[] = [];

  // HTTP method
  args.push('-X', config.method || 'GET');

  // Write-out format for metrics
  args.push('-w', writeOutMarker);

  // Headers
  if (config.headers) {
    for (const [key, value] of Object.entries(config.headers)) {
      args.push('-H', `${key}: ${value}`);
    }
  }

  // Authentication
  if (config.auth) {
    if (config.auth.type === 'basic' && config.auth.username && config.auth.password) {
      args.push('-u', `${config.auth.username}:${config.auth.password}`);
    } else if (config.auth.type === 'bearer' && config.auth.token) {
      args.push('-H', `Authorization: Bearer ${config.auth.token}`);
    }
  }

  // Body or form data
  if (config.formData) {
    for (const [fieldName, fieldValue] of Object.entries(config.formData)) {
      if (isFileAttachment(fieldValue)) {
        // File attachment: -F "field=@filepath;filename=name;type=mimetype"
        let fileSpec = `@${fieldValue.file}`;
        if (fieldValue.filename) {
          fileSpec += `;filename=${fieldValue.filename}`;
        }
        if (fieldValue.contentType) {
          fileSpec += `;type=${fieldValue.contentType}`;
        }
        args.push('-F', `${fieldName}=${fileSpec}`);
      } else {
        // Regular form field: --form-string prevents @ and < interpretation
        const strValue = String(fieldValue);
        args.push('--form-string', `${fieldName}=${strValue}`);
      }
    }
  } else if (config.body) {
    const bodyStr = typeof config.body === 'string' ? config.body : JSON.stringify(config.body);
    args.push('-d', bodyStr);

    // Auto-add Content-Type if not present
    if (!config.headers?.['Content-Type']) {
      args.push('-H', 'Content-Type: application/json');
    }
  }

  // Timeout
  if (config.timeout) {
    args.push('--max-time', config.timeout.toString());
  }

  // Follow redirects
  if (config.followRedirects !== false) {
    args.push('-L');
    if (config.maxRedirects) {
      args.push('--max-redirs', config.maxRedirects.toString());
    }
  }

  // Proxy
  if (config.proxy) {
    args.push('-x', config.proxy);
  }

  // SSL/TLS configuration
  if (config.insecure || config.ssl?.verify === false) {
    args.push('-k');
  }

  if (config.ssl) {
    if (config.ssl.ca) {
      args.push('--cacert', config.ssl.ca);
    }
    if (config.ssl.cert) {
      args.push('--cert', config.ssl.cert);
    }
    if (config.ssl.key) {
      args.push('--key', config.ssl.key);
    }
  }

  // Output file
  if (includeOutputFlag && config.output) {
    args.push('-o', config.output);
  }

  // HTTP/2
  if (includeHttp2Flag && config.http2) {
    args.push('--http2');
  }

  // Silent mode (suppress progress but show errors)
  if (includeSilentFlags) {
    args.push('-s', '-S');
  }

  // Build final URL with query params
  let url = config.url;
  if (config.params && Object.keys(config.params).length > 0) {
    const queryString = new URLSearchParams(config.params).toString();
    url += (url.includes('?') ? '&' : '?') + queryString;
  }

  return { args, url };
}

/**
 * Formats curl args array as shell-safe command string for display/debugging.
 */
export function formatArgsForDisplay(args: string[]): string {
  return ['curl', ...args]
    .map((arg) =>
      arg.includes(' ') || arg.includes('"') || arg.includes("'")
        ? `'${arg.replace(/'/g, "'\\''")}'`
        : arg,
    )
    .join(' ');
}

/**
 * Creates a write-out marker for batched requests.
 */
export function createBatchMarker(requestIndex: number): string {
  return `\n__CURL_BATCH_${requestIndex}_START__%{json}__CURL_BATCH_${requestIndex}_END__\n`;
}

/**
 * Marker constants for parsing.
 */
export const METRICS_MARKER_START = '__CURL_METRICS_START__';
export const METRICS_MARKER_END = '__CURL_METRICS_END__';
