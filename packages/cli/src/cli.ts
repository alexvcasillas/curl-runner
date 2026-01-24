#!/usr/bin/env bun

import { Glob } from 'bun';
import { RequestExecutor } from './executor/request-executor';
import { YamlParser } from './parser/yaml';
import type {
  ExecutionResult,
  ExecutionSummary,
  GlobalConfig,
  RequestConfig,
  WatchConfig,
} from './types/config';
import { Logger } from './utils/logger';
import { VersionChecker } from './utils/version-checker';
import { getVersion } from './version';
import { FileWatcher } from './watcher/file-watcher';

class CurlRunnerCLI {
  private logger = new Logger();

  private async loadConfigFile(): Promise<Partial<GlobalConfig>> {
    const configFiles = [
      'curl-runner.yaml',
      'curl-runner.yml',
      '.curl-runner.yaml',
      '.curl-runner.yml',
    ];

    for (const filename of configFiles) {
      try {
        const file = Bun.file(filename);
        if (await file.exists()) {
          const yamlContent = await YamlParser.parseFile(filename);
          // Extract global config from the YAML file
          const config = yamlContent.global || yamlContent;
          this.logger.logInfo(`Loaded configuration from ${filename}`);
          return config;
        }
      } catch (error) {
        this.logger.logWarning(`Failed to load configuration from ${filename}: ${error}`);
      }
    }

    return {};
  }

  private loadEnvironmentVariables(): Partial<GlobalConfig> {
    const envConfig: Partial<GlobalConfig> = {};

    // Load environment variables
    if (process.env.CURL_RUNNER_TIMEOUT) {
      envConfig.defaults = {
        ...envConfig.defaults,
        timeout: Number.parseInt(process.env.CURL_RUNNER_TIMEOUT, 10),
      };
    }

    if (process.env.CURL_RUNNER_RETRIES) {
      envConfig.defaults = {
        ...envConfig.defaults,
        retry: {
          ...envConfig.defaults?.retry,
          count: Number.parseInt(process.env.CURL_RUNNER_RETRIES, 10),
        },
      };
    }

    if (process.env.CURL_RUNNER_RETRY_DELAY) {
      envConfig.defaults = {
        ...envConfig.defaults,
        retry: {
          ...envConfig.defaults?.retry,
          delay: Number.parseInt(process.env.CURL_RUNNER_RETRY_DELAY, 10),
        },
      };
    }

    if (process.env.CURL_RUNNER_VERBOSE) {
      envConfig.output = {
        ...envConfig.output,
        verbose: process.env.CURL_RUNNER_VERBOSE.toLowerCase() === 'true',
      };
    }

    if (process.env.CURL_RUNNER_EXECUTION) {
      envConfig.execution = process.env.CURL_RUNNER_EXECUTION as 'sequential' | 'parallel';
    }

    if (process.env.CURL_RUNNER_CONTINUE_ON_ERROR) {
      envConfig.continueOnError =
        process.env.CURL_RUNNER_CONTINUE_ON_ERROR.toLowerCase() === 'true';
    }

    if (process.env.CURL_RUNNER_MAX_CONCURRENCY) {
      const maxConcurrency = Number.parseInt(process.env.CURL_RUNNER_MAX_CONCURRENCY, 10);
      if (maxConcurrency > 0) {
        envConfig.maxConcurrency = maxConcurrency;
      }
    }

    if (process.env.CURL_RUNNER_OUTPUT_FORMAT) {
      const format = process.env.CURL_RUNNER_OUTPUT_FORMAT;
      if (['json', 'pretty', 'raw'].includes(format)) {
        envConfig.output = { ...envConfig.output, format: format as 'json' | 'pretty' | 'raw' };
      }
    }

    if (process.env.CURL_RUNNER_PRETTY_LEVEL) {
      const level = process.env.CURL_RUNNER_PRETTY_LEVEL;
      if (['minimal', 'standard', 'detailed'].includes(level)) {
        envConfig.output = {
          ...envConfig.output,
          prettyLevel: level as 'minimal' | 'standard' | 'detailed',
        };
      }
    }

    if (process.env.CURL_RUNNER_OUTPUT_FILE) {
      envConfig.output = { ...envConfig.output, saveToFile: process.env.CURL_RUNNER_OUTPUT_FILE };
    }

    // CI exit code configuration
    if (process.env.CURL_RUNNER_STRICT_EXIT) {
      envConfig.ci = {
        ...envConfig.ci,
        strictExit: process.env.CURL_RUNNER_STRICT_EXIT.toLowerCase() === 'true',
      };
    }

    if (process.env.CURL_RUNNER_FAIL_ON) {
      envConfig.ci = {
        ...envConfig.ci,
        failOn: Number.parseInt(process.env.CURL_RUNNER_FAIL_ON, 10),
      };
    }

    if (process.env.CURL_RUNNER_FAIL_ON_PERCENTAGE) {
      const percentage = Number.parseFloat(process.env.CURL_RUNNER_FAIL_ON_PERCENTAGE);
      if (percentage >= 0 && percentage <= 100) {
        envConfig.ci = {
          ...envConfig.ci,
          failOnPercentage: percentage,
        };
      }
    }

    // Watch mode configuration
    if (process.env.CURL_RUNNER_WATCH) {
      envConfig.watch = {
        ...envConfig.watch,
        enabled: process.env.CURL_RUNNER_WATCH.toLowerCase() === 'true',
      };
    }

    if (process.env.CURL_RUNNER_WATCH_DEBOUNCE) {
      envConfig.watch = {
        ...envConfig.watch,
        debounce: Number.parseInt(process.env.CURL_RUNNER_WATCH_DEBOUNCE, 10),
      };
    }

    if (process.env.CURL_RUNNER_WATCH_CLEAR) {
      envConfig.watch = {
        ...envConfig.watch,
        clear: process.env.CURL_RUNNER_WATCH_CLEAR.toLowerCase() !== 'false',
      };
    }

    return envConfig;
  }

  async run(args: string[]): Promise<void> {
    try {
      const { files, options } = this.parseArguments(args);

      // Check for updates in the background (non-blocking)
      if (!options.version && !options.help) {
        const versionChecker = new VersionChecker();
        versionChecker.checkForUpdates().catch(() => {
          // Silently ignore any errors
        });
      }

      if (options.help) {
        this.showHelp();
        return;
      }

      if (options.version) {
        console.log(`curl-runner v${getVersion()}`);
        return;
      }

      // Load configuration from environment variables, config file, then CLI options
      const envConfig = this.loadEnvironmentVariables();
      const configFile = await this.loadConfigFile();

      const yamlFiles = await this.findYamlFiles(files, options);

      if (yamlFiles.length === 0) {
        this.logger.logError('No YAML files found');
        process.exit(1);
      }

      this.logger.logInfo(`Found ${yamlFiles.length} YAML file(s)`);

      let globalConfig: GlobalConfig = this.mergeGlobalConfigs(envConfig, configFile);
      const allRequests: RequestConfig[] = [];

      // Group requests by file to show clear file separations in output
      const fileGroups: Array<{ file: string; requests: RequestConfig[]; config?: GlobalConfig }> =
        [];

      for (const file of yamlFiles) {
        this.logger.logInfo(`Processing: ${file}`);
        const { requests, config } = await this.processYamlFile(file);

        // Associate each request with its source file's output configuration and filename
        const fileOutputConfig = config?.output || {};
        const requestsWithSourceConfig = requests.map((request) => ({
          ...request,
          sourceOutputConfig: fileOutputConfig,
          sourceFile: file,
        }));

        // Only merge non-output global configs (execution, continueOnError, variables, defaults)
        if (config) {
          const { ...nonOutputConfig } = config;
          globalConfig = this.mergeGlobalConfigs(globalConfig, nonOutputConfig);
        }

        fileGroups.push({ file, requests: requestsWithSourceConfig, config });
        allRequests.push(...requestsWithSourceConfig);
      }

      if (options.execution) {
        globalConfig.execution = options.execution as 'sequential' | 'parallel';
      }
      if (options.maxConcurrent !== undefined) {
        globalConfig.maxConcurrency = options.maxConcurrent as number;
      }
      if (options.continueOnError !== undefined) {
        globalConfig.continueOnError = options.continueOnError;
      }
      if (options.verbose !== undefined) {
        globalConfig.output = { ...globalConfig.output, verbose: options.verbose };
      }
      if (options.quiet !== undefined) {
        globalConfig.output = { ...globalConfig.output, verbose: false };
      }
      if (options.output) {
        globalConfig.output = { ...globalConfig.output, saveToFile: options.output };
      }
      if (options.outputFormat) {
        globalConfig.output = {
          ...globalConfig.output,
          format: options.outputFormat as 'json' | 'pretty' | 'raw',
        };
      }
      if (options.prettyLevel) {
        globalConfig.output = {
          ...globalConfig.output,
          prettyLevel: options.prettyLevel as 'minimal' | 'standard' | 'detailed',
        };
      }
      if (options.showHeaders !== undefined) {
        globalConfig.output = { ...globalConfig.output, showHeaders: options.showHeaders };
      }
      if (options.showBody !== undefined) {
        globalConfig.output = { ...globalConfig.output, showBody: options.showBody };
      }
      if (options.showMetrics !== undefined) {
        globalConfig.output = { ...globalConfig.output, showMetrics: options.showMetrics };
      }

      // Apply timeout and retry settings to defaults
      if (options.timeout) {
        globalConfig.defaults = { ...globalConfig.defaults, timeout: options.timeout };
      }
      if (options.retries || options.noRetry) {
        const retryCount = options.noRetry ? 0 : options.retries || 0;
        globalConfig.defaults = {
          ...globalConfig.defaults,
          retry: {
            ...globalConfig.defaults?.retry,
            count: retryCount,
          },
        };
      }
      if (options.retryDelay) {
        globalConfig.defaults = {
          ...globalConfig.defaults,
          retry: {
            ...globalConfig.defaults?.retry,
            delay: options.retryDelay,
          },
        };
      }

      // Apply CI exit code options
      if (options.strictExit !== undefined) {
        globalConfig.ci = { ...globalConfig.ci, strictExit: options.strictExit as boolean };
      }
      if (options.failOn !== undefined) {
        globalConfig.ci = { ...globalConfig.ci, failOn: options.failOn as number };
      }
      if (options.failOnPercentage !== undefined) {
        globalConfig.ci = {
          ...globalConfig.ci,
          failOnPercentage: options.failOnPercentage as number,
        };
      }

      if (allRequests.length === 0) {
        this.logger.logError('No requests found in YAML files');
        process.exit(1);
      }

      // Check if watch mode is enabled
      const watchEnabled = options.watch || globalConfig.watch?.enabled;

      if (watchEnabled) {
        // Build watch config from options and global config
        const watchConfig: WatchConfig = {
          enabled: true,
          debounce:
            (options.watchDebounce as number | undefined) ?? globalConfig.watch?.debounce ?? 300,
          clear: (options.watchClear as boolean | undefined) ?? globalConfig.watch?.clear ?? true,
        };

        const watcher = new FileWatcher({
          files: yamlFiles,
          config: watchConfig,
          logger: this.logger,
          onRun: async () => {
            await this.executeRequests(yamlFiles, globalConfig);
          },
        });

        await watcher.start();
      } else {
        // Normal execution mode
        const summary = await this.executeRequests(yamlFiles, globalConfig);
        const exitCode = this.determineExitCode(summary, globalConfig);
        process.exit(exitCode);
      }
    } catch (error) {
      this.logger.logError(error instanceof Error ? error.message : String(error));
      process.exit(1);
    }
  }

  private async executeRequests(
    yamlFiles: string[],
    globalConfig: GlobalConfig,
  ): Promise<ExecutionSummary> {
    // Process YAML files and collect requests
    const fileGroups: Array<{ file: string; requests: RequestConfig[]; config?: GlobalConfig }> =
      [];
    const allRequests: RequestConfig[] = [];

    for (const file of yamlFiles) {
      const { requests, config } = await this.processYamlFile(file);

      const fileOutputConfig = config?.output || {};
      const requestsWithSourceConfig = requests.map((request) => ({
        ...request,
        sourceOutputConfig: fileOutputConfig,
        sourceFile: file,
      }));

      fileGroups.push({ file, requests: requestsWithSourceConfig, config });
      allRequests.push(...requestsWithSourceConfig);
    }

    const executor = new RequestExecutor(globalConfig);
    let summary: ExecutionSummary;

    // If multiple files, execute them with file separators for clarity
    if (fileGroups.length > 1) {
      const allResults: ExecutionResult[] = [];
      let totalDuration = 0;

      for (let i = 0; i < fileGroups.length; i++) {
        const group = fileGroups[i];

        // Show file header for better organization
        this.logger.logFileHeader(group.file, group.requests.length);

        const fileSummary = await executor.execute(group.requests);
        allResults.push(...fileSummary.results);
        totalDuration += fileSummary.duration;

        // Add spacing between files (except for the last one)
        if (i < fileGroups.length - 1) {
          console.log();
        }
      }

      // Create combined summary
      const successful = allResults.filter((r) => r.success).length;
      const failed = allResults.filter((r) => !r.success).length;

      summary = {
        total: allResults.length,
        successful,
        failed,
        duration: totalDuration,
        results: allResults,
      };

      // Show final summary
      this.logger.logSummary(summary, true);
    } else {
      // Single file - use normal execution
      summary = await executor.execute(allRequests);
    }

    return summary;
  }

  private parseArguments(args: string[]): { files: string[]; options: Record<string, unknown> } {
    const options: Record<string, unknown> = {};
    const files: string[] = [];

    for (let i = 0; i < args.length; i++) {
      const arg = args[i];

      if (arg.startsWith('--')) {
        const key = arg.slice(2);
        const nextArg = args[i + 1];

        if (key === 'help' || key === 'version') {
          options[key] = true;
        } else if (key === 'no-retry') {
          options.noRetry = true;
        } else if (key === 'quiet') {
          options.quiet = true;
        } else if (key === 'show-headers') {
          options.showHeaders = true;
        } else if (key === 'show-body') {
          options.showBody = true;
        } else if (key === 'show-metrics') {
          options.showMetrics = true;
        } else if (key === 'strict-exit') {
          options.strictExit = true;
        } else if (key === 'watch') {
          options.watch = true;
        } else if (key === 'watch-clear') {
          options.watchClear = true;
        } else if (key === 'no-watch-clear') {
          options.watchClear = false;
        } else if (nextArg && !nextArg.startsWith('--')) {
          if (key === 'continue-on-error') {
            options.continueOnError = nextArg === 'true';
          } else if (key === 'verbose') {
            options.verbose = nextArg === 'true';
          } else if (key === 'timeout') {
            options.timeout = Number.parseInt(nextArg, 10);
          } else if (key === 'retries') {
            options.retries = Number.parseInt(nextArg, 10);
          } else if (key === 'retry-delay') {
            options.retryDelay = Number.parseInt(nextArg, 10);
          } else if (key === 'max-concurrent') {
            const maxConcurrent = Number.parseInt(nextArg, 10);
            if (maxConcurrent > 0) {
              options.maxConcurrent = maxConcurrent;
            }
          } else if (key === 'fail-on') {
            options.failOn = Number.parseInt(nextArg, 10);
          } else if (key === 'fail-on-percentage') {
            const percentage = Number.parseFloat(nextArg);
            if (percentage >= 0 && percentage <= 100) {
              options.failOnPercentage = percentage;
            }
          } else if (key === 'output-format') {
            if (['json', 'pretty', 'raw'].includes(nextArg)) {
              options.outputFormat = nextArg;
            }
          } else if (key === 'pretty-level') {
            if (['minimal', 'standard', 'detailed'].includes(nextArg)) {
              options.prettyLevel = nextArg;
            }
          } else if (key === 'watch-debounce') {
            options.watchDebounce = Number.parseInt(nextArg, 10);
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
            case 'w':
              options.watch = true;
              break;
            case 'o': {
              // Handle -o flag for output file
              const outputArg = args[i + 1];
              if (outputArg && !outputArg.startsWith('-')) {
                options.output = outputArg;
                i++;
              }
              break;
            }
          }
        }
      } else {
        files.push(arg);
      }
    }

    return { files, options };
  }

  private async findYamlFiles(
    patterns: string[],
    options: Record<string, unknown>,
  ): Promise<string[]> {
    const files: Set<string> = new Set();

    let searchPatterns: string[] = [];

    if (patterns.length === 0) {
      searchPatterns = options.all ? ['**/*.yaml', '**/*.yml'] : ['*.yaml', '*.yml'];
    } else {
      // Check if patterns include directories
      for (const pattern of patterns) {
        try {
          // Use Bun's file system API to check if it's a directory
          const fs = await import('node:fs/promises');
          const stat = await fs.stat(pattern);

          if (stat.isDirectory()) {
            // Add glob patterns for all YAML files in this directory
            searchPatterns.push(`${pattern}/*.yaml`, `${pattern}/*.yml`);
            // If --all flag is set, search recursively
            if (options.all) {
              searchPatterns.push(`${pattern}/**/*.yaml`, `${pattern}/**/*.yml`);
            }
          } else if (stat.isFile()) {
            // It's a file, add it directly
            searchPatterns.push(pattern);
          }
        } catch {
          // If stat fails, assume it's a glob pattern
          searchPatterns.push(pattern);
        }
      }
    }

    for (const pattern of searchPatterns) {
      const globber = new Glob(pattern);
      for await (const file of globber.scan('.')) {
        // Only add files with .yaml or .yml extension
        if (file.endsWith('.yaml') || file.endsWith('.yml')) {
          files.add(file);
        }
      }
    }

    return Array.from(files).sort();
  }

  private async processYamlFile(
    filepath: string,
  ): Promise<{ requests: RequestConfig[]; config?: GlobalConfig }> {
    const yamlContent = await YamlParser.parseFile(filepath);
    const requests: RequestConfig[] = [];
    let globalConfig: GlobalConfig | undefined;

    if (yamlContent.global) {
      globalConfig = yamlContent.global;
    }

    const variables = {
      ...yamlContent.global?.variables,
      ...yamlContent.collection?.variables,
    };

    const defaults = {
      ...yamlContent.global?.defaults,
      ...yamlContent.collection?.defaults,
    };

    if (yamlContent.request) {
      const request = this.prepareRequest(yamlContent.request, variables, defaults);
      requests.push(request);
    }

    if (yamlContent.requests) {
      for (const req of yamlContent.requests) {
        const request = this.prepareRequest(req, variables, defaults);
        requests.push(request);
      }
    }

    if (yamlContent.collection?.requests) {
      for (const req of yamlContent.collection.requests) {
        const request = this.prepareRequest(req, variables, defaults);
        requests.push(request);
      }
    }

    return { requests, config: globalConfig };
  }

  private prepareRequest(
    request: RequestConfig,
    variables: Record<string, string>,
    defaults: Partial<RequestConfig>,
  ): RequestConfig {
    const interpolated = YamlParser.interpolateVariables(request, variables);
    return YamlParser.mergeConfigs(defaults, interpolated);
  }

  private mergeGlobalConfigs(base: GlobalConfig, override: GlobalConfig): GlobalConfig {
    return {
      ...base,
      ...override,
      variables: { ...base.variables, ...override.variables },
      output: { ...base.output, ...override.output },
      defaults: { ...base.defaults, ...override.defaults },
      ci: { ...base.ci, ...override.ci },
      watch: { ...base.watch, ...override.watch },
    };
  }

  /**
   * Determines the appropriate exit code based on execution results and CI configuration.
   *
   * Exit code logic:
   * - If strictExit is enabled: exit 1 if ANY failures occur
   * - If failOn is set: exit 1 if failures exceed the threshold
   * - If failOnPercentage is set: exit 1 if failure percentage exceeds the threshold
   * - Default behavior: exit 1 only if failures exist AND continueOnError is false
   *
   * @param summary - The execution summary containing success/failure counts
   * @param config - Global configuration including CI exit options
   * @returns 0 for success, 1 for failure
   */
  private determineExitCode(summary: ExecutionSummary, config: GlobalConfig): number {
    const { failed, total } = summary;
    const ci = config.ci;

    // If no failures, always exit with 0
    if (failed === 0) {
      return 0;
    }

    // Check CI exit code options
    if (ci) {
      // strictExit: exit 1 if ANY failures occur
      if (ci.strictExit) {
        return 1;
      }

      // failOn: exit 1 if failures exceed the threshold
      if (ci.failOn !== undefined && failed > ci.failOn) {
        return 1;
      }

      // failOnPercentage: exit 1 if failure percentage exceeds the threshold
      if (ci.failOnPercentage !== undefined && total > 0) {
        const failurePercentage = (failed / total) * 100;
        if (failurePercentage > ci.failOnPercentage) {
          return 1;
        }
      }

      // If any CI option is set but thresholds not exceeded, exit 0
      if (ci.failOn !== undefined || ci.failOnPercentage !== undefined) {
        return 0;
      }
    }

    // Default behavior: exit 1 if failures AND continueOnError is false
    return !config.continueOnError ? 1 : 0;
  }

  private showHelp(): void {
    console.log(`
${this.logger.color('🚀 CURL RUNNER', 'bright')}

${this.logger.color('USAGE:', 'yellow')}
  curl-runner [files...] [options]

${this.logger.color('OPTIONS:', 'yellow')}
  -h, --help                    Show this help message
  -v, --verbose                 Enable verbose output
  -q, --quiet                   Suppress non-error output
  -p, --execution parallel      Execute requests in parallel
  --max-concurrent <n>          Limit concurrent requests in parallel mode
  -c, --continue-on-error       Continue execution on errors
  -o, --output <file>           Save results to file
  --all                         Find all YAML files recursively
  --timeout <ms>                Set request timeout in milliseconds
  --retries <count>             Set maximum retry attempts
  --retry-delay <ms>            Set delay between retries in milliseconds
  --no-retry                    Disable retry mechanism
  --output-format <format>      Set output format (json|pretty|raw)
  --pretty-level <level>        Set pretty format level (minimal|standard|detailed)
  --show-headers                Include response headers in output
  --show-body                   Include response body in output
  --show-metrics                Include performance metrics in output
  --version                     Show version

${this.logger.color('WATCH MODE:', 'yellow')}
  -w, --watch                   Watch files and re-run on changes
  --watch-debounce <ms>         Debounce delay for watch mode (default: 300)
  --no-watch-clear              Don't clear screen between watch runs

${this.logger.color('CI/CD OPTIONS:', 'yellow')}
  --strict-exit                 Exit with code 1 if any validation fails (for CI/CD)
  --fail-on <count>             Exit with code 1 if failures exceed this count
  --fail-on-percentage <pct>    Exit with code 1 if failure percentage exceeds this value

${this.logger.color('EXAMPLES:', 'yellow')}
  # Run all YAML files in current directory
  curl-runner

  # Run specific file
  curl-runner api-tests.yaml

  # Run all files in a directory
  curl-runner examples/
  
  # Run all files in multiple directories
  curl-runner tests/ examples/

  # Run all files recursively in parallel
  curl-runner --all -p

  # Run in parallel with max 5 concurrent requests
  curl-runner -p --max-concurrent 5 tests.yaml

  # Run directory recursively
  curl-runner --all examples/

  # Run with verbose output and continue on errors
  curl-runner tests/*.yaml -vc
  
  # Run with minimal pretty output (only status and errors)
  curl-runner --output-format pretty --pretty-level minimal test.yaml
  
  # Run with detailed pretty output (show all information)
  curl-runner --output-format pretty --pretty-level detailed test.yaml

  # CI/CD: Fail if any validation fails (strict mode)
  curl-runner tests/ --strict-exit

  # CI/CD: Run all tests but fail if any validation fails
  curl-runner tests/ --continue-on-error --strict-exit

  # CI/CD: Allow up to 2 failures
  curl-runner tests/ --fail-on 2

  # CI/CD: Allow up to 10% failures
  curl-runner tests/ --fail-on-percentage 10

  # Watch mode - re-run on file changes
  curl-runner api.yaml --watch

  # Watch with custom debounce
  curl-runner tests/ -w --watch-debounce 500

${this.logger.color('YAML STRUCTURE:', 'yellow')}
  Single request:
    request:
      url: https://api.example.com
      method: GET

  Multiple requests:
    requests:
      - url: https://api.example.com/users
        method: GET
      - url: https://api.example.com/posts
        method: POST
        body: { title: "Test" }

  With global config:
    global:
      execution: parallel
      variables:
        BASE_URL: https://api.example.com
    requests:
      - url: \${BASE_URL}/users
        method: GET
`);
  }
}

const cli = new CurlRunnerCLI();
cli.run(process.argv.slice(2));
