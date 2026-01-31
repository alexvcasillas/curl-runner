/**
 * Execution result types.
 */

import type { DiffCompareResult } from './diff';
import type { JsonValue } from './json';
import type { ProfileStats } from './profile';
import type { RequestConfig } from './request';
import type { SnapshotCompareResult } from './snapshot';

export interface ExecutionResult {
  request: RequestConfig;
  success: boolean;
  status?: number;
  headers?: Record<string, string>;
  body?: JsonValue;
  error?: string;
  /** True if this result is from dry-run mode (no actual request made) */
  dryRun?: boolean;
  metrics?: {
    duration: number;
    size?: number;
    dnsLookup?: number;
    tcpConnection?: number;
    tlsHandshake?: number;
    firstByte?: number;
    download?: number;
  };
  /** Snapshot comparison result (if snapshot testing enabled). */
  snapshotResult?: SnapshotCompareResult;
  /** Diff comparison result (if response diffing enabled). */
  diffResult?: DiffCompareResult;
  /** Whether this request was skipped due to a `when` condition. */
  skipped?: boolean;
  /** Reason the request was skipped (condition that failed). */
  skipReason?: string;
}

export interface ExecutionSummary {
  total: number;
  successful: number;
  failed: number;
  skipped: number;
  duration: number;
  results: ExecutionResult[];
}

/**
 * Context for storing response values between sequential requests.
 * Values are stored as strings and can be referenced using ${store.variableName} syntax.
 */
export type ResponseStoreContext = Record<string, string>;

/**
 * Result of a profiled request execution.
 */
export interface ProfileResult {
  request: RequestConfig;
  stats: ProfileStats;
  /** Individual results from each iteration */
  iterations: ExecutionResult[];
}
