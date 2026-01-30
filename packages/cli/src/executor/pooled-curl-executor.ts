import type {
  ConnectionPoolConfig,
  ExecutionResult,
  FileAttachment,
  FormFieldValue,
  JsonValue,
  RequestConfig,
} from '../types/config';

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

interface BatchedRequest {
  index: number;
  config: RequestConfig;
}

interface HostGroup {
  host: string;
  requests: BatchedRequest[];
}

/**
 * Executes curl requests with TCP connection pooling and HTTP/2 multiplexing.
 * Groups requests by host and batches them into single curl processes.
 */
export class PooledCurlExecutor {
  private poolConfig: ConnectionPoolConfig;

  constructor(poolConfig: ConnectionPoolConfig = {}) {
    this.poolConfig = {
      enabled: true,
      maxStreamsPerHost: 10,
      keepaliveTime: 60,
      connectTimeout: 30,
      ...poolConfig,
    };
  }

  /**
   * Extracts host key (scheme://host:port) from URL for grouping.
   */
  private getHostKey(url: string): string {
    try {
      const parsed = new URL(url);
      return `${parsed.protocol}//${parsed.host}`;
    } catch {
      return url;
    }
  }

  /**
   * Groups requests by their host for batched execution.
   */
  groupRequestsByHost(requests: RequestConfig[]): HostGroup[] {
    const groups = new Map<string, BatchedRequest[]>();

    requests.forEach((config, index) => {
      const hostKey = this.getHostKey(config.url);
      const existing = groups.get(hostKey) || [];
      existing.push({ index, config });
      groups.set(hostKey, existing);
    });

    return Array.from(groups.entries()).map(([host, reqs]) => ({
      host,
      requests: reqs,
    }));
  }

  /**
   * Checks if a form field value is a file attachment.
   */
  private isFileAttachment(value: FormFieldValue): value is FileAttachment {
    return typeof value === 'object' && value !== null && 'file' in value;
  }

  /**
   * Builds curl args for a single request within a batch.
   * Does NOT include the URL - that's added separately.
   */
  private buildRequestArgs(config: RequestConfig, requestIndex: number): string[] {
    const args: string[] = [];

    args.push('-X', config.method || 'GET');

    // Unique marker per request for output parsing
    args.push(
      '-w',
      `\n__CURL_BATCH_${requestIndex}_START__%{json}__CURL_BATCH_${requestIndex}_END__\n`,
    );

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
      for (const [fieldName, fieldValue] of Object.entries(config.formData)) {
        if (this.isFileAttachment(fieldValue)) {
          let fileSpec = `@${fieldValue.file}`;
          if (fieldValue.filename) {
            fileSpec += `;filename=${fieldValue.filename}`;
          }
          if (fieldValue.contentType) {
            fileSpec += `;type=${fieldValue.contentType}`;
          }
          args.push('-F', `${fieldName}=${fileSpec}`);
        } else {
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

    // Build URL with params
    let url = config.url;
    if (config.params && Object.keys(config.params).length > 0) {
      const queryString = new URLSearchParams(config.params).toString();
      url += (url.includes('?') ? '&' : '?') + queryString;
    }

    args.push(url);

    return args;
  }

  /**
   * Builds a batched curl command for a host group.
   * Uses -Z for parallel execution and --next to separate requests.
   */
  buildBatchedCommand(group: HostGroup): string[] {
    const args: string[] = [];

    // Global flags for connection pooling
    args.push('-Z'); // Enable parallel transfers
    args.push('--parallel-max', this.poolConfig.maxStreamsPerHost!.toString());
    args.push('--http2'); // Enable HTTP/2 multiplexing
    args.push('--keepalive-time', this.poolConfig.keepaliveTime!.toString());
    args.push('--connect-timeout', this.poolConfig.connectTimeout!.toString());
    args.push('-s', '-S'); // Silent but show errors

    // Add each request with --next separator
    group.requests.forEach((req, i) => {
      if (i > 0) {
        args.push('--next');
      }
      const requestArgs = this.buildRequestArgs(req.config, req.index);
      args.push(...requestArgs);
    });

    return args;
  }

  /**
   * Parses batched curl output into individual results.
   */
  parseBatchedOutput(
    stdout: string,
    stderr: string,
    group: HostGroup,
    exitCode: number,
  ): Map<number, ExecutionResult> {
    const results = new Map<number, ExecutionResult>();

    for (const req of group.requests) {
      const markerStart = `__CURL_BATCH_${req.index}_START__`;
      const markerEnd = `__CURL_BATCH_${req.index}_END__`;

      const startIdx = stdout.indexOf(markerStart);
      const endIdx = stdout.indexOf(markerEnd);

      if (startIdx === -1 || endIdx === -1) {
        // Failed to find markers - request likely failed
        results.set(req.index, {
          request: req.config,
          success: false,
          error: stderr || `Request failed (exit code: ${exitCode})`,
          metrics: { duration: 0 },
        });
        continue;
      }

      const metricsJson = stdout.substring(startIdx + markerStart.length, endIdx);
      let metrics: CurlMetrics = {};

      try {
        metrics = JSON.parse(metricsJson);
      } catch {
        // Ignore parse errors
      }

      // Extract body: everything before the marker for this request
      // This is tricky with batched output - we need to find the body boundaries
      const bodyEnd = startIdx;
      let bodyStart = 0;

      // Find the end of the previous request's marker (if any)
      for (const otherReq of group.requests) {
        if (otherReq.index === req.index) {
          continue;
        }
        const otherEnd = `__CURL_BATCH_${otherReq.index}_END__`;
        const otherEndIdx = stdout.indexOf(otherEnd);
        if (
          otherEndIdx !== -1 &&
          otherEndIdx < bodyEnd &&
          otherEndIdx + otherEnd.length > bodyStart
        ) {
          bodyStart = otherEndIdx + otherEnd.length;
        }
      }

      const body: string | undefined = stdout.substring(bodyStart, bodyEnd).trim();

      // Try to parse as JSON
      let parsedBody: JsonValue = body;
      try {
        if (body && (body.startsWith('{') || body.startsWith('['))) {
          parsedBody = JSON.parse(body) as JsonValue;
        }
      } catch {
        // Keep as string
      }

      results.set(req.index, {
        request: req.config,
        success: (metrics.response_code || metrics.http_code || 0) < 400,
        status: metrics.response_code || metrics.http_code,
        body: parsedBody,
        metrics: {
          duration: (metrics.time_total || 0) * 1000,
          size: metrics.size_download,
          dnsLookup: (metrics.time_namelookup || 0) * 1000,
          tcpConnection: (metrics.time_connect || 0) * 1000,
          tlsHandshake: (metrics.time_appconnect || 0) * 1000,
          firstByte: (metrics.time_starttransfer || 0) * 1000,
          download: (metrics.time_total || 0) * 1000,
        },
      });
    }

    return results;
  }

  /**
   * Executes a batched curl command for a host group.
   */
  async executeBatch(group: HostGroup): Promise<Map<number, ExecutionResult>> {
    const args = this.buildBatchedCommand(group);

    try {
      const proc = Bun.spawn(['curl', ...args], {
        stdout: 'pipe',
        stderr: 'pipe',
      });

      const stdout = await new Response(proc.stdout).text();
      const stderr = await new Response(proc.stderr).text();
      await proc.exited;

      return this.parseBatchedOutput(stdout, stderr, group, proc.exitCode || 0);
    } catch (error) {
      // Return error for all requests in batch
      const results = new Map<number, ExecutionResult>();
      for (const req of group.requests) {
        results.set(req.index, {
          request: req.config,
          success: false,
          error: error instanceof Error ? error.message : String(error),
          metrics: { duration: 0 },
        });
      }
      return results;
    }
  }

  /**
   * Executes all requests with connection pooling.
   * Groups by host and executes each group as a batch.
   */
  async executeAll(requests: RequestConfig[]): Promise<ExecutionResult[]> {
    const groups = this.groupRequestsByHost(requests);
    const allResults = new Map<number, ExecutionResult>();

    // Execute all host groups in parallel
    const batchPromises = groups.map((group) => this.executeBatch(group));
    const batchResults = await Promise.all(batchPromises);

    // Merge results from all batches
    for (const resultMap of batchResults) {
      for (const [index, result] of resultMap) {
        allResults.set(index, result);
      }
    }

    // Return results in original order
    return requests.map((_, index) => {
      return (
        allResults.get(index) || {
          request: requests[index],
          success: false,
          error: 'Request not executed',
          metrics: { duration: 0 },
        }
      );
    });
  }

  /**
   * Formats the batched command for display/debugging.
   */
  formatBatchedCommandForDisplay(group: HostGroup): string {
    const args = this.buildBatchedCommand(group);
    return ['curl', ...args]
      .map((arg) =>
        arg.includes(' ') || arg.includes('"') || arg.includes("'")
          ? `'${arg.replace(/'/g, "'\\''")}'`
          : arg,
      )
      .join(' ');
  }
}
