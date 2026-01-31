/**
 * Types for retry configuration and execution.
 */

/**
 * Retry configuration options.
 */
export interface RetryConfig {
  /** Number of retry attempts (0 = no retries) */
  count?: number;
  /** Delay between retries in milliseconds */
  delay?: number;
  /** Backoff multiplier for exponential backoff (default: 1 = no backoff) */
  backoff?: number;
}

/**
 * Result from a retry operation.
 */
export interface RetryResult<T> {
  /** The result value (if successful) */
  value?: T;
  /** Whether the operation succeeded */
  success: boolean;
  /** Number of attempts made */
  attempts: number;
  /** Last error message (if failed) */
  error?: string;
}

/**
 * Callback for logging retry attempts.
 */
export type RetryLogger = (attempt: number, maxAttempts: number) => void;

/**
 * Options for the retry function.
 */
export interface RetryOptions<T> {
  /** Retry configuration */
  config?: RetryConfig;
  /** Function to check if result should trigger retry (return true to retry) */
  shouldRetry?: (result: T) => boolean;
  /** Callback for logging retry attempts */
  onRetry?: RetryLogger;
}
