/**
 * Retry execution logic with exponential backoff.
 */

import type { RetryConfig, RetryOptions, RetryResult } from './types';

/**
 * Calculates delay for a retry attempt with exponential backoff.
 */
export function calculateBackoffDelay(
  attempt: number,
  baseDelay: number,
  backoffMultiplier: number,
): number {
  // attempt is 1-indexed (1st retry, 2nd retry, etc.)
  return baseDelay * backoffMultiplier ** (attempt - 1);
}

/**
 * Executes an async operation with retry logic.
 *
 * @param operation - The async operation to execute
 * @param options - Retry options
 * @returns RetryResult with value, success status, and attempt count
 */
export async function withRetry<T>(
  operation: () => Promise<T>,
  options: RetryOptions<T> = {},
): Promise<RetryResult<T>> {
  const { config, shouldRetry, onRetry } = options;
  const maxAttempts = (config?.count ?? 0) + 1;
  const delay = config?.delay ?? 0;
  const backoff = config?.backoff ?? 1;

  let attempt = 0;
  let lastError: string | undefined;

  while (attempt < maxAttempts) {
    // Log retry attempt (not the first attempt)
    if (attempt > 0 && onRetry) {
      onRetry(attempt, config?.count ?? 0);
    }

    // Wait before retry (not the first attempt)
    if (attempt > 0 && delay > 0) {
      const waitTime = calculateBackoffDelay(attempt, delay, backoff);
      await Bun.sleep(waitTime);
    }

    attempt++;

    try {
      const result = await operation();

      // Check if we should retry based on result
      if (shouldRetry?.(result)) {
        lastError = 'Operation returned retriable result';
        continue;
      }

      return {
        value: result,
        success: true,
        attempts: attempt,
      };
    } catch (error) {
      lastError = error instanceof Error ? error.message : String(error);
    }
  }

  return {
    success: false,
    attempts: attempt,
    error: lastError,
  };
}

/**
 * Creates retry options from a retry config.
 */
export function createRetryOptions<T>(
  config?: RetryConfig,
  shouldRetry?: (result: T) => boolean,
  onRetry?: (attempt: number, maxAttempts: number) => void,
): RetryOptions<T> {
  return { config, shouldRetry, onRetry };
}
