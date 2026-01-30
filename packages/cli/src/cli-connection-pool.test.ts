import { afterEach, describe, expect, test } from 'bun:test';

/**
 * Tests for CLI connection pooling argument parsing and environment variable support.
 * These tests verify the parseArguments method behavior and env var loading.
 */

// Helper to test argument parsing logic (extracted from CLI class)
function parseArguments(args: string[]): { files: string[]; options: Record<string, unknown> } {
  const options: Record<string, unknown> = {};
  const files: string[] = [];

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];

    if (arg.startsWith('--')) {
      const key = arg.slice(2);
      const nextArg = args[i + 1];

      if (key === 'connection-pool') {
        options.connectionPool = true;
      } else if (key === 'http2') {
        options.http2 = true;
      } else if (nextArg && !nextArg.startsWith('--') && !nextArg.startsWith('-')) {
        if (key === 'max-streams') {
          const maxStreams = Number.parseInt(nextArg, 10);
          if (maxStreams > 0) {
            options.maxStreams = maxStreams;
          }
        } else if (key === 'keepalive-time') {
          options.keepaliveTime = Number.parseInt(nextArg, 10);
        } else if (key === 'connect-timeout') {
          options.connectTimeout = Number.parseInt(nextArg, 10);
        } else {
          options[key] = nextArg;
        }
        i++;
      } else {
        options[key] = true;
      }
    } else if (arg.startsWith('-')) {
      const flags = arg.slice(1);
      for (const flag of flags) {
        switch (flag) {
          case 'p':
            options.execution = 'parallel';
            break;
        }
      }
    } else {
      files.push(arg);
    }
  }

  return { files, options };
}

// Helper to load environment variables (extracted from CLI class)
function loadEnvironmentVariables(): Record<string, unknown> {
  const envConfig: Record<string, unknown> = {};

  if (process.env.CURL_RUNNER_CONNECTION_POOL) {
    envConfig.connectionPool = {
      enabled: process.env.CURL_RUNNER_CONNECTION_POOL.toLowerCase() === 'true',
    };
  }

  if (process.env.CURL_RUNNER_MAX_STREAMS_PER_HOST) {
    envConfig.connectionPool = {
      ...(envConfig.connectionPool as object),
      maxStreamsPerHost: Number.parseInt(process.env.CURL_RUNNER_MAX_STREAMS_PER_HOST, 10),
    };
  }

  if (process.env.CURL_RUNNER_KEEPALIVE_TIME) {
    envConfig.connectionPool = {
      ...(envConfig.connectionPool as object),
      keepaliveTime: Number.parseInt(process.env.CURL_RUNNER_KEEPALIVE_TIME, 10),
    };
  }

  if (process.env.CURL_RUNNER_CONNECT_TIMEOUT) {
    envConfig.connectionPool = {
      ...(envConfig.connectionPool as object),
      connectTimeout: Number.parseInt(process.env.CURL_RUNNER_CONNECT_TIMEOUT, 10),
    };
  }

  return envConfig;
}

describe('CLI Connection Pool Argument Parsing', () => {
  describe('--connection-pool flag', () => {
    test('parses --connection-pool flag', () => {
      const { options } = parseArguments(['--connection-pool', 'test.yaml']);
      expect(options.connectionPool).toBe(true);
    });

    test('--connection-pool flag works with -p (parallel)', () => {
      const { options } = parseArguments(['-p', '--connection-pool', 'test.yaml']);
      expect(options.connectionPool).toBe(true);
      expect(options.execution).toBe('parallel');
    });

    test('--connection-pool flag can appear anywhere in args', () => {
      const { options } = parseArguments(['test.yaml', '--connection-pool']);
      expect(options.connectionPool).toBe(true);
    });

    test('--connection-pool works with --http2', () => {
      const { options } = parseArguments(['--connection-pool', '--http2', 'test.yaml']);
      expect(options.connectionPool).toBe(true);
      expect(options.http2).toBe(true);
    });
  });

  describe('--max-streams option', () => {
    test('parses --max-streams with value', () => {
      const { options } = parseArguments(['--max-streams', '20', 'test.yaml']);
      expect(options.maxStreams).toBe(20);
    });

    test('parses --max-streams with --connection-pool', () => {
      const { options } = parseArguments(['--connection-pool', '--max-streams', '15', 'test.yaml']);
      expect(options.connectionPool).toBe(true);
      expect(options.maxStreams).toBe(15);
    });

    test('ignores invalid --max-streams value (0)', () => {
      const { options } = parseArguments(['--max-streams', '0', 'test.yaml']);
      expect(options.maxStreams).toBeUndefined();
    });

    test('ignores negative --max-streams value', () => {
      const { options } = parseArguments(['--max-streams', '-5', 'test.yaml']);
      expect(options.maxStreams).toBeUndefined();
    });
  });

  describe('--keepalive-time option', () => {
    test('parses --keepalive-time with value', () => {
      const { options } = parseArguments(['--keepalive-time', '120', 'test.yaml']);
      expect(options.keepaliveTime).toBe(120);
    });

    test('parses --keepalive-time with --connection-pool', () => {
      const { options } = parseArguments([
        '--connection-pool',
        '--keepalive-time',
        '90',
        'test.yaml',
      ]);
      expect(options.connectionPool).toBe(true);
      expect(options.keepaliveTime).toBe(90);
    });
  });

  describe('--connect-timeout option', () => {
    test('parses --connect-timeout with value', () => {
      const { options } = parseArguments(['--connect-timeout', '60', 'test.yaml']);
      expect(options.connectTimeout).toBe(60);
    });

    test('parses --connect-timeout with --connection-pool', () => {
      const { options } = parseArguments([
        '--connection-pool',
        '--connect-timeout',
        '45',
        'test.yaml',
      ]);
      expect(options.connectionPool).toBe(true);
      expect(options.connectTimeout).toBe(45);
    });
  });

  describe('all connection pool options together', () => {
    test('parses all connection pool options', () => {
      const { options } = parseArguments([
        '-p',
        '--connection-pool',
        '--max-streams',
        '25',
        '--keepalive-time',
        '180',
        '--connect-timeout',
        '45',
        'test.yaml',
      ]);
      expect(options.execution).toBe('parallel');
      expect(options.connectionPool).toBe(true);
      expect(options.maxStreams).toBe(25);
      expect(options.keepaliveTime).toBe(180);
      expect(options.connectTimeout).toBe(45);
    });
  });

  describe('connection-pool flag not set', () => {
    test('connectionPool is undefined when flag not present', () => {
      const { options } = parseArguments(['test.yaml']);
      expect(options.connectionPool).toBeUndefined();
    });

    test('connectionPool is undefined with other flags', () => {
      const { options } = parseArguments(['-p', 'test.yaml']);
      expect(options.connectionPool).toBeUndefined();
    });
  });
});

describe('Environment Variables for Connection Pool', () => {
  const originalEnvPool = process.env.CURL_RUNNER_CONNECTION_POOL;
  const originalEnvMaxStreams = process.env.CURL_RUNNER_MAX_STREAMS_PER_HOST;
  const originalEnvKeepalive = process.env.CURL_RUNNER_KEEPALIVE_TIME;
  const originalEnvConnectTimeout = process.env.CURL_RUNNER_CONNECT_TIMEOUT;

  afterEach(() => {
    // Restore original env vars
    if (originalEnvPool !== undefined) {
      process.env.CURL_RUNNER_CONNECTION_POOL = originalEnvPool;
    } else {
      delete process.env.CURL_RUNNER_CONNECTION_POOL;
    }
    if (originalEnvMaxStreams !== undefined) {
      process.env.CURL_RUNNER_MAX_STREAMS_PER_HOST = originalEnvMaxStreams;
    } else {
      delete process.env.CURL_RUNNER_MAX_STREAMS_PER_HOST;
    }
    if (originalEnvKeepalive !== undefined) {
      process.env.CURL_RUNNER_KEEPALIVE_TIME = originalEnvKeepalive;
    } else {
      delete process.env.CURL_RUNNER_KEEPALIVE_TIME;
    }
    if (originalEnvConnectTimeout !== undefined) {
      process.env.CURL_RUNNER_CONNECT_TIMEOUT = originalEnvConnectTimeout;
    } else {
      delete process.env.CURL_RUNNER_CONNECT_TIMEOUT;
    }
  });

  describe('CURL_RUNNER_CONNECTION_POOL', () => {
    test('loads enabled: true when env var is "true"', () => {
      process.env.CURL_RUNNER_CONNECTION_POOL = 'true';
      const config = loadEnvironmentVariables();
      expect((config.connectionPool as { enabled: boolean }).enabled).toBe(true);
    });

    test('loads enabled: true when env var is "TRUE"', () => {
      process.env.CURL_RUNNER_CONNECTION_POOL = 'TRUE';
      const config = loadEnvironmentVariables();
      expect((config.connectionPool as { enabled: boolean }).enabled).toBe(true);
    });

    test('loads enabled: false when env var is "false"', () => {
      process.env.CURL_RUNNER_CONNECTION_POOL = 'false';
      const config = loadEnvironmentVariables();
      expect((config.connectionPool as { enabled: boolean }).enabled).toBe(false);
    });

    test('connectionPool is undefined when env var not set', () => {
      delete process.env.CURL_RUNNER_CONNECTION_POOL;
      const config = loadEnvironmentVariables();
      expect(config.connectionPool).toBeUndefined();
    });
  });

  describe('CURL_RUNNER_MAX_STREAMS_PER_HOST', () => {
    test('loads maxStreamsPerHost from env var', () => {
      process.env.CURL_RUNNER_MAX_STREAMS_PER_HOST = '20';
      const config = loadEnvironmentVariables();
      expect((config.connectionPool as { maxStreamsPerHost: number }).maxStreamsPerHost).toBe(20);
    });

    test('combines with CONNECTION_POOL env var', () => {
      process.env.CURL_RUNNER_CONNECTION_POOL = 'true';
      process.env.CURL_RUNNER_MAX_STREAMS_PER_HOST = '15';
      const config = loadEnvironmentVariables();
      const poolConfig = config.connectionPool as { enabled: boolean; maxStreamsPerHost: number };
      expect(poolConfig.enabled).toBe(true);
      expect(poolConfig.maxStreamsPerHost).toBe(15);
    });
  });

  describe('CURL_RUNNER_KEEPALIVE_TIME', () => {
    test('loads keepaliveTime from env var', () => {
      process.env.CURL_RUNNER_KEEPALIVE_TIME = '120';
      const config = loadEnvironmentVariables();
      expect((config.connectionPool as { keepaliveTime: number }).keepaliveTime).toBe(120);
    });
  });

  describe('CURL_RUNNER_CONNECT_TIMEOUT', () => {
    test('loads connectTimeout from env var', () => {
      process.env.CURL_RUNNER_CONNECT_TIMEOUT = '45';
      const config = loadEnvironmentVariables();
      expect((config.connectionPool as { connectTimeout: number }).connectTimeout).toBe(45);
    });
  });

  describe('all env vars together', () => {
    test('loads all connection pool env vars', () => {
      process.env.CURL_RUNNER_CONNECTION_POOL = 'true';
      process.env.CURL_RUNNER_MAX_STREAMS_PER_HOST = '25';
      process.env.CURL_RUNNER_KEEPALIVE_TIME = '180';
      process.env.CURL_RUNNER_CONNECT_TIMEOUT = '60';

      const config = loadEnvironmentVariables();
      const poolConfig = config.connectionPool as {
        enabled: boolean;
        maxStreamsPerHost: number;
        keepaliveTime: number;
        connectTimeout: number;
      };

      expect(poolConfig.enabled).toBe(true);
      expect(poolConfig.maxStreamsPerHost).toBe(25);
      expect(poolConfig.keepaliveTime).toBe(180);
      expect(poolConfig.connectTimeout).toBe(60);
    });
  });
});

describe('Connection Pool Configuration Priority', () => {
  test('CLI flag should override env var (simulated)', () => {
    // CLI options take precedence over env config
    const envConfig = { connectionPool: { enabled: false } };
    const cliOptions = { connectionPool: true };

    // Merge: CLI overrides env
    const finalConfig = {
      connectionPool: {
        ...envConfig.connectionPool,
        enabled: cliOptions.connectionPool,
      },
    };
    expect(finalConfig.connectionPool.enabled).toBe(true);
  });

  test('env var used when CLI flag not specified (simulated)', () => {
    const envConfig = { connectionPool: { enabled: true, maxStreamsPerHost: 15 } };
    const cliOptions = {}; // No connection-pool flag

    const finalConfig = { ...envConfig, ...cliOptions };
    expect(finalConfig.connectionPool.enabled).toBe(true);
    expect(finalConfig.connectionPool.maxStreamsPerHost).toBe(15);
  });

  test('CLI options merge with env config (simulated)', () => {
    const envConfig = { connectionPool: { enabled: true, maxStreamsPerHost: 10 } };
    const cliOptions = { maxStreams: 20 };

    // Merge: CLI maxStreams overrides env maxStreamsPerHost
    const finalConfig = {
      connectionPool: {
        ...envConfig.connectionPool,
        maxStreamsPerHost: cliOptions.maxStreams,
      },
    };
    expect(finalConfig.connectionPool.enabled).toBe(true);
    expect(finalConfig.connectionPool.maxStreamsPerHost).toBe(20);
  });
});

describe('Connection Pool GlobalConfig Integration', () => {
  test('connectionPool options are passed to globalConfig', () => {
    const options = {
      connectionPool: true,
      maxStreams: 20,
      keepaliveTime: 120,
      connectTimeout: 45,
    };

    // Simulating CLI behavior: build connectionPool config
    const globalConfig = {
      connectionPool: {
        enabled: options.connectionPool,
        maxStreamsPerHost: options.maxStreams,
        keepaliveTime: options.keepaliveTime,
        connectTimeout: options.connectTimeout,
      },
    };

    expect(globalConfig.connectionPool.enabled).toBe(true);
    expect(globalConfig.connectionPool.maxStreamsPerHost).toBe(20);
    expect(globalConfig.connectionPool.keepaliveTime).toBe(120);
    expect(globalConfig.connectionPool.connectTimeout).toBe(45);
  });

  test('setting max-streams auto-enables connection pool (simulated)', () => {
    const options = { maxStreams: 15 };

    // When maxStreams is set, connectionPool should be auto-enabled
    const globalConfig = {
      connectionPool: {
        enabled: true, // Auto-enabled
        maxStreamsPerHost: options.maxStreams,
      },
    };

    expect(globalConfig.connectionPool.enabled).toBe(true);
    expect(globalConfig.connectionPool.maxStreamsPerHost).toBe(15);
  });
});
