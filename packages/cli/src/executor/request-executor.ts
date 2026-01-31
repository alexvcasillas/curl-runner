import type { CurlExecutionResult } from '../core/curl';
import { withRetry } from '../core/execution';
import { validateResponse } from '../core/validation';
import { YamlParser } from '../parser/yaml';
import { SnapshotManager } from '../snapshot/snapshot-manager';
import type {
  ExecutionResult,
  ExecutionSummary,
  FileAttachment,
  FormFieldValue,
  GlobalConfig,
  RequestConfig,
  ResponseStoreContext,
  SnapshotConfig,
} from '../types/config';
import { evaluateCondition } from '../utils/condition-evaluator';
import { CurlBuilder } from '../utils/curl-builder';
import { Logger } from '../utils/logger';
import { createStoreContext, extractStoreValues } from '../utils/response-store';
import { PooledCurlExecutor } from './pooled-curl-executor';

export class RequestExecutor {
  private logger: Logger;
  private globalConfig: GlobalConfig;
  private snapshotManager: SnapshotManager;
  private pooledExecutor: PooledCurlExecutor | null = null;

  constructor(globalConfig: GlobalConfig = {}) {
    this.globalConfig = globalConfig;
    this.logger = new Logger(globalConfig.output);
    this.snapshotManager = new SnapshotManager(globalConfig.snapshot);

    // Initialize pooled executor if connection pooling is enabled
    if (globalConfig.connectionPool?.enabled) {
      this.pooledExecutor = new PooledCurlExecutor(globalConfig.connectionPool);
    }
  }

  private mergeOutputConfig(config: RequestConfig): GlobalConfig['output'] {
    // Precedence: Individual YAML file > curl-runner.yaml > CLI options > env vars > defaults
    return {
      ...this.globalConfig.output, // CLI options, env vars, and defaults (lowest priority)
      ...config.sourceOutputConfig, // Individual file's output config (highest priority)
    };
  }

  /**
   * Gets the effective snapshot config for a request.
   */
  private getSnapshotConfig(config: RequestConfig): SnapshotConfig | null {
    return SnapshotManager.mergeConfig(this.globalConfig.snapshot, config.snapshot);
  }

  /**
   * Checks if a form field value is a file attachment.
   */
  private isFileAttachment(value: FormFieldValue): value is FileAttachment {
    return typeof value === 'object' && value !== null && 'file' in value;
  }

  /**
   * Validates that all file attachments in formData exist.
   * Returns an error message if any file is missing, or undefined if all files exist.
   */
  private async validateFileAttachments(config: RequestConfig): Promise<string | undefined> {
    if (!config.formData) {
      return undefined;
    }

    const missingFiles: string[] = [];

    for (const [fieldName, fieldValue] of Object.entries(config.formData)) {
      if (this.isFileAttachment(fieldValue)) {
        const filePath = fieldValue.file;
        const file = Bun.file(filePath);
        const exists = await file.exists();
        if (!exists) {
          missingFiles.push(`${fieldName}: ${filePath}`);
        }
      }
    }

    if (missingFiles.length > 0) {
      return `File(s) not found: ${missingFiles.join(', ')}`;
    }

    return undefined;
  }

  async executeRequest(config: RequestConfig, index: number = 0): Promise<ExecutionResult> {
    const startTime = performance.now();

    // Create per-request logger with merged output configuration
    const outputConfig = this.mergeOutputConfig(config);
    const requestLogger = new Logger(outputConfig);

    requestLogger.logRequestStart(config, index);

    // Validate file attachments exist before executing
    const fileError = await this.validateFileAttachments(config);
    if (fileError) {
      const failedResult: ExecutionResult = {
        request: config,
        success: false,
        error: fileError,
        metrics: {
          duration: performance.now() - startTime,
        },
      };
      requestLogger.logRequestComplete(failedResult);
      return failedResult;
    }

    const args = CurlBuilder.buildCommand(config);
    requestLogger.logCommand(CurlBuilder.formatCommandForDisplay(args));

    // Dry run mode: show command and return mock result
    if (this.globalConfig.dryRun) {
      const dryRunResult: ExecutionResult = {
        request: config,
        success: true,
        dryRun: true,
        metrics: {
          duration: 0,
        },
      };
      requestLogger.logRequestComplete(dryRunResult);
      return dryRunResult;
    }

    // Execute curl with retry logic
    const retryResult = await withRetry<CurlExecutionResult>(() => CurlBuilder.executeCurl(args), {
      config: config.retry,
      shouldRetry: (result) => !result.success,
      onRetry: (attempt, max) => requestLogger.logRetry(attempt, max),
    });

    // Handle curl execution failure (all retries exhausted)
    if (!retryResult.success || !retryResult.value?.success) {
      const failedResult: ExecutionResult = {
        request: config,
        success: false,
        error: retryResult.error || retryResult.value?.error,
        metrics: {
          duration: performance.now() - startTime,
        },
      };
      requestLogger.logRequestComplete(failedResult);
      return failedResult;
    }

    // Process successful curl result
    const curlResult = retryResult.value;
    const executionResult = this.processCurlResult(config, curlResult, startTime);

    // Validate response
    if (config.expect) {
      const validationResult = validateResponse(executionResult, config.expect);
      if (!validationResult.success) {
        executionResult.success = false;
        executionResult.error = validationResult.error;
      }
    }

    // Snapshot testing
    const snapshotConfig = this.getSnapshotConfig(config);
    if (snapshotConfig && config.sourceFile) {
      const snapshotResult = await this.snapshotManager.compareAndUpdate(
        config.sourceFile,
        config.name || 'Request',
        executionResult,
        snapshotConfig,
      );
      executionResult.snapshotResult = snapshotResult;

      if (!snapshotResult.match && !snapshotResult.updated) {
        executionResult.success = false;
        if (!executionResult.error) {
          executionResult.error = 'Snapshot mismatch';
        }
      }
    }

    requestLogger.logRequestComplete(executionResult);
    return executionResult;
  }

  /**
   * Processes a successful curl result into an execution result.
   */
  private processCurlResult(
    config: RequestConfig,
    curlResult: CurlExecutionResult,
    startTime: number,
  ): ExecutionResult {
    let body = curlResult.body;

    // Auto-parse JSON response
    try {
      if (
        curlResult.headers?.['content-type']?.includes('application/json') ||
        (body && (body.trim().startsWith('{') || body.trim().startsWith('[')))
      ) {
        body = JSON.parse(body);
      }
    } catch {
      // Keep body as string if JSON parse fails
    }

    return {
      request: config,
      success: true,
      status: curlResult.status,
      headers: curlResult.headers,
      body,
      metrics: {
        ...curlResult.metrics,
        duration: performance.now() - startTime,
      },
    };
  }

  async executeSequential(requests: RequestConfig[]): Promise<ExecutionSummary> {
    const startTime = performance.now();
    const results: ExecutionResult[] = [];
    const storeContext = createStoreContext();

    for (let i = 0; i < requests.length; i++) {
      // Interpolate store variables before execution
      const interpolatedRequest = this.interpolateStoreVariables(requests[i], storeContext);

      // Evaluate `when` condition if present
      if (interpolatedRequest.when) {
        const conditionResult = evaluateCondition(interpolatedRequest.when, storeContext);
        if (!conditionResult.shouldRun) {
          const skippedResult: ExecutionResult = {
            request: interpolatedRequest,
            success: true, // Skipped requests are not failures
            skipped: true,
            skipReason: conditionResult.reason,
          };
          results.push(skippedResult);
          this.logger.logSkipped(interpolatedRequest, i + 1, conditionResult.reason);
          continue;
        }
      }

      const result = await this.executeRequest(interpolatedRequest, i + 1);
      results.push(result);

      // Store values from successful responses
      if (result.success && interpolatedRequest.store) {
        const storedValues = extractStoreValues(result, interpolatedRequest.store);
        Object.assign(storeContext, storedValues);
        this.logStoredValues(storedValues);
      }

      if (!result.success && !this.globalConfig.continueOnError) {
        this.logger.logError('Stopping execution due to error');
        break;
      }
    }

    return this.createSummary(results, performance.now() - startTime);
  }

  /**
   * Interpolates store variables (${store.variableName}) in a request config.
   * This is called at execution time to resolve values from previous responses.
   */
  private interpolateStoreVariables(
    request: RequestConfig,
    storeContext: ResponseStoreContext,
  ): RequestConfig {
    // Only interpolate if there are stored values
    if (Object.keys(storeContext).length === 0) {
      return request;
    }

    // Re-interpolate the request with store context
    // We pass empty variables since static variables were already resolved
    return YamlParser.interpolateVariables(request, {}, storeContext) as RequestConfig;
  }

  /**
   * Logs stored values for debugging purposes.
   */
  private logStoredValues(values: ResponseStoreContext): void {
    if (Object.keys(values).length === 0) {
      return;
    }

    const entries = Object.entries(values);
    for (const [key, value] of entries) {
      const displayValue = value.length > 50 ? `${value.substring(0, 50)}...` : value;
      this.logger.logInfo(`Stored: ${key} = "${displayValue}"`);
    }
  }

  async executeParallel(requests: RequestConfig[]): Promise<ExecutionSummary> {
    const startTime = performance.now();

    // Use pooled execution if enabled (HTTP/2 multiplexing + connection reuse)
    if (this.pooledExecutor && !this.globalConfig.dryRun) {
      return this.executePooled(requests, startTime);
    }

    const maxConcurrency = this.globalConfig.maxConcurrency;

    // If no concurrency limit, execute all requests simultaneously
    if (!maxConcurrency || maxConcurrency >= requests.length) {
      const promises = requests.map((request, index) => this.executeRequest(request, index + 1));
      const results = await Promise.all(promises);
      return this.createSummary(results, performance.now() - startTime);
    }

    // Execute in chunks with limited concurrency
    const results: ExecutionResult[] = [];
    for (let i = 0; i < requests.length; i += maxConcurrency) {
      const chunk = requests.slice(i, i + maxConcurrency);
      const chunkPromises = chunk.map((request, chunkIndex) =>
        this.executeRequest(request, i + chunkIndex + 1),
      );
      const chunkResults = await Promise.all(chunkPromises);
      results.push(...chunkResults);

      // Check if we should stop on error
      const hasError = chunkResults.some((r) => !r.success);
      if (hasError && !this.globalConfig.continueOnError) {
        this.logger.logError('Stopping execution due to error');
        break;
      }
    }

    return this.createSummary(results, performance.now() - startTime);
  }

  /**
   * Executes requests using connection pooling with HTTP/2 multiplexing.
   * Groups requests by host and batches them into single curl processes.
   */
  private async executePooled(
    requests: RequestConfig[],
    startTime: number,
  ): Promise<ExecutionSummary> {
    this.logger.logInfo(
      `Using connection pooling (HTTP/2 multiplexing) for ${requests.length} requests`,
    );

    // Log host groups for visibility
    const groups = this.pooledExecutor!.groupRequestsByHost(requests);
    for (const group of groups) {
      this.logger.logInfo(`  ${group.host}: ${group.requests.length} request(s) batched`);
    }

    // Execute all batches
    const rawResults = await this.pooledExecutor!.executeAll(requests);

    // Post-process results: validation, snapshots, logging
    const results: ExecutionResult[] = [];
    for (let i = 0; i < rawResults.length; i++) {
      const result = rawResults[i];
      const config = requests[i];

      // Create per-request logger with merged output configuration
      const outputConfig = this.mergeOutputConfig(config);
      const requestLogger = new Logger(outputConfig);

      requestLogger.logRequestStart(config, i + 1);

      // Apply validation if configured
      if (result.success && config.expect) {
        const validationResult = validateResponse(result, config.expect);
        if (!validationResult.success) {
          result.success = false;
          result.error = validationResult.error;
        }
      }

      // Snapshot testing
      const snapshotConfig = this.getSnapshotConfig(config);
      if (snapshotConfig && config.sourceFile && result.success) {
        const snapshotResult = await this.snapshotManager.compareAndUpdate(
          config.sourceFile,
          config.name || 'Request',
          result,
          snapshotConfig,
        );
        result.snapshotResult = snapshotResult;

        if (!snapshotResult.match && !snapshotResult.updated) {
          result.success = false;
          if (!result.error) {
            result.error = 'Snapshot mismatch';
          }
        }
      }

      requestLogger.logRequestComplete(result);
      results.push(result);

      // Check if we should stop on error
      if (!result.success && !this.globalConfig.continueOnError) {
        this.logger.logError('Stopping execution due to error');
        break;
      }
    }

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
    const skipped = results.filter((r) => r.skipped).length;
    const successful = results.filter((r) => r.success && !r.skipped).length;
    const failed = results.filter((r) => !r.success).length;

    return {
      total: results.length,
      successful,
      failed,
      skipped,
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
