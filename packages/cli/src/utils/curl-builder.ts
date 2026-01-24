import type { FileAttachment, FormFieldValue, RequestConfig, SSLConfig } from '../types/config';

interface CurlMetrics {
  response_code?: number;
  http_code?: number;
  time_total?: number;
  size_download?: number;
  time_namelookup?: number;
  time_connect?: number;
  time_appconnect?: number;
  time_starttransfer?: number;
}

/**
 * Checks if a form field value is a file attachment.
 */
function isFileAttachment(value: FormFieldValue): value is FileAttachment {
  return typeof value === 'object' && value !== null && 'file' in value;
}

/**
 * Escapes a string value for use in curl -F flag.
 */
function escapeFormValue(value: string): string {
  return value.replace(/'/g, "'\\''");
}

// Using class for organization, but could be refactored to functions
export class CurlBuilder {
  static buildCommand(config: RequestConfig): string {
    const parts: string[] = ['curl'];

    parts.push('-X', config.method || 'GET');

    parts.push('-w', '"\\n__CURL_METRICS_START__%{json}__CURL_METRICS_END__"');

    if (config.headers) {
      for (const [key, value] of Object.entries(config.headers)) {
        parts.push('-H', `"${key}: ${value}"`);
      }
    }

    if (config.auth) {
      if (config.auth.type === 'basic' && config.auth.username && config.auth.password) {
        parts.push('-u', `"${config.auth.username}:${config.auth.password}"`);
      } else if (config.auth.type === 'bearer' && config.auth.token) {
        parts.push('-H', `"Authorization: Bearer ${config.auth.token}"`);
      }
    }

    if (config.formData) {
      // Use -F flags for multipart/form-data
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
          parts.push('-F', `'${fieldName}=${escapeFormValue(fileSpec)}'`);
        } else {
          // Regular form field: -F "field=value"
          const strValue = String(fieldValue);
          parts.push('-F', `'${fieldName}=${escapeFormValue(strValue)}'`);
        }
      }
    } else if (config.body) {
      const bodyStr = typeof config.body === 'string' ? config.body : JSON.stringify(config.body);
      parts.push('-d', `'${bodyStr.replace(/'/g, "'\\''")}'`);

      if (!config.headers?.['Content-Type']) {
        parts.push('-H', '"Content-Type: application/json"');
      }
    }

    if (config.timeout) {
      parts.push('--max-time', config.timeout.toString());
    }

    if (config.followRedirects !== false) {
      parts.push('-L');
      if (config.maxRedirects) {
        parts.push('--max-redirs', config.maxRedirects.toString());
      }
    }

    if (config.proxy) {
      parts.push('-x', config.proxy);
    }

    // SSL/TLS configuration
    // insecure: true takes precedence (backwards compatibility)
    // ssl.verify: false is equivalent to insecure: true
    if (config.insecure || config.ssl?.verify === false) {
      parts.push('-k');
    }

    // SSL certificate options
    if (config.ssl) {
      if (config.ssl.ca) {
        parts.push('--cacert', `"${config.ssl.ca}"`);
      }
      if (config.ssl.cert) {
        parts.push('--cert', `"${config.ssl.cert}"`);
      }
      if (config.ssl.key) {
        parts.push('--key', `"${config.ssl.key}"`);
      }
    }

    if (config.output) {
      parts.push('-o', config.output);
    }

    parts.push('-s', '-S');

    let url = config.url;
    if (config.params && Object.keys(config.params).length > 0) {
      const queryString = new URLSearchParams(config.params).toString();
      url += (url.includes('?') ? '&' : '?') + queryString;
    }

    parts.push(`"${url}"`);

    return parts.join(' ');
  }

  static async executeCurl(command: string): Promise<{
    success: boolean;
    status?: number;
    headers?: Record<string, string>;
    body?: string;
    metrics?: CurlMetrics;
    error?: string;
  }> {
    try {
      const proc = Bun.spawn(['sh', '-c', command], {
        stdout: 'pipe',
        stderr: 'pipe',
      });

      const stdout = await new Response(proc.stdout).text();
      const stderr = await new Response(proc.stderr).text();

      await proc.exited;

      if (proc.exitCode !== 0 && !stdout) {
        return {
          success: false,
          error: stderr || `Command failed with exit code ${proc.exitCode}`,
        };
      }

      let responseBody = stdout;
      let metrics: CurlMetrics = {};

      const metricsMatch = stdout.match(/__CURL_METRICS_START__(.+?)__CURL_METRICS_END__/);
      if (metricsMatch) {
        responseBody = stdout.replace(/__CURL_METRICS_START__.+?__CURL_METRICS_END__/, '').trim();
        try {
          metrics = JSON.parse(metricsMatch[1]);
        } catch (_e) {}
      }

      const responseHeaders: Record<string, string> = {};
      if (metrics.response_code) {
        const headerLines = stderr.split('\n').filter((line) => line.includes(':'));
        for (const line of headerLines) {
          const [key, ...valueParts] = line.split(':');
          if (key && valueParts.length > 0) {
            responseHeaders[key.trim()] = valueParts.join(':').trim();
          }
        }
      }

      return {
        success: true,
        status: metrics.response_code || metrics.http_code,
        headers: responseHeaders,
        body: responseBody,
        metrics: {
          duration: (metrics.time_total || 0) * 1000,
          size: metrics.size_download,
          dnsLookup: (metrics.time_namelookup || 0) * 1000,
          tcpConnection: (metrics.time_connect || 0) * 1000,
          tlsHandshake: (metrics.time_appconnect || 0) * 1000,
          firstByte: (metrics.time_starttransfer || 0) * 1000,
          download: (metrics.time_total || 0) * 1000,
        },
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }
}
