/**
 * Environment variable loader for curl-runner configuration.
 * All CURL_RUNNER_* environment variables are processed here.
 */

import type { GlobalConfig } from '../../types/config';

/**
 * Loads configuration from environment variables.
 * Environment variables are prefixed with CURL_RUNNER_.
 */
export function loadEnvironmentConfig(): Partial<GlobalConfig> {
  const config: Partial<GlobalConfig> = {};

  // Execution
  loadExecutionEnvVars(config);

  // Connection Pool
  loadConnectionPoolEnvVars(config);

  // Output
  loadOutputEnvVars(config);

  // CI Exit Codes
  loadCIEnvVars(config);

  // Watch Mode
  loadWatchEnvVars(config);

  // Profile Mode
  loadProfileEnvVars(config);

  // Snapshot
  loadSnapshotEnvVars(config);

  // Diff
  loadDiffEnvVars(config);

  // Timeouts & Retries
  loadRetryEnvVars(config);

  return config;
}

function loadExecutionEnvVars(config: Partial<GlobalConfig>): void {
  if (process.env.CURL_RUNNER_EXECUTION) {
    config.execution = process.env.CURL_RUNNER_EXECUTION as 'sequential' | 'parallel';
  }

  if (process.env.CURL_RUNNER_CONTINUE_ON_ERROR) {
    config.continueOnError = process.env.CURL_RUNNER_CONTINUE_ON_ERROR.toLowerCase() === 'true';
  }

  if (process.env.CURL_RUNNER_DRY_RUN) {
    config.dryRun = process.env.CURL_RUNNER_DRY_RUN.toLowerCase() === 'true';
  }

  if (process.env.CURL_RUNNER_HTTP2) {
    config.http2 = process.env.CURL_RUNNER_HTTP2.toLowerCase() === 'true';
  }

  if (process.env.CURL_RUNNER_MAX_CONCURRENCY) {
    const maxConcurrency = Number.parseInt(process.env.CURL_RUNNER_MAX_CONCURRENCY, 10);
    if (maxConcurrency > 0) {
      config.maxConcurrency = maxConcurrency;
    }
  }
}

function loadConnectionPoolEnvVars(config: Partial<GlobalConfig>): void {
  if (process.env.CURL_RUNNER_CONNECTION_POOL) {
    config.connectionPool = {
      ...config.connectionPool,
      enabled: process.env.CURL_RUNNER_CONNECTION_POOL.toLowerCase() === 'true',
    };
  }

  if (process.env.CURL_RUNNER_MAX_STREAMS_PER_HOST) {
    config.connectionPool = {
      ...config.connectionPool,
      maxStreamsPerHost: Number.parseInt(process.env.CURL_RUNNER_MAX_STREAMS_PER_HOST, 10),
    };
  }

  if (process.env.CURL_RUNNER_KEEPALIVE_TIME) {
    config.connectionPool = {
      ...config.connectionPool,
      keepaliveTime: Number.parseInt(process.env.CURL_RUNNER_KEEPALIVE_TIME, 10),
    };
  }

  if (process.env.CURL_RUNNER_CONNECT_TIMEOUT) {
    config.connectionPool = {
      ...config.connectionPool,
      connectTimeout: Number.parseInt(process.env.CURL_RUNNER_CONNECT_TIMEOUT, 10),
    };
  }
}

function loadOutputEnvVars(config: Partial<GlobalConfig>): void {
  if (process.env.CURL_RUNNER_VERBOSE) {
    config.output = {
      ...config.output,
      verbose: process.env.CURL_RUNNER_VERBOSE.toLowerCase() === 'true',
    };
  }

  if (process.env.CURL_RUNNER_OUTPUT_FORMAT) {
    const format = process.env.CURL_RUNNER_OUTPUT_FORMAT;
    if (['json', 'pretty', 'raw'].includes(format)) {
      config.output = { ...config.output, format: format as 'json' | 'pretty' | 'raw' };
    }
  }

  if (process.env.CURL_RUNNER_PRETTY_LEVEL) {
    const level = process.env.CURL_RUNNER_PRETTY_LEVEL;
    if (['minimal', 'standard', 'detailed'].includes(level)) {
      config.output = {
        ...config.output,
        prettyLevel: level as 'minimal' | 'standard' | 'detailed',
      };
    }
  }

  if (process.env.CURL_RUNNER_OUTPUT_FILE) {
    config.output = { ...config.output, saveToFile: process.env.CURL_RUNNER_OUTPUT_FILE };
  }
}

function loadCIEnvVars(config: Partial<GlobalConfig>): void {
  if (process.env.CURL_RUNNER_STRICT_EXIT) {
    config.ci = {
      ...config.ci,
      strictExit: process.env.CURL_RUNNER_STRICT_EXIT.toLowerCase() === 'true',
    };
  }

  if (process.env.CURL_RUNNER_FAIL_ON) {
    config.ci = {
      ...config.ci,
      failOn: Number.parseInt(process.env.CURL_RUNNER_FAIL_ON, 10),
    };
  }

  if (process.env.CURL_RUNNER_FAIL_ON_PERCENTAGE) {
    const percentage = Number.parseFloat(process.env.CURL_RUNNER_FAIL_ON_PERCENTAGE);
    if (percentage >= 0 && percentage <= 100) {
      config.ci = {
        ...config.ci,
        failOnPercentage: percentage,
      };
    }
  }
}

function loadWatchEnvVars(config: Partial<GlobalConfig>): void {
  if (process.env.CURL_RUNNER_WATCH) {
    config.watch = {
      ...config.watch,
      enabled: process.env.CURL_RUNNER_WATCH.toLowerCase() === 'true',
    };
  }

  if (process.env.CURL_RUNNER_WATCH_DEBOUNCE) {
    config.watch = {
      ...config.watch,
      debounce: Number.parseInt(process.env.CURL_RUNNER_WATCH_DEBOUNCE, 10),
    };
  }

  if (process.env.CURL_RUNNER_WATCH_CLEAR) {
    config.watch = {
      ...config.watch,
      clear: process.env.CURL_RUNNER_WATCH_CLEAR.toLowerCase() !== 'false',
    };
  }
}

function loadProfileEnvVars(config: Partial<GlobalConfig>): void {
  if (process.env.CURL_RUNNER_PROFILE) {
    const iterations = Number.parseInt(process.env.CURL_RUNNER_PROFILE, 10);
    if (iterations > 0) {
      config.profile = {
        ...config.profile,
        iterations,
      };
    }
  }

  if (process.env.CURL_RUNNER_PROFILE_WARMUP) {
    config.profile = {
      ...config.profile,
      iterations: config.profile?.iterations ?? 10,
      warmup: Number.parseInt(process.env.CURL_RUNNER_PROFILE_WARMUP, 10),
    };
  }

  if (process.env.CURL_RUNNER_PROFILE_CONCURRENCY) {
    config.profile = {
      ...config.profile,
      iterations: config.profile?.iterations ?? 10,
      concurrency: Number.parseInt(process.env.CURL_RUNNER_PROFILE_CONCURRENCY, 10),
    };
  }

  if (process.env.CURL_RUNNER_PROFILE_HISTOGRAM) {
    config.profile = {
      ...config.profile,
      iterations: config.profile?.iterations ?? 10,
      histogram: process.env.CURL_RUNNER_PROFILE_HISTOGRAM.toLowerCase() === 'true',
    };
  }

  if (process.env.CURL_RUNNER_PROFILE_EXPORT) {
    config.profile = {
      ...config.profile,
      iterations: config.profile?.iterations ?? 10,
      exportFile: process.env.CURL_RUNNER_PROFILE_EXPORT,
    };
  }
}

function loadSnapshotEnvVars(config: Partial<GlobalConfig>): void {
  if (process.env.CURL_RUNNER_SNAPSHOT) {
    config.snapshot = {
      ...config.snapshot,
      enabled: process.env.CURL_RUNNER_SNAPSHOT.toLowerCase() === 'true',
    };
  }

  if (process.env.CURL_RUNNER_SNAPSHOT_UPDATE) {
    const mode = process.env.CURL_RUNNER_SNAPSHOT_UPDATE.toLowerCase();
    if (['none', 'all', 'failing'].includes(mode)) {
      config.snapshot = {
        ...config.snapshot,
        updateMode: mode as 'none' | 'all' | 'failing',
      };
    }
  }

  if (process.env.CURL_RUNNER_SNAPSHOT_DIR) {
    config.snapshot = {
      ...config.snapshot,
      dir: process.env.CURL_RUNNER_SNAPSHOT_DIR,
    };
  }

  if (process.env.CURL_RUNNER_SNAPSHOT_CI) {
    config.snapshot = {
      ...config.snapshot,
      ci: process.env.CURL_RUNNER_SNAPSHOT_CI.toLowerCase() === 'true',
    };
  }
}

function loadDiffEnvVars(config: Partial<GlobalConfig>): void {
  if (process.env.CURL_RUNNER_DIFF) {
    config.diff = {
      ...config.diff,
      enabled: process.env.CURL_RUNNER_DIFF.toLowerCase() === 'true',
    };
  }

  if (process.env.CURL_RUNNER_DIFF_SAVE) {
    config.diff = {
      ...config.diff,
      save: process.env.CURL_RUNNER_DIFF_SAVE.toLowerCase() === 'true',
    };
  }

  if (process.env.CURL_RUNNER_DIFF_LABEL) {
    config.diff = {
      ...config.diff,
      label: process.env.CURL_RUNNER_DIFF_LABEL,
    };
  }

  if (process.env.CURL_RUNNER_DIFF_COMPARE) {
    config.diff = {
      ...config.diff,
      compareWith: process.env.CURL_RUNNER_DIFF_COMPARE,
    };
  }

  if (process.env.CURL_RUNNER_DIFF_DIR) {
    config.diff = {
      ...config.diff,
      dir: process.env.CURL_RUNNER_DIFF_DIR,
    };
  }

  if (process.env.CURL_RUNNER_DIFF_OUTPUT) {
    const format = process.env.CURL_RUNNER_DIFF_OUTPUT.toLowerCase();
    if (['terminal', 'json', 'markdown'].includes(format)) {
      config.diff = {
        ...config.diff,
        outputFormat: format as 'terminal' | 'json' | 'markdown',
      };
    }
  }
}

function loadRetryEnvVars(config: Partial<GlobalConfig>): void {
  if (process.env.CURL_RUNNER_TIMEOUT) {
    config.defaults = {
      ...config.defaults,
      timeout: Number.parseInt(process.env.CURL_RUNNER_TIMEOUT, 10),
    };
  }

  if (process.env.CURL_RUNNER_RETRIES) {
    config.defaults = {
      ...config.defaults,
      retry: {
        ...config.defaults?.retry,
        count: Number.parseInt(process.env.CURL_RUNNER_RETRIES, 10),
      },
    };
  }

  if (process.env.CURL_RUNNER_RETRY_DELAY) {
    config.defaults = {
      ...config.defaults,
      retry: {
        ...config.defaults?.retry,
        delay: Number.parseInt(process.env.CURL_RUNNER_RETRY_DELAY, 10),
      },
    };
  }
}
