/**
 * Response diffing/baseline types.
 */

import type { JsonValue } from './json';

/**
 * Configuration for response diffing at request level.
 */
export interface DiffConfig {
  /** Enable diffing for this request. */
  enabled?: boolean;
  /** Paths to exclude from comparison (e.g., 'body.timestamp'). */
  exclude?: string[];
  /** Match rules for dynamic values (path -> '*' or 'regex:pattern'). */
  match?: Record<string, string>;
  /** Include timing differences in comparison. Default: false */
  includeTimings?: boolean;
}

/**
 * Global configuration for response diffing.
 */
export interface GlobalDiffConfig extends DiffConfig {
  /** Directory for baseline files. Default: '__baselines__' */
  dir?: string;
  /** Label for current run (e.g., 'staging', 'production'). */
  label?: string;
  /** Label to compare against. */
  compareWith?: string;
  /** Save current run as baseline. */
  save?: boolean;
  /** Output format for diff results. Default: 'terminal' */
  outputFormat?: 'terminal' | 'json' | 'markdown';
}

/**
 * Stored baseline data for a single request.
 */
export interface Baseline {
  status?: number;
  headers?: Record<string, string>;
  body?: JsonValue;
  timing?: number;
  hash: string;
  capturedAt: string;
}

/**
 * Baseline file format.
 */
export interface BaselineFile {
  version: number;
  label: string;
  capturedAt: string;
  baselines: Record<string, Baseline>;
}

/**
 * Single difference in response comparison.
 */
export interface ResponseDiff {
  path: string;
  baseline: unknown;
  current: unknown;
  type: 'added' | 'removed' | 'changed' | 'type_mismatch';
}

/**
 * Result of comparing a response against a baseline.
 */
export interface DiffCompareResult {
  requestName: string;
  hasDifferences: boolean;
  isNewBaseline: boolean;
  baselineLabel: string;
  currentLabel: string;
  differences: ResponseDiff[];
  timingDiff?: {
    baseline: number;
    current: number;
    changePercent: number;
  };
}

/**
 * Summary of diff comparison across all requests.
 */
export interface DiffSummary {
  totalRequests: number;
  unchanged: number;
  changed: number;
  newBaselines: number;
  results: DiffCompareResult[];
}
