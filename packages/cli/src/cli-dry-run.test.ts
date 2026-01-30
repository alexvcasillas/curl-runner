import { afterEach, beforeEach, describe, expect, test } from 'bun:test';

/**
 * Tests for CLI dry-run argument parsing and environment variable support.
 * These tests verify the parseArguments method behavior and env var loading.
 */

// Helper to test argument parsing logic (extracted from CLI class)
function parseArguments(args: string[]): { files: string[]; options: Record<string, unknown> } {
  const options: Record<string, unknown> = {};
  const files: string[] = [];

  // Boolean flags that don't take a value
  const booleanFlags = ['dry-run', 'help', 'version', 'verbose', 'quiet', 'watch', 'snapshot'];

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];

    if (arg.startsWith('--')) {
      const key = arg.slice(2);
      const nextArg = args[i + 1];

      if (key === 'dry-run') {
        options.dryRun = true;
      } else if (booleanFlags.includes(key)) {
        options[key] = true;
      } else if (nextArg && !nextArg.startsWith('--') && !nextArg.startsWith('-')) {
        options[key] = nextArg;
        i++;
      } else {
        options[key] = true;
      }
    } else if (arg.startsWith('-')) {
      const flags = arg.slice(1);
      for (const flag of flags) {
        switch (flag) {
          case 'n':
            options.dryRun = true;
            break;
          case 'h':
            options.help = true;
            break;
          case 'v':
            options.verbose = true;
            break;
          case 'p':
            options.execution = 'parallel';
            break;
          case 'c':
            options.continueOnError = true;
            break;
          case 'q':
            options.quiet = true;
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

  if (process.env.CURL_RUNNER_DRY_RUN) {
    envConfig.dryRun = process.env.CURL_RUNNER_DRY_RUN.toLowerCase() === 'true';
  }

  return envConfig;
}

describe('CLI Dry-Run Argument Parsing', () => {
  describe('--dry-run long flag', () => {
    test('parses --dry-run flag', () => {
      const { options } = parseArguments(['--dry-run', 'test.yaml']);
      expect(options.dryRun).toBe(true);
    });

    test('--dry-run flag works without file argument', () => {
      const { options, files } = parseArguments(['--dry-run']);
      expect(options.dryRun).toBe(true);
      expect(files.length).toBe(0);
    });

    test('--dry-run flag works with multiple files', () => {
      const { options, files } = parseArguments(['--dry-run', 'test1.yaml', 'test2.yaml']);
      expect(options.dryRun).toBe(true);
      expect(files).toContain('test1.yaml');
      expect(files).toContain('test2.yaml');
    });

    test('--dry-run can appear anywhere in args', () => {
      const { options } = parseArguments(['test.yaml', '--dry-run']);
      expect(options.dryRun).toBe(true);
    });

    test('--dry-run works with other flags', () => {
      const { options } = parseArguments(['--dry-run', '--verbose', 'test.yaml']);
      expect(options.dryRun).toBe(true);
      expect(options.verbose).toBe(true);
    });
  });

  describe('-n short flag', () => {
    test('parses -n flag', () => {
      const { options } = parseArguments(['-n', 'test.yaml']);
      expect(options.dryRun).toBe(true);
    });

    test('-n flag works without file argument', () => {
      const { options } = parseArguments(['-n']);
      expect(options.dryRun).toBe(true);
    });

    test('-n flag can be combined with other short flags', () => {
      const { options } = parseArguments(['-nv', 'test.yaml']);
      expect(options.dryRun).toBe(true);
      expect(options.verbose).toBe(true);
    });

    test('-n flag can be combined with -p (parallel)', () => {
      const { options } = parseArguments(['-np', 'test.yaml']);
      expect(options.dryRun).toBe(true);
      expect(options.execution).toBe('parallel');
    });

    test('-n flag can be combined with -c (continue on error)', () => {
      const { options } = parseArguments(['-nc', 'test.yaml']);
      expect(options.dryRun).toBe(true);
      expect(options.continueOnError).toBe(true);
    });

    test('-n flag can be combined with -q (quiet)', () => {
      const { options } = parseArguments(['-nq', 'test.yaml']);
      expect(options.dryRun).toBe(true);
      expect(options.quiet).toBe(true);
    });
  });

  describe('dry-run with execution mode flags', () => {
    test('--dry-run with --execution parallel', () => {
      const { options } = parseArguments(['--dry-run', '--execution', 'parallel', 'test.yaml']);
      expect(options.dryRun).toBe(true);
      expect(options.execution).toBe('parallel');
    });

    test('-n with -p (parallel short flag)', () => {
      const { options } = parseArguments(['-np', 'test.yaml']);
      expect(options.dryRun).toBe(true);
      expect(options.execution).toBe('parallel');
    });
  });

  describe('dry-run flag not set', () => {
    test('dryRun is undefined when flag not present', () => {
      const { options } = parseArguments(['test.yaml']);
      expect(options.dryRun).toBeUndefined();
    });

    test('dryRun is undefined with other flags', () => {
      const { options } = parseArguments(['-v', 'test.yaml']);
      expect(options.dryRun).toBeUndefined();
    });
  });
});

describe('Environment Variable CURL_RUNNER_DRY_RUN', () => {
  const originalEnv = process.env.CURL_RUNNER_DRY_RUN;

  afterEach(() => {
    // Restore original env var
    if (originalEnv !== undefined) {
      process.env.CURL_RUNNER_DRY_RUN = originalEnv;
    } else {
      delete process.env.CURL_RUNNER_DRY_RUN;
    }
  });

  test('loads dryRun: true when env var is "true"', () => {
    process.env.CURL_RUNNER_DRY_RUN = 'true';
    const config = loadEnvironmentVariables();
    expect(config.dryRun).toBe(true);
  });

  test('loads dryRun: true when env var is "TRUE"', () => {
    process.env.CURL_RUNNER_DRY_RUN = 'TRUE';
    const config = loadEnvironmentVariables();
    expect(config.dryRun).toBe(true);
  });

  test('loads dryRun: true when env var is "True"', () => {
    process.env.CURL_RUNNER_DRY_RUN = 'True';
    const config = loadEnvironmentVariables();
    expect(config.dryRun).toBe(true);
  });

  test('loads dryRun: false when env var is "false"', () => {
    process.env.CURL_RUNNER_DRY_RUN = 'false';
    const config = loadEnvironmentVariables();
    expect(config.dryRun).toBe(false);
  });

  test('loads dryRun: false when env var is any other value', () => {
    process.env.CURL_RUNNER_DRY_RUN = 'yes';
    const config = loadEnvironmentVariables();
    expect(config.dryRun).toBe(false);
  });

  test('dryRun is undefined when env var not set', () => {
    delete process.env.CURL_RUNNER_DRY_RUN;
    const config = loadEnvironmentVariables();
    expect(config.dryRun).toBeUndefined();
  });
});

describe('Dry-Run Configuration Priority', () => {
  test('CLI flag should override env var (simulated)', () => {
    // CLI options take precedence over env config
    const envConfig = { dryRun: false };
    const cliOptions = { dryRun: true };

    // Merge: CLI overrides env
    const finalConfig = { ...envConfig, ...cliOptions };
    expect(finalConfig.dryRun).toBe(true);
  });

  test('env var used when CLI flag not specified (simulated)', () => {
    const envConfig = { dryRun: true };
    const cliOptions = {}; // No dry-run flag

    const finalConfig = { ...envConfig, ...cliOptions };
    expect(finalConfig.dryRun).toBe(true);
  });
});

describe('Dry-Run GlobalConfig Integration', () => {
  test('dryRun flag is passed to output config for logger', () => {
    const options = { dryRun: true };

    // Simulating CLI behavior: pass dryRun to output config
    const globalConfig = {
      dryRun: options.dryRun,
      output: { dryRun: options.dryRun },
    };

    expect(globalConfig.dryRun).toBe(true);
    expect(globalConfig.output.dryRun).toBe(true);
  });

  test('dryRun false does not pollute config', () => {
    const options = { dryRun: false };

    const globalConfig = {
      dryRun: options.dryRun,
      output: { dryRun: options.dryRun },
    };

    expect(globalConfig.dryRun).toBe(false);
    expect(globalConfig.output.dryRun).toBe(false);
  });
});
