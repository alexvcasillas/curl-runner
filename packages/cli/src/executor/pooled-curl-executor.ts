import {
  buildCurlArgs,
  type CurlMetrics,
  createBatchMarker,
  extractBatchMetrics,
  formatArgsForDisplay,
  getStatusCode,
  isSuccessStatus,
  parseMetrics,
  parseResponseBody,
} from '../core/curl';
import type { ConnectionPoolConfig, ExecutionResult, RequestConfig } from '../types/config';

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
 * Uses shared argument building logic from core/curl.
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
   * Builds curl args for a single request within a batch.
   * Uses shared buildCurlArgs with batch-specific marker.
   */
  private buildRequestArgs(config: RequestConfig, requestIndex: number): string[] {
    const { args, url } = buildCurlArgs(config, {
      writeOutMarker: createBatchMarker(requestIndex),
      includeSilentFlags: false, // Added at batch level
      includeHttp2Flag: false, // Added at batch level
      includeOutputFlag: false, // Not used in batching
    });

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
      const { found, metricsJson, startIdx } = extractBatchMetrics(stdout, req.index);

      if (!found) {
        results.set(req.index, {
          request: req.config,
          success: false,
          error: stderr || `Request failed (exit code: ${exitCode})`,
          metrics: { duration: 0 },
        });
        continue;
      }

      let rawMetrics: CurlMetrics = {};
      try {
        rawMetrics = JSON.parse(metricsJson);
      } catch {
        // Ignore parse errors
      }

      // Extract body: everything before the marker for this request
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
      const statusCode = getStatusCode(rawMetrics);
      const metrics = parseMetrics(rawMetrics);

      results.set(req.index, {
        request: req.config,
        success: isSuccessStatus(statusCode),
        status: statusCode,
        body: parseResponseBody(body),
        metrics,
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

    const batchPromises = groups.map((group) => this.executeBatch(group));
    const batchResults = await Promise.all(batchPromises);

    for (const resultMap of batchResults) {
      for (const [index, result] of resultMap) {
        allResults.set(index, result);
      }
    }

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
    return formatArgsForDisplay(args);
  }
}
