import type {
  ExecutionResult,
  ExecutionSummary,
  GlobalConfig,
  RequestConfig,
} from '../types/config';
import { CurlBuilder } from '../utils/curl-builder';
import { Logger } from '../utils/logger';

export class RequestExecutor {
  private logger: Logger;
  private globalConfig: GlobalConfig;

  constructor(globalConfig: GlobalConfig = {}) {
    this.globalConfig = globalConfig;
    this.logger = new Logger(globalConfig.output);
  }

  async executeRequest(config: RequestConfig, index: number = 0): Promise<ExecutionResult> {
    const startTime = performance.now();

    this.logger.logRequestStart(config, index);

    const command = CurlBuilder.buildCommand(config);
    this.logger.logCommand(command);

    let attempt = 0;
    let lastError: string | undefined;
    const maxAttempts = (config.retry?.count || 0) + 1;

    while (attempt < maxAttempts) {
      if (attempt > 0) {
        this.logger.logRetry(attempt, maxAttempts - 1);
        if (config.retry?.delay) {
          await Bun.sleep(config.retry.delay);
        }
      }

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
        } catch (_e) {}

        const executionResult: ExecutionResult = {
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

        if (config.expect) {
          const validationResult = this.validateResponse(executionResult, config.expect);
          if (!validationResult.success) {
            executionResult.success = false;
            executionResult.error = validationResult.error;
          }
        }

        this.logger.logRequestComplete(executionResult);
        return executionResult;
      }

      lastError = result.error;
      attempt++;
    }

    const failedResult: ExecutionResult = {
      request: config,
      success: false,
      error: lastError,
      metrics: {
        duration: performance.now() - startTime,
      },
    };

    this.logger.logRequestComplete(failedResult);
    return failedResult;
  }

  private validateResponse(
    result: ExecutionResult,
    expect: RequestConfig['expect'],
  ): { success: boolean; error?: string } {
    if (!expect) {
      return { success: true };
    }

    const errors: string[] = [];

    if (expect.status !== undefined) {
      const expectedStatuses = Array.isArray(expect.status) ? expect.status : [expect.status];
      if (!expectedStatuses.includes(result.status || 0)) {
        errors.push(`Expected status ${expectedStatuses.join(' or ')}, got ${result.status}`);
      }
    }

    if (expect.headers) {
      for (const [key, value] of Object.entries(expect.headers)) {
        const actualValue = result.headers?.[key] || result.headers?.[key.toLowerCase()];
        if (actualValue !== value) {
          errors.push(`Expected header ${key}="${value}", got "${actualValue}"`);
        }
      }
    }

    if (expect.body !== undefined) {
      const actualBody = JSON.stringify(result.body);
      const expectedBody = JSON.stringify(expect.body);
      if (actualBody !== expectedBody) {
        errors.push(`Body mismatch`);
      }
    }

    return errors.length > 0 ? { success: false, error: errors.join('; ') } : { success: true };
  }

  async executeSequential(requests: RequestConfig[]): Promise<ExecutionSummary> {
    const startTime = performance.now();
    const results: ExecutionResult[] = [];

    for (let i = 0; i < requests.length; i++) {
      const result = await this.executeRequest(requests[i], i + 1);
      results.push(result);

      if (!result.success && !this.globalConfig.continueOnError) {
        this.logger.logError('Stopping execution due to error');
        break;
      }
    }

    return this.createSummary(results, performance.now() - startTime);
  }

  async executeParallel(requests: RequestConfig[]): Promise<ExecutionSummary> {
    const startTime = performance.now();

    const promises = requests.map((request, index) => this.executeRequest(request, index + 1));

    const results = await Promise.all(promises);

    return this.createSummary(results, performance.now() - startTime);
  }

  async execute(requests: RequestConfig[]): Promise<ExecutionSummary> {
    this.logger.logExecutionStart(requests.length, this.globalConfig.execution || 'sequential');

    const summary =
      this.globalConfig.execution === 'parallel'
        ? await this.executeParallel(requests)
        : await this.executeSequential(requests);

    this.logger.logSummary(summary);

    if (this.globalConfig.output?.saveToFile) {
      await this.saveSummaryToFile(summary);
    }

    return summary;
  }

  private createSummary(results: ExecutionResult[], duration: number): ExecutionSummary {
    const successful = results.filter((r) => r.success).length;
    const failed = results.filter((r) => !r.success).length;

    return {
      total: results.length,
      successful,
      failed,
      duration,
      results,
    };
  }

  private async saveSummaryToFile(summary: ExecutionSummary): Promise<void> {
    const file = this.globalConfig.output?.saveToFile;
    if (!file) {
      return;
    }

    const content = JSON.stringify(summary, null, 2);
    await Bun.write(file, content);
    this.logger.logInfo(`Results saved to ${file}`);
  }
}
