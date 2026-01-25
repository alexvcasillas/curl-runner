import type {
  ExecutionResult,
  GlobalConfig,
  ProfileConfig,
  ProfileResult,
  RequestConfig,
} from '../types/config';
import { CurlBuilder } from '../utils/curl-builder';
import { Logger } from '../utils/logger';
import { calculateProfileStats } from '../utils/stats';

export class ProfileExecutor {
  private logger: Logger;
  private profileConfig: ProfileConfig;

  constructor(globalConfig: GlobalConfig, profileConfig: ProfileConfig) {
    this.globalConfig = globalConfig;
    this.profileConfig = profileConfig;
    this.logger = new Logger(globalConfig.output);
  }

  /**
   * Execute a single iteration of the request (minimal overhead version).
   * Skips logging and validation for accurate timing.
   */
  private async executeSingleIteration(config: RequestConfig): Promise<ExecutionResult> {
    const startTime = performance.now();
    const command = CurlBuilder.buildCommand(config);
    const result = await CurlBuilder.executeCurl(command);

    if (result.success) {
      let body = result.body;
      try {
        if (
          result.headers?.['content-type']?.includes('application/json') ||
          (body && (body.trim().startsWith('{') || body.trim().startsWith('[')))
        ) {
          body = JSON.parse(body);
        }
      } catch (_e) {
        // Keep raw body
      }

      return {
        request: config,
        success: true,
        status: result.status,
        headers: result.headers,
        body,
        metrics: {
          ...result.metrics,
          duration: performance.now() - startTime,
        },
      };
    }

    return {
      request: config,
      success: false,
      error: result.error,
      metrics: {
        duration: performance.now() - startTime,
      },
    };
  }

  /**
   * Execute iterations in chunks for controlled concurrency.
   */
  private async executeWithConcurrency(
    config: RequestConfig,
    iterations: number,
    concurrency: number,
  ): Promise<ExecutionResult[]> {
    const results: ExecutionResult[] = [];

    for (let i = 0; i < iterations; i += concurrency) {
      const chunkSize = Math.min(concurrency, iterations - i);
      const chunk = await Promise.all(
        Array.from({ length: chunkSize }, () => this.executeSingleIteration(config)),
      );
      results.push(...chunk);
    }

    return results;
  }

  /**
   * Profile a single request by running it multiple times.
   */
  async profileRequest(config: RequestConfig, index = 0): Promise<ProfileResult> {
    const iterations = this.profileConfig.iterations;
    const warmup = this.profileConfig.warmup ?? 1;
    const concurrency = this.profileConfig.concurrency ?? 1;
    const requestName = config.name || `Request ${index + 1}`;

    this.logger.logProfileStart(requestName, iterations, warmup, concurrency);

    const results =
      concurrency === 1
        ? await this.executeSequentially(config, iterations)
        : await this.executeWithConcurrency(config, iterations, concurrency);

    // Collect timings and count failures
    const timings: number[] = [];
    let failures = 0;

    for (const result of results) {
      if (result.success && result.metrics?.duration !== undefined) {
        timings.push(result.metrics.duration);
      } else {
        failures++;
        // Use 0 as placeholder for failed requests (excluded from stats)
        timings.push(0);
      }
    }

    // Filter out failed timings (0s) for stats calculation
    const successfulTimings = timings
      .map((t, i) => (results[i].success ? t : -1))
      .filter((t) => t >= 0);

    // Recalculate stats with only successful timings
    const stats = calculateProfileStats(
      successfulTimings,
      Math.min(warmup, successfulTimings.length),
      failures,
    );

    return {
      request: config,
      stats,
      iterations: results,
    };
  }

  /**
   * Execute iterations sequentially (default behavior).
   */
  private async executeSequentially(
    config: RequestConfig,
    iterations: number,
  ): Promise<ExecutionResult[]> {
    const results: ExecutionResult[] = [];

    for (let i = 0; i < iterations; i++) {
      const result = await this.executeSingleIteration(config);
      results.push(result);
    }

    return results;
  }

  /**
   * Profile multiple requests.
   */
  async profileRequests(requests: RequestConfig[]): Promise<ProfileResult[]> {
    const results: ProfileResult[] = [];

    for (let i = 0; i < requests.length; i++) {
      const result = await this.profileRequest(requests[i], i);
      results.push(result);
      this.logger.logProfileResult(result, this.profileConfig.histogram ?? false);
    }

    return results;
  }
}
