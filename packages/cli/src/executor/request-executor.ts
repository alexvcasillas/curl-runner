import { YamlParser } from '../parser/yaml';
import type {
  ExecutionResult,
  ExecutionSummary,
  GlobalConfig,
  JsonValue,
  RequestConfig,
  ResponseStoreContext,
} from '../types/config';
import { CurlBuilder } from '../utils/curl-builder';
import { Logger } from '../utils/logger';
import { createStoreContext, extractStoreValues } from '../utils/response-store';

export class RequestExecutor {
  private logger: Logger;
  private globalConfig: GlobalConfig;

  constructor(globalConfig: GlobalConfig = {}) {
    this.globalConfig = globalConfig;
    this.logger = new Logger(globalConfig.output);
  }

  private mergeOutputConfig(config: RequestConfig): GlobalConfig['output'] {
    // Precedence: Individual YAML file > curl-runner.yaml > CLI options > env vars > defaults
    return {
      ...this.globalConfig.output, // CLI options, env vars, and defaults (lowest priority)
      ...config.sourceOutputConfig, // Individual file's output config (highest priority)
    };
  }

  async executeRequest(config: RequestConfig, index: number = 0): Promise<ExecutionResult> {
    const startTime = performance.now();

    // Create per-request logger with merged output configuration
    const outputConfig = this.mergeOutputConfig(config);
    const requestLogger = new Logger(outputConfig);

    requestLogger.logRequestStart(config, index);

    const command = CurlBuilder.buildCommand(config);
    requestLogger.logCommand(command);

    let attempt = 0;
    let lastError: string | undefined;
    const maxAttempts = (config.retry?.count || 0) + 1;

    while (attempt < maxAttempts) {
      if (attempt > 0) {
        requestLogger.logRetry(attempt, maxAttempts - 1);
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

        requestLogger.logRequestComplete(executionResult);
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

    requestLogger.logRequestComplete(failedResult);
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

    // Validate status
    if (expect.status !== undefined) {
      const expectedStatuses = Array.isArray(expect.status) ? expect.status : [expect.status];
      if (!expectedStatuses.includes(result.status || 0)) {
        errors.push(`Expected status ${expectedStatuses.join(' or ')}, got ${result.status}`);
      }
    }

    // Validate headers
    if (expect.headers) {
      for (const [key, value] of Object.entries(expect.headers)) {
        const actualValue = result.headers?.[key] || result.headers?.[key.toLowerCase()];
        if (actualValue !== value) {
          errors.push(`Expected header ${key}="${value}", got "${actualValue}"`);
        }
      }
    }

    // Validate body
    if (expect.body !== undefined) {
      const bodyErrors = this.validateBodyProperties(result.body, expect.body, '');
      if (bodyErrors.length > 0) {
        errors.push(...bodyErrors);
      }
    }

    // Validate response time
    if (expect.responseTime !== undefined && result.metrics) {
      const responseTimeMs = result.metrics.duration;
      if (!this.validateRangePattern(responseTimeMs, expect.responseTime)) {
        errors.push(
          `Expected response time to match ${expect.responseTime}ms, got ${responseTimeMs.toFixed(2)}ms`,
        );
      }
    }

    const hasValidationErrors = errors.length > 0;

    // Handle failure expectation logic
    if (expect.failure === true) {
      // We expect this request to fail (negative testing)
      // Success means: validations pass AND status indicates error (4xx/5xx)
      if (hasValidationErrors) {
        return { success: false, error: errors.join('; ') };
      }

      // Check if status indicates an error
      const status = result.status || 0;
      if (status >= 400) {
        // Status indicates error and validations passed - SUCCESS for negative testing
        return { success: true };
      } else {
        // Expected failure but got success status - FAILURE
        return {
          success: false,
          error: `Expected request to fail (4xx/5xx) but got status ${status}`,
        };
      }
    } else {
      // Normal case: expect success (validations should pass)
      if (hasValidationErrors) {
        return { success: false, error: errors.join('; ') };
      } else {
        return { success: true };
      }
    }
  }

  private validateBodyProperties(
    actualBody: JsonValue,
    expectedBody: JsonValue,
    path: string,
  ): string[] {
    const errors: string[] = [];

    if (typeof expectedBody !== 'object' || expectedBody === null) {
      // Advanced value validation
      const validationResult = this.validateValue(actualBody, expectedBody, path || 'body');
      if (!validationResult.isValid) {
        errors.push(validationResult.error!);
      }
      return errors;
    }

    // Array validation
    if (Array.isArray(expectedBody)) {
      const validationResult = this.validateValue(actualBody, expectedBody, path || 'body');
      if (!validationResult.isValid) {
        errors.push(validationResult.error!);
      }
      return errors;
    }

    // Object property comparison with array selector support
    for (const [key, expectedValue] of Object.entries(expectedBody)) {
      const currentPath = path ? `${path}.${key}` : key;
      let actualValue: JsonValue;

      // Handle array selectors like [0], [-1], *, slice(0,3)
      if (Array.isArray(actualBody) && this.isArraySelector(key)) {
        actualValue = this.getArrayValue(actualBody, key);
      } else {
        actualValue = actualBody?.[key];
      }

      if (
        typeof expectedValue === 'object' &&
        expectedValue !== null &&
        !Array.isArray(expectedValue)
      ) {
        // Recursive validation for nested objects
        const nestedErrors = this.validateBodyProperties(actualValue, expectedValue, currentPath);
        errors.push(...nestedErrors);
      } else {
        // Advanced value validation
        const validationResult = this.validateValue(actualValue, expectedValue, currentPath);
        if (!validationResult.isValid) {
          errors.push(validationResult.error!);
        }
      }
    }

    return errors;
  }

  private validateValue(
    actualValue: JsonValue,
    expectedValue: JsonValue,
    path: string,
  ): { isValid: boolean; error?: string } {
    // Wildcard validation - accept any value
    if (expectedValue === '*') {
      return { isValid: true };
    }

    // Multiple acceptable values (array)
    if (Array.isArray(expectedValue)) {
      const isMatch = expectedValue.some((expected) => {
        if (expected === '*') {
          return true;
        }
        if (typeof expected === 'string' && this.isRegexPattern(expected)) {
          return this.validateRegexPattern(actualValue, expected);
        }
        if (typeof expected === 'string' && this.isRangePattern(expected)) {
          return this.validateRangePattern(actualValue, expected);
        }
        return actualValue === expected;
      });

      if (!isMatch) {
        return {
          isValid: false,
          error: `Expected ${path} to match one of ${JSON.stringify(expectedValue)}, got ${JSON.stringify(actualValue)}`,
        };
      }
      return { isValid: true };
    }

    // Regex pattern validation
    if (typeof expectedValue === 'string' && this.isRegexPattern(expectedValue)) {
      if (!this.validateRegexPattern(actualValue, expectedValue)) {
        return {
          isValid: false,
          error: `Expected ${path} to match pattern ${expectedValue}, got ${JSON.stringify(actualValue)}`,
        };
      }
      return { isValid: true };
    }

    // Numeric range validation
    if (typeof expectedValue === 'string' && this.isRangePattern(expectedValue)) {
      if (!this.validateRangePattern(actualValue, expectedValue)) {
        return {
          isValid: false,
          error: `Expected ${path} to match range ${expectedValue}, got ${JSON.stringify(actualValue)}`,
        };
      }
      return { isValid: true };
    }

    // Null handling
    if (expectedValue === 'null' || expectedValue === null) {
      if (actualValue !== null) {
        return {
          isValid: false,
          error: `Expected ${path} to be null, got ${JSON.stringify(actualValue)}`,
        };
      }
      return { isValid: true };
    }

    // Exact value comparison
    if (actualValue !== expectedValue) {
      return {
        isValid: false,
        error: `Expected ${path} to be ${JSON.stringify(expectedValue)}, got ${JSON.stringify(actualValue)}`,
      };
    }

    return { isValid: true };
  }

  private isRegexPattern(pattern: string): boolean {
    return (
      pattern.startsWith('^') ||
      pattern.endsWith('$') ||
      pattern.includes('\\d') ||
      pattern.includes('\\w') ||
      pattern.includes('\\s') ||
      pattern.includes('[') ||
      pattern.includes('*') ||
      pattern.includes('+') ||
      pattern.includes('?')
    );
  }

  private validateRegexPattern(actualValue: JsonValue, pattern: string): boolean {
    // Convert value to string for regex matching
    const stringValue = String(actualValue);
    try {
      const regex = new RegExp(pattern);
      return regex.test(stringValue);
    } catch {
      return false;
    }
  }

  private isRangePattern(pattern: string): boolean {
    // Only match explicit comparison operators, not simple number-dash-number patterns
    // This prevents matching things like zip codes "92998-3874" as ranges
    return /^(>=?|<=?|>|<)\s*[\d.-]+(\s*,\s*(>=?|<=?|>|<)\s*[\d.-]+)*$/.test(pattern);
  }

  private validateRangePattern(actualValue: JsonValue, pattern: string): boolean {
    const numValue = Number(actualValue);
    if (Number.isNaN(numValue)) {
      return false;
    }

    // Handle range like "10 - 50" or "10-50"
    const rangeMatch = pattern.match(/^([\d.-]+)\s*-\s*([\d.-]+)$/);
    if (rangeMatch) {
      const min = Number(rangeMatch[1]);
      const max = Number(rangeMatch[2]);
      return numValue >= min && numValue <= max;
    }

    // Handle comma-separated conditions like ">= 0, <= 100"
    const conditions = pattern.split(',').map((c) => c.trim());
    return conditions.every((condition) => {
      const match = condition.match(/^(>=?|<=?|>|<)\s*([\d.-]+)$/);
      if (!match) {
        return false;
      }

      const operator = match[1];
      const targetValue = Number(match[2]);

      switch (operator) {
        case '>':
          return numValue > targetValue;
        case '>=':
          return numValue >= targetValue;
        case '<':
          return numValue < targetValue;
        case '<=':
          return numValue <= targetValue;
        default:
          return false;
      }
    });
  }

  private isArraySelector(key: string): boolean {
    return /^\[.*\]$/.test(key) || key === '*' || key.startsWith('slice(');
  }

  private getArrayValue(array: JsonValue[], selector: string): JsonValue {
    if (selector === '*') {
      return array; // Return entire array for * validation
    }

    if (selector.startsWith('[') && selector.endsWith(']')) {
      const index = selector.slice(1, -1);
      if (index === '*') {
        return array;
      }

      const numIndex = Number(index);
      if (!Number.isNaN(numIndex)) {
        return numIndex >= 0 ? array[numIndex] : array[array.length + numIndex];
      }
    }

    if (selector.startsWith('slice(')) {
      const match = selector.match(/slice\((\d+)(?:,(\d+))?\)/);
      if (match) {
        const start = Number(match[1]);
        const end = match[2] ? Number(match[2]) : undefined;
        return array.slice(start, end);
      }
    }

    return undefined;
  }

  async executeSequential(requests: RequestConfig[]): Promise<ExecutionSummary> {
    const startTime = performance.now();
    const results: ExecutionResult[] = [];
    const storeContext = createStoreContext();

    for (let i = 0; i < requests.length; i++) {
      // Interpolate store variables before execution
      const interpolatedRequest = this.interpolateStoreVariables(requests[i], storeContext);
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
