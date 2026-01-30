import type { FileAttachment, FormFieldValue, RequestConfig } from '../types/config';

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

// Using class for organization, but could be refactored to functions
export class CurlBuilder {
  static buildCommand(config: RequestConfig): string[] {
    const args: string[] = [];

    args.push('-X', config.method || 'GET');

    args.push('-w', '\n__CURL_METRICS_START__%{json}__CURL_METRICS_END__');

    if (config.headers) {
      for (const [key, value] of Object.entries(config.headers)) {
        args.push('-H', `${key}: ${value}`);
      }
    }

    if (config.auth) {
      if (config.auth.type === 'basic' && config.auth.username && config.auth.password) {
        args.push('-u', `${config.auth.username}:${config.auth.password}`);
      } else if (config.auth.type === 'bearer' && config.auth.token) {
        args.push('-H', `Authorization: Bearer ${config.auth.token}`);
      }
    }

    if (config.formData) {
      // Use -F for file attachments (need @ interpretation)
      // Use --form-string for text fields (prevents @ and < interpretation)
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

      if (!config.headers?.['Content-Type']) {
        args.push('-H', 'Content-Type: application/json');
      }
    }

    if (config.timeout) {
      args.push('--max-time', config.timeout.toString());
    }

    if (config.followRedirects !== false) {
      args.push('-L');
      if (config.maxRedirects) {
        args.push('--max-redirs', config.maxRedirects.toString());
      }
    }

    if (config.proxy) {
      args.push('-x', config.proxy);
    }

    // SSL/TLS configuration
    // insecure: true takes precedence (backwards compatibility)
    // ssl.verify: false is equivalent to insecure: true
    if (config.insecure || config.ssl?.verify === false) {
      args.push('-k');
    }

    // SSL certificate options
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

    if (config.output) {
      args.push('-o', config.output);
    }

    if (config.http2) {
      args.push('--http2');
    }

    args.push('-s', '-S');

    let url = config.url;
    if (config.params && Object.keys(config.params).length > 0) {
      const queryString = new URLSearchParams(config.params).toString();
      url += (url.includes('?') ? '&' : '?') + queryString;
    }

    args.push(url);

    return args;
  }

  /**
   * Formats args array as shell-safe command string for display/debugging.
   */
  static formatCommandForDisplay(args: string[]): string {
    return ['curl', ...args]
      .map((arg) =>
        arg.includes(' ') || arg.includes('"') || arg.includes("'")
          ? `'${arg.replace(/'/g, "'\\''")}'`
          : arg,
      )
      .join(' ');
  }

  static async executeCurl(args: string[]): Promise<{
    success: boolean;
    status?: number;
    headers?: Record<string, string>;
    body?: string;
    metrics?: {
      duration: number;
      size?: number;
      dnsLookup?: number;
      tcpConnection?: number;
      tlsHandshake?: number;
      firstByte?: number;
      download?: number;
    };
    error?: string;
  }> {
    try {
      const proc = Bun.spawn(['curl', ...args], {
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
