/**
 * Performance profiling types.
 */

/**
 * Configuration for performance profiling mode.
 * Runs requests multiple times to collect latency statistics.
 */
export interface ProfileConfig {
  /** Number of iterations to run. Default: 10 */
  iterations: number;
  /** Number of warmup iterations to exclude from stats. Default: 1 */
  warmup?: number;
  /** Number of concurrent iterations. Default: 1 (sequential) */
  concurrency?: number;
  /** Show ASCII histogram in output. Default: false */
  histogram?: boolean;
  /** Export raw timings to file (JSON or CSV based on extension) */
  exportFile?: string;
}

/**
 * Statistics computed from profile run timings.
 */
export interface ProfileStats {
  /** Total iterations run (excluding warmup) */
  iterations: number;
  /** Warmup iterations excluded */
  warmup: number;
  /** Minimum latency in ms */
  min: number;
  /** Maximum latency in ms */
  max: number;
  /** Mean latency in ms */
  mean: number;
  /** Median latency in ms (same as p50) */
  median: number;
  /** 50th percentile latency in ms */
  p50: number;
  /** 95th percentile latency in ms */
  p95: number;
  /** 99th percentile latency in ms */
  p99: number;
  /** Standard deviation in ms */
  stdDev: number;
  /** Number of failed iterations */
  failures: number;
  /** Failure rate as percentage */
  failureRate: number;
  /** Raw timing values (for export) */
  timings: number[];
}
