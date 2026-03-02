/**
 * Types for retry configuration and execution.
 */

/** Default HTTP status codes that trigger a retry. */
export const DEFAULT_RETRYABLE_STATUSES = [429, 500, 502, 503, 504] as const;

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
  /** HTTP status codes that trigger a retry. Default: [429, 500, 502, 503, 504] */
  retryableStatuses?: number[];
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
  /** Per-attempt delay override in ms (e.g. from Retry-After header). Falls back to config.delay + backoff. */
  getDelay?: (attempt: number, result: T | undefined) => number | undefined;
}
