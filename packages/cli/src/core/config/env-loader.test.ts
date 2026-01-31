import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import { loadEnvironmentConfig } from './env-loader';

describe('loadEnvironmentConfig', () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    // Clear all CURL_RUNNER_ env vars
    for (const key of Object.keys(process.env)) {
      if (key.startsWith('CURL_RUNNER_')) {
        delete process.env[key];
      }
    }
  });

  afterEach(() => {
    process.env = { ...originalEnv };
  });

  test('returns empty config when no env vars set', () => {
    const config = loadEnvironmentConfig();
    expect(config).toEqual({});
  });

  describe('execution env vars', () => {
    test('loads CURL_RUNNER_EXECUTION', () => {
      process.env.CURL_RUNNER_EXECUTION = 'parallel';
      const config = loadEnvironmentConfig();
      expect(config.execution).toBe('parallel');
    });

    test('loads CURL_RUNNER_CONTINUE_ON_ERROR', () => {
      process.env.CURL_RUNNER_CONTINUE_ON_ERROR = 'true';
      const config = loadEnvironmentConfig();
      expect(config.continueOnError).toBe(true);
    });

    test('loads CURL_RUNNER_CONTINUE_ON_ERROR case insensitive', () => {
      process.env.CURL_RUNNER_CONTINUE_ON_ERROR = 'TRUE';
      const config = loadEnvironmentConfig();
      expect(config.continueOnError).toBe(true);
    });

    test('loads CURL_RUNNER_DRY_RUN', () => {
      process.env.CURL_RUNNER_DRY_RUN = 'true';
      const config = loadEnvironmentConfig();
      expect(config.dryRun).toBe(true);
    });

    test('loads CURL_RUNNER_HTTP2', () => {
      process.env.CURL_RUNNER_HTTP2 = 'true';
      const config = loadEnvironmentConfig();
      expect(config.http2).toBe(true);
    });

    test('loads CURL_RUNNER_MAX_CONCURRENCY', () => {
      process.env.CURL_RUNNER_MAX_CONCURRENCY = '10';
      const config = loadEnvironmentConfig();
      expect(config.maxConcurrency).toBe(10);
    });

    test('ignores invalid CURL_RUNNER_MAX_CONCURRENCY', () => {
      process.env.CURL_RUNNER_MAX_CONCURRENCY = '0';
      const config = loadEnvironmentConfig();
      expect(config.maxConcurrency).toBeUndefined();
    });
  });

  describe('connection pool env vars', () => {
    test('loads CURL_RUNNER_CONNECTION_POOL', () => {
      process.env.CURL_RUNNER_CONNECTION_POOL = 'true';
      const config = loadEnvironmentConfig();
      expect(config.connectionPool?.enabled).toBe(true);
    });

    test('loads CURL_RUNNER_MAX_STREAMS_PER_HOST', () => {
      process.env.CURL_RUNNER_MAX_STREAMS_PER_HOST = '20';
      const config = loadEnvironmentConfig();
      expect(config.connectionPool?.maxStreamsPerHost).toBe(20);
    });

    test('loads CURL_RUNNER_KEEPALIVE_TIME', () => {
      process.env.CURL_RUNNER_KEEPALIVE_TIME = '120';
      const config = loadEnvironmentConfig();
      expect(config.connectionPool?.keepaliveTime).toBe(120);
    });

    test('loads CURL_RUNNER_CONNECT_TIMEOUT', () => {
      process.env.CURL_RUNNER_CONNECT_TIMEOUT = '60';
      const config = loadEnvironmentConfig();
      expect(config.connectionPool?.connectTimeout).toBe(60);
    });
  });

  describe('output env vars', () => {
    test('loads CURL_RUNNER_VERBOSE', () => {
      process.env.CURL_RUNNER_VERBOSE = 'true';
      const config = loadEnvironmentConfig();
      expect(config.output?.verbose).toBe(true);
    });

    test('loads CURL_RUNNER_OUTPUT_FORMAT json', () => {
      process.env.CURL_RUNNER_OUTPUT_FORMAT = 'json';
      const config = loadEnvironmentConfig();
      expect(config.output?.format).toBe('json');
    });

    test('loads CURL_RUNNER_OUTPUT_FORMAT pretty', () => {
      process.env.CURL_RUNNER_OUTPUT_FORMAT = 'pretty';
      const config = loadEnvironmentConfig();
      expect(config.output?.format).toBe('pretty');
    });

    test('ignores invalid CURL_RUNNER_OUTPUT_FORMAT', () => {
      process.env.CURL_RUNNER_OUTPUT_FORMAT = 'invalid';
      const config = loadEnvironmentConfig();
      expect(config.output?.format).toBeUndefined();
    });

    test('loads CURL_RUNNER_PRETTY_LEVEL minimal', () => {
      process.env.CURL_RUNNER_PRETTY_LEVEL = 'minimal';
      const config = loadEnvironmentConfig();
      expect(config.output?.prettyLevel).toBe('minimal');
    });

    test('loads CURL_RUNNER_PRETTY_LEVEL detailed', () => {
      process.env.CURL_RUNNER_PRETTY_LEVEL = 'detailed';
      const config = loadEnvironmentConfig();
      expect(config.output?.prettyLevel).toBe('detailed');
    });

    test('loads CURL_RUNNER_OUTPUT_FILE', () => {
      process.env.CURL_RUNNER_OUTPUT_FILE = 'results.json';
      const config = loadEnvironmentConfig();
      expect(config.output?.saveToFile).toBe('results.json');
    });
  });

  describe('CI env vars', () => {
    test('loads CURL_RUNNER_STRICT_EXIT', () => {
      process.env.CURL_RUNNER_STRICT_EXIT = 'true';
      const config = loadEnvironmentConfig();
      expect(config.ci?.strictExit).toBe(true);
    });

    test('loads CURL_RUNNER_FAIL_ON', () => {
      process.env.CURL_RUNNER_FAIL_ON = '3';
      const config = loadEnvironmentConfig();
      expect(config.ci?.failOn).toBe(3);
    });

    test('loads CURL_RUNNER_FAIL_ON_PERCENTAGE', () => {
      process.env.CURL_RUNNER_FAIL_ON_PERCENTAGE = '25.5';
      const config = loadEnvironmentConfig();
      expect(config.ci?.failOnPercentage).toBe(25.5);
    });

    test('ignores invalid CURL_RUNNER_FAIL_ON_PERCENTAGE above 100', () => {
      process.env.CURL_RUNNER_FAIL_ON_PERCENTAGE = '150';
      const config = loadEnvironmentConfig();
      expect(config.ci?.failOnPercentage).toBeUndefined();
    });

    test('ignores negative CURL_RUNNER_FAIL_ON_PERCENTAGE', () => {
      process.env.CURL_RUNNER_FAIL_ON_PERCENTAGE = '-10';
      const config = loadEnvironmentConfig();
      expect(config.ci?.failOnPercentage).toBeUndefined();
    });
  });

  describe('watch env vars', () => {
    test('loads CURL_RUNNER_WATCH', () => {
      process.env.CURL_RUNNER_WATCH = 'true';
      const config = loadEnvironmentConfig();
      expect(config.watch?.enabled).toBe(true);
    });

    test('loads CURL_RUNNER_WATCH_DEBOUNCE', () => {
      process.env.CURL_RUNNER_WATCH_DEBOUNCE = '500';
      const config = loadEnvironmentConfig();
      expect(config.watch?.debounce).toBe(500);
    });

    test('loads CURL_RUNNER_WATCH_CLEAR true', () => {
      process.env.CURL_RUNNER_WATCH_CLEAR = 'true';
      const config = loadEnvironmentConfig();
      expect(config.watch?.clear).toBe(true);
    });

    test('loads CURL_RUNNER_WATCH_CLEAR false', () => {
      process.env.CURL_RUNNER_WATCH_CLEAR = 'false';
      const config = loadEnvironmentConfig();
      expect(config.watch?.clear).toBe(false);
    });
  });

  describe('profile env vars', () => {
    test('loads CURL_RUNNER_PROFILE', () => {
      process.env.CURL_RUNNER_PROFILE = '100';
      const config = loadEnvironmentConfig();
      expect(config.profile?.iterations).toBe(100);
    });

    test('ignores invalid CURL_RUNNER_PROFILE', () => {
      process.env.CURL_RUNNER_PROFILE = '0';
      const config = loadEnvironmentConfig();
      expect(config.profile?.iterations).toBeUndefined();
    });

    test('loads CURL_RUNNER_PROFILE_WARMUP', () => {
      process.env.CURL_RUNNER_PROFILE_WARMUP = '5';
      const config = loadEnvironmentConfig();
      expect(config.profile?.warmup).toBe(5);
    });

    test('loads CURL_RUNNER_PROFILE_CONCURRENCY', () => {
      process.env.CURL_RUNNER_PROFILE_CONCURRENCY = '4';
      const config = loadEnvironmentConfig();
      expect(config.profile?.concurrency).toBe(4);
    });

    test('loads CURL_RUNNER_PROFILE_HISTOGRAM', () => {
      process.env.CURL_RUNNER_PROFILE_HISTOGRAM = 'true';
      const config = loadEnvironmentConfig();
      expect(config.profile?.histogram).toBe(true);
    });

    test('loads CURL_RUNNER_PROFILE_EXPORT', () => {
      process.env.CURL_RUNNER_PROFILE_EXPORT = 'perf.json';
      const config = loadEnvironmentConfig();
      expect(config.profile?.exportFile).toBe('perf.json');
    });
  });

  describe('snapshot env vars', () => {
    test('loads CURL_RUNNER_SNAPSHOT', () => {
      process.env.CURL_RUNNER_SNAPSHOT = 'true';
      const config = loadEnvironmentConfig();
      expect(config.snapshot?.enabled).toBe(true);
    });

    test('loads CURL_RUNNER_SNAPSHOT_UPDATE all', () => {
      process.env.CURL_RUNNER_SNAPSHOT_UPDATE = 'all';
      const config = loadEnvironmentConfig();
      expect(config.snapshot?.updateMode).toBe('all');
    });

    test('loads CURL_RUNNER_SNAPSHOT_UPDATE failing', () => {
      process.env.CURL_RUNNER_SNAPSHOT_UPDATE = 'failing';
      const config = loadEnvironmentConfig();
      expect(config.snapshot?.updateMode).toBe('failing');
    });

    test('loads CURL_RUNNER_SNAPSHOT_UPDATE none', () => {
      process.env.CURL_RUNNER_SNAPSHOT_UPDATE = 'none';
      const config = loadEnvironmentConfig();
      expect(config.snapshot?.updateMode).toBe('none');
    });

    test('ignores invalid CURL_RUNNER_SNAPSHOT_UPDATE', () => {
      process.env.CURL_RUNNER_SNAPSHOT_UPDATE = 'invalid';
      const config = loadEnvironmentConfig();
      expect(config.snapshot?.updateMode).toBeUndefined();
    });

    test('loads CURL_RUNNER_SNAPSHOT_DIR', () => {
      process.env.CURL_RUNNER_SNAPSHOT_DIR = '.snapshots';
      const config = loadEnvironmentConfig();
      expect(config.snapshot?.dir).toBe('.snapshots');
    });

    test('loads CURL_RUNNER_SNAPSHOT_CI', () => {
      process.env.CURL_RUNNER_SNAPSHOT_CI = 'true';
      const config = loadEnvironmentConfig();
      expect(config.snapshot?.ci).toBe(true);
    });
  });

  describe('diff env vars', () => {
    test('loads CURL_RUNNER_DIFF', () => {
      process.env.CURL_RUNNER_DIFF = 'true';
      const config = loadEnvironmentConfig();
      expect(config.diff?.enabled).toBe(true);
    });

    test('loads CURL_RUNNER_DIFF_SAVE', () => {
      process.env.CURL_RUNNER_DIFF_SAVE = 'true';
      const config = loadEnvironmentConfig();
      expect(config.diff?.save).toBe(true);
    });

    test('loads CURL_RUNNER_DIFF_LABEL', () => {
      process.env.CURL_RUNNER_DIFF_LABEL = 'production';
      const config = loadEnvironmentConfig();
      expect(config.diff?.label).toBe('production');
    });

    test('loads CURL_RUNNER_DIFF_COMPARE', () => {
      process.env.CURL_RUNNER_DIFF_COMPARE = 'baseline';
      const config = loadEnvironmentConfig();
      expect(config.diff?.compareWith).toBe('baseline');
    });

    test('loads CURL_RUNNER_DIFF_DIR', () => {
      process.env.CURL_RUNNER_DIFF_DIR = '.baselines';
      const config = loadEnvironmentConfig();
      expect(config.diff?.dir).toBe('.baselines');
    });

    test('loads CURL_RUNNER_DIFF_OUTPUT terminal', () => {
      process.env.CURL_RUNNER_DIFF_OUTPUT = 'terminal';
      const config = loadEnvironmentConfig();
      expect(config.diff?.outputFormat).toBe('terminal');
    });

    test('loads CURL_RUNNER_DIFF_OUTPUT json', () => {
      process.env.CURL_RUNNER_DIFF_OUTPUT = 'json';
      const config = loadEnvironmentConfig();
      expect(config.diff?.outputFormat).toBe('json');
    });

    test('loads CURL_RUNNER_DIFF_OUTPUT markdown', () => {
      process.env.CURL_RUNNER_DIFF_OUTPUT = 'markdown';
      const config = loadEnvironmentConfig();
      expect(config.diff?.outputFormat).toBe('markdown');
    });

    test('ignores invalid CURL_RUNNER_DIFF_OUTPUT', () => {
      process.env.CURL_RUNNER_DIFF_OUTPUT = 'invalid';
      const config = loadEnvironmentConfig();
      expect(config.diff?.outputFormat).toBeUndefined();
    });
  });

  describe('retry env vars', () => {
    test('loads CURL_RUNNER_TIMEOUT', () => {
      process.env.CURL_RUNNER_TIMEOUT = '30';
      const config = loadEnvironmentConfig();
      expect(config.defaults?.timeout).toBe(30);
    });

    test('loads CURL_RUNNER_RETRIES', () => {
      process.env.CURL_RUNNER_RETRIES = '5';
      const config = loadEnvironmentConfig();
      expect(config.defaults?.retry?.count).toBe(5);
    });

    test('loads CURL_RUNNER_RETRY_DELAY', () => {
      process.env.CURL_RUNNER_RETRY_DELAY = '2000';
      const config = loadEnvironmentConfig();
      expect(config.defaults?.retry?.delay).toBe(2000);
    });
  });

  describe('multiple env vars', () => {
    test('loads multiple env vars together', () => {
      process.env.CURL_RUNNER_EXECUTION = 'parallel';
      process.env.CURL_RUNNER_MAX_CONCURRENCY = '5';
      process.env.CURL_RUNNER_HTTP2 = 'true';
      process.env.CURL_RUNNER_VERBOSE = 'true';
      process.env.CURL_RUNNER_TIMEOUT = '30';

      const config = loadEnvironmentConfig();

      expect(config.execution).toBe('parallel');
      expect(config.maxConcurrency).toBe(5);
      expect(config.http2).toBe(true);
      expect(config.output?.verbose).toBe(true);
      expect(config.defaults?.timeout).toBe(30);
    });
  });
});
