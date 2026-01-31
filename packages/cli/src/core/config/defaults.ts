/**
 * Centralized default values for all configuration options.
 * These defaults are applied when no value is provided via CLI, env vars, or config file.
 */

export const CONFIG_DEFAULTS = {
  // Execution
  execution: 'sequential' as const,
  continueOnError: false,
  dryRun: false,
  http2: false,

  // Timeouts & Retries
  timeout: 30000,
  retryCount: 0,
  retryDelay: 1000,
  retryBackoff: 1,

  // Output
  verbose: false,
  outputFormat: 'pretty' as const,
  prettyLevel: 'standard' as const,
  showHeaders: false,
  showBody: true,
  showMetrics: false,

  // Watch
  watchDebounce: 300,
  watchClear: true,

  // Profile
  profileIterations: 10,
  profileWarmup: 1,
  profileConcurrency: 1,
  profileHistogram: false,

  // Snapshot
  snapshotDir: '__snapshots__',
  snapshotUpdateMode: 'none' as const,
  snapshotCi: false,

  // Diff/Baseline
  baselineDir: '__baselines__',
  diffOutputFormat: 'terminal' as const,

  // Connection Pool
  connectionPoolEnabled: false,
  maxStreamsPerHost: 10,
  keepaliveTime: 60,
  connectTimeout: 30,
} as const;

export type ConfigDefaults = typeof CONFIG_DEFAULTS;
