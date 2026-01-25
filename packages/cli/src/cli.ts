#!/usr/bin/env bun

import { Glob } from 'bun';
import { showUpgradeHelp, UpgradeCommand } from './commands/upgrade';
import { BaselineManager, DiffFormatter, DiffOrchestrator } from './diff';
import { ProfileExecutor } from './executor/profile-executor';
import { RequestExecutor } from './executor/request-executor';
import { YamlParser } from './parser/yaml';
import type {
  DiffConfig,
  ExecutionResult,
  ExecutionSummary,
  GlobalConfig,
  GlobalDiffConfig,
  ProfileConfig,
  RequestConfig,
  WatchConfig,
} from './types/config';
import { Logger } from './utils/logger';
import { exportToCSV, exportToJSON } from './utils/stats';
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
          const config = yamlContent.global || {};
          this.logger.logInfo(`Loaded configuration from ${filename}`);
          return config as Partial<GlobalConfig>;
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

    // Profile mode configuration
    if (process.env.CURL_RUNNER_PROFILE) {
      const iterations = Number.parseInt(process.env.CURL_RUNNER_PROFILE, 10);
      if (iterations > 0) {
        envConfig.profile = {
          ...envConfig.profile,
          iterations,
        };
      }
    }

    if (process.env.CURL_RUNNER_PROFILE_WARMUP) {
      envConfig.profile = {
        ...envConfig.profile,
        iterations: envConfig.profile?.iterations ?? 10,
        warmup: Number.parseInt(process.env.CURL_RUNNER_PROFILE_WARMUP, 10),
      };
    }

    if (process.env.CURL_RUNNER_PROFILE_CONCURRENCY) {
      envConfig.profile = {
        ...envConfig.profile,
        iterations: envConfig.profile?.iterations ?? 10,
        concurrency: Number.parseInt(process.env.CURL_RUNNER_PROFILE_CONCURRENCY, 10),
      };
    }

    if (process.env.CURL_RUNNER_PROFILE_HISTOGRAM) {
      envConfig.profile = {
        ...envConfig.profile,
        iterations: envConfig.profile?.iterations ?? 10,
        histogram: process.env.CURL_RUNNER_PROFILE_HISTOGRAM.toLowerCase() === 'true',
      };
    }

    if (process.env.CURL_RUNNER_PROFILE_EXPORT) {
      envConfig.profile = {
        ...envConfig.profile,
        iterations: envConfig.profile?.iterations ?? 10,
        exportFile: process.env.CURL_RUNNER_PROFILE_EXPORT,
      };
    }

    // Snapshot configuration
    if (process.env.CURL_RUNNER_SNAPSHOT) {
      envConfig.snapshot = {
        ...envConfig.snapshot,
        enabled: process.env.CURL_RUNNER_SNAPSHOT.toLowerCase() === 'true',
      };
    }

    if (process.env.CURL_RUNNER_SNAPSHOT_UPDATE) {
      const mode = process.env.CURL_RUNNER_SNAPSHOT_UPDATE.toLowerCase();
      if (['none', 'all', 'failing'].includes(mode)) {
        envConfig.snapshot = {
          ...envConfig.snapshot,
          updateMode: mode as 'none' | 'all' | 'failing',
        };
      }
    }

    if (process.env.CURL_RUNNER_SNAPSHOT_DIR) {
      envConfig.snapshot = {
        ...envConfig.snapshot,
        dir: process.env.CURL_RUNNER_SNAPSHOT_DIR,
      };
    }

    if (process.env.CURL_RUNNER_SNAPSHOT_CI) {
      envConfig.snapshot = {
        ...envConfig.snapshot,
        ci: process.env.CURL_RUNNER_SNAPSHOT_CI.toLowerCase() === 'true',
      };
    }

    // Diff configuration
    if (process.env.CURL_RUNNER_DIFF) {
      envConfig.diff = {
        ...envConfig.diff,
        enabled: process.env.CURL_RUNNER_DIFF.toLowerCase() === 'true',
      };
    }

    if (process.env.CURL_RUNNER_DIFF_SAVE) {
      envConfig.diff = {
        ...envConfig.diff,
        save: process.env.CURL_RUNNER_DIFF_SAVE.toLowerCase() === 'true',
      };
    }

    if (process.env.CURL_RUNNER_DIFF_LABEL) {
      envConfig.diff = {
        ...envConfig.diff,
        label: process.env.CURL_RUNNER_DIFF_LABEL,
      };
    }

    if (process.env.CURL_RUNNER_DIFF_COMPARE) {
      envConfig.diff = {
        ...envConfig.diff,
        compareWith: process.env.CURL_RUNNER_DIFF_COMPARE,
      };
    }

    if (process.env.CURL_RUNNER_DIFF_DIR) {
      envConfig.diff = {
        ...envConfig.diff,
        dir: process.env.CURL_RUNNER_DIFF_DIR,
      };
    }

    if (process.env.CURL_RUNNER_DIFF_OUTPUT) {
      const format = process.env.CURL_RUNNER_DIFF_OUTPUT.toLowerCase();
      if (['terminal', 'json', 'markdown'].includes(format)) {
        envConfig.diff = {
          ...envConfig.diff,
          outputFormat: format as 'terminal' | 'json' | 'markdown',
        };
      }
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

      // Handle upgrade subcommand: curl-runner upgrade [options]
      if (args[0] === 'upgrade') {
        if (args.includes('--help') || args.includes('-h')) {
          showUpgradeHelp();
          return;
        }
        const upgradeCmd = new UpgradeCommand();
        await upgradeCmd.run(args.slice(1));
        return;
      }

      // Handle diff subcommand: curl-runner diff <label1> <label2> [file]
      if (args[0] === 'diff' && args.length >= 3) {
        await this.executeDiffSubcommand(args.slice(1), options);
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
        globalConfig.continueOnError = options.continueOnError as boolean;
      }
      if (options.verbose !== undefined) {
        globalConfig.output = { ...globalConfig.output, verbose: options.verbose as boolean };
      }
      if (options.quiet !== undefined) {
        globalConfig.output = { ...globalConfig.output, verbose: false };
      }
      if (options.output) {
        globalConfig.output = { ...globalConfig.output, saveToFile: options.output as string };
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
        globalConfig.output = {
          ...globalConfig.output,
          showHeaders: options.showHeaders as boolean,
        };
      }
      if (options.showBody !== undefined) {
        globalConfig.output = { ...globalConfig.output, showBody: options.showBody as boolean };
      }
      if (options.showMetrics !== undefined) {
        globalConfig.output = {
          ...globalConfig.output,
          showMetrics: options.showMetrics as boolean,
        };
      }

      // Apply timeout and retry settings to defaults
      if (options.timeout) {
        globalConfig.defaults = { ...globalConfig.defaults, timeout: options.timeout as number };
      }
      if (options.retries || options.noRetry) {
        const retryCount = options.noRetry ? 0 : (options.retries as number) || 0;
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
            delay: options.retryDelay as number,
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

      // Apply snapshot options
      if (options.snapshot !== undefined) {
        globalConfig.snapshot = {
          ...globalConfig.snapshot,
          enabled: options.snapshot as boolean,
        };
      }
      if (options.snapshotUpdate !== undefined) {
        globalConfig.snapshot = {
          ...globalConfig.snapshot,
          enabled: true,
          updateMode: options.snapshotUpdate as 'none' | 'all' | 'failing',
        };
      }
      if (options.snapshotDir !== undefined) {
        globalConfig.snapshot = {
          ...globalConfig.snapshot,
          dir: options.snapshotDir as string,
        };
      }
      if (options.snapshotCi !== undefined) {
        globalConfig.snapshot = {
          ...globalConfig.snapshot,
          ci: options.snapshotCi as boolean,
        };
      }

      // Apply diff options
      if (options.diff !== undefined) {
        globalConfig.diff = {
          ...globalConfig.diff,
          enabled: options.diff as boolean,
        };
      }
      if (options.diffSave !== undefined) {
        globalConfig.diff = {
          ...globalConfig.diff,
          enabled: true,
          save: options.diffSave as boolean,
        };
      }
      if (options.diffLabel !== undefined) {
        globalConfig.diff = {
          ...globalConfig.diff,
          label: options.diffLabel as string,
        };
      }
      if (options.diffCompare !== undefined) {
        globalConfig.diff = {
          ...globalConfig.diff,
          enabled: true,
          compareWith: options.diffCompare as string,
        };
      }
      if (options.diffDir !== undefined) {
        globalConfig.diff = {
          ...globalConfig.diff,
          dir: options.diffDir as string,
        };
      }
      if (options.diffOutput !== undefined) {
        globalConfig.diff = {
          ...globalConfig.diff,
          outputFormat: options.diffOutput as 'terminal' | 'json' | 'markdown',
        };
      }

      if (allRequests.length === 0) {
        this.logger.logError('No requests found in YAML files');
        process.exit(1);
      }

      // Check if watch mode is enabled
      const watchEnabled = options.watch || globalConfig.watch?.enabled;

      // Check if profile mode is enabled (mutually exclusive with watch mode)
      const profileIterations =
        (options.profile as number | undefined) ?? globalConfig.profile?.iterations;
      const profileEnabled = profileIterations && profileIterations > 0;

      if (watchEnabled && profileEnabled) {
        this.logger.logError('Profile mode and watch mode cannot be used together');
        process.exit(1);
      }

      if (profileEnabled) {
        // Profile mode - run requests multiple times for latency stats
        const profileConfig: ProfileConfig = {
          iterations: profileIterations,
          warmup:
            (options.profileWarmup as number | undefined) ?? globalConfig.profile?.warmup ?? 1,
          concurrency:
            (options.profileConcurrency as number | undefined) ??
            globalConfig.profile?.concurrency ??
            1,
          histogram:
            (options.profileHistogram as boolean | undefined) ??
            globalConfig.profile?.histogram ??
            false,
          exportFile:
            (options.profileExport as string | undefined) ?? globalConfig.profile?.exportFile,
        };

        await this.executeProfileMode(allRequests, globalConfig, profileConfig);
      } else if (watchEnabled) {
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

  private async executeProfileMode(
    requests: RequestConfig[],
    globalConfig: GlobalConfig,
    profileConfig: ProfileConfig,
  ): Promise<void> {
    const profileExecutor = new ProfileExecutor(globalConfig, profileConfig);
    const results = await profileExecutor.profileRequests(requests);

    this.logger.logProfileSummary(results);

    // Export results if requested
    if (profileConfig.exportFile) {
      const exportData: string[] = [];
      const isCSV = profileConfig.exportFile.endsWith('.csv');

      for (const result of results) {
        const name = result.request.name || result.request.url;
        if (isCSV) {
          exportData.push(exportToCSV(result.stats, name));
        } else {
          exportData.push(exportToJSON(result.stats, name));
        }
      }

      const content = isCSV ? exportData.join('\n\n') : `[${exportData.join(',\n')}]`;
      await Bun.write(profileConfig.exportFile, content);
      this.logger.logInfo(`Profile results exported to ${profileConfig.exportFile}`);
    }

    // Exit with code 1 if failure rate is high
    const totalFailures = results.reduce((sum, r) => sum + r.stats.failures, 0);
    const totalIterations = results.reduce(
      (sum, r) => sum + r.stats.iterations + r.stats.warmup,
      0,
    );

    if (totalFailures > 0 && totalFailures / totalIterations > 0.5) {
      process.exit(1);
    }

    process.exit(0);
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
      const successful = allResults.filter((r) => r.success && !r.skipped).length;
      const failed = allResults.filter((r) => !r.success && !r.skipped).length;
      const skipped = allResults.filter((r) => r.skipped).length;

      summary = {
        total: allResults.length,
        successful,
        failed,
        skipped,
        duration: totalDuration,
        results: allResults,
      };

      // Show final summary
      this.logger.logSummary(summary, true);
    } else {
      // Single file - use normal execution
      summary = await executor.execute(allRequests);
    }

    // Handle diff mode
    if (globalConfig.diff?.enabled || globalConfig.diff?.save || globalConfig.diff?.compareWith) {
      await this.handleDiffMode(yamlFiles[0], summary.results, globalConfig.diff);
    }

    return summary;
  }

  private async handleDiffMode(
    yamlPath: string,
    results: ExecutionResult[],
    diffConfig: GlobalDiffConfig,
  ): Promise<void> {
    const orchestrator = new DiffOrchestrator(diffConfig);
    const formatter = new DiffFormatter(diffConfig.outputFormat || 'terminal');
    const config: DiffConfig = BaselineManager.mergeConfig(diffConfig, true) || {};

    const currentLabel = diffConfig.label || 'current';
    const compareLabel = diffConfig.compareWith;

    // Save baseline if requested
    if (diffConfig.save) {
      await orchestrator.saveBaseline(yamlPath, currentLabel, results, config);
      this.logger.logInfo(`Baseline saved as '${currentLabel}'`);
    }

    // Compare with baseline if requested
    if (compareLabel) {
      const diffSummary = await orchestrator.compareWithBaseline(
        yamlPath,
        results,
        currentLabel,
        compareLabel,
        config,
      );

      // Check if baseline exists
      if (diffSummary.newBaselines === diffSummary.totalRequests) {
        this.logger.logWarning(
          `No baseline '${compareLabel}' found. Saving current run as baseline.`,
        );
        await orchestrator.saveBaseline(yamlPath, compareLabel, results, config);
        return;
      }

      const output = formatter.formatSummary(diffSummary, compareLabel, currentLabel);
      console.log(output);

      // Save current as baseline if configured
      if (diffConfig.save) {
        await orchestrator.saveBaseline(yamlPath, currentLabel, results, config);
      }
    } else if (diffConfig.enabled && !diffConfig.save) {
      // Auto-detect: list available baselines or save first baseline
      const labels = await orchestrator.listLabels(yamlPath);

      if (labels.length === 0) {
        // No baselines exist - save current as default baseline
        await orchestrator.saveBaseline(yamlPath, 'baseline', results, config);
        this.logger.logInfo(`No baselines found. Saved current run as 'baseline'.`);
      } else if (labels.length === 1) {
        // One baseline exists - compare against it
        const diffSummary = await orchestrator.compareWithBaseline(
          yamlPath,
          results,
          currentLabel,
          labels[0],
          config,
        );
        const output = formatter.formatSummary(diffSummary, labels[0], currentLabel);
        console.log(output);
      } else {
        // Multiple baselines - list them
        this.logger.logInfo(`Available baselines: ${labels.join(', ')}`);
        this.logger.logInfo(`Use --diff-compare <label> to compare against a specific baseline.`);
      }
    }
  }

  /**
   * Executes the diff subcommand to compare two stored baselines.
   * Usage: curl-runner diff <label1> <label2> [file.yaml]
   */
  private async executeDiffSubcommand(
    args: string[],
    options: Record<string, unknown>,
  ): Promise<void> {
    const label1 = args[0];
    const label2 = args[1];
    let yamlFile = args[2];

    // Find YAML file if not specified
    if (!yamlFile) {
      const yamlFiles = await this.findYamlFiles([], options);
      if (yamlFiles.length === 0) {
        this.logger.logError(
          'No YAML files found. Specify a file: curl-runner diff <label1> <label2> <file.yaml>',
        );
        process.exit(1);
      }
      if (yamlFiles.length > 1) {
        this.logger.logError('Multiple YAML files found. Specify which file to use.');
        process.exit(1);
      }
      yamlFile = yamlFiles[0];
    }

    const diffConfig: GlobalDiffConfig = {
      dir: (options.diffDir as string) || '__baselines__',
      outputFormat: (options.diffOutput as 'terminal' | 'json' | 'markdown') || 'terminal',
    };

    const orchestrator = new DiffOrchestrator(diffConfig);
    const formatter = new DiffFormatter(diffConfig.outputFormat || 'terminal');
    const config: DiffConfig = { exclude: [], match: {} };

    try {
      const diffSummary = await orchestrator.compareTwoBaselines(yamlFile, label1, label2, config);
      const output = formatter.formatSummary(diffSummary, label1, label2);
      console.log(output);

      // Exit with code 1 if differences found
      if (diffSummary.changed > 0) {
        process.exit(1);
      }
      process.exit(0);
    } catch (error) {
      this.logger.logError(error instanceof Error ? error.message : String(error));
      process.exit(1);
    }
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
        } else if (key === 'profile-histogram') {
          options.profileHistogram = true;
        } else if (key === 'snapshot') {
          options.snapshot = true;
        } else if (key === 'update-snapshots') {
          options.snapshotUpdate = 'all';
        } else if (key === 'update-failing') {
          options.snapshotUpdate = 'failing';
        } else if (key === 'ci-snapshot') {
          options.snapshotCi = true;
        } else if (key === 'diff') {
          options.diff = true;
        } else if (key === 'diff-save') {
          options.diffSave = true;
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
          } else if (key === 'profile') {
            options.profile = Number.parseInt(nextArg, 10);
          } else if (key === 'profile-warmup') {
            options.profileWarmup = Number.parseInt(nextArg, 10);
          } else if (key === 'profile-concurrency') {
            options.profileConcurrency = Number.parseInt(nextArg, 10);
          } else if (key === 'profile-export') {
            options.profileExport = nextArg;
          } else if (key === 'snapshot-dir') {
            options.snapshotDir = nextArg;
          } else if (key === 'diff-label') {
            options.diffLabel = nextArg;
          } else if (key === 'diff-compare') {
            options.diffCompare = nextArg;
          } else if (key === 'diff-dir') {
            options.diffDir = nextArg;
          } else if (key === 'diff-output') {
            if (['terminal', 'json', 'markdown'].includes(nextArg)) {
              options.diffOutput = nextArg;
            }
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
            case 's':
              options.snapshot = true;
              break;
            case 'u':
              options.snapshotUpdate = 'all';
              break;
            case 'd':
              options.diff = true;
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
            case 'P': {
              // Handle -P flag for profile mode
              const profileArg = args[i + 1];
              if (profileArg && !profileArg.startsWith('-')) {
                options.profile = Number.parseInt(profileArg, 10);
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
    const interpolated = YamlParser.interpolateVariables(request, variables) as RequestConfig;
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
      snapshot: { ...base.snapshot, ...override.snapshot },
      diff: { ...base.diff, ...override.diff },
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

${this.logger.color('PROFILE MODE:', 'yellow')}
  -P, --profile <n>             Run each request N times for latency stats
  --profile-warmup <n>          Warmup iterations to exclude from stats (default: 1)
  --profile-concurrency <n>     Concurrent iterations (default: 1 = sequential)
  --profile-histogram           Show ASCII histogram of latency distribution
  --profile-export <file>       Export raw timings to file (.json or .csv)

${this.logger.color('CI/CD OPTIONS:', 'yellow')}
  --strict-exit                 Exit with code 1 if any validation fails (for CI/CD)
  --fail-on <count>             Exit with code 1 if failures exceed this count
  --fail-on-percentage <pct>    Exit with code 1 if failure percentage exceeds this value

${this.logger.color('SNAPSHOT OPTIONS:', 'yellow')}
  -s, --snapshot                Enable snapshot testing
  -u, --update-snapshots        Update all snapshots
  --update-failing              Update only failing snapshots
  --snapshot-dir <dir>          Custom snapshot directory (default: __snapshots__)
  --ci-snapshot                 Fail if snapshot is missing (CI mode)

${this.logger.color('DIFF OPTIONS:', 'yellow')}
  -d, --diff                    Enable response diffing (compare with baseline)
  --diff-save                   Save current run as baseline
  --diff-label <name>           Label for current run (e.g., 'staging', 'v1.0')
  --diff-compare <label>        Compare against this baseline label
  --diff-dir <dir>              Baseline storage directory (default: __baselines__)
  --diff-output <format>        Output format (terminal|json|markdown)

${this.logger.color('DIFF SUBCOMMAND:', 'yellow')}
  curl-runner diff <label1> <label2> [file.yaml]
                                Compare two stored baselines without making requests

${this.logger.color('UPGRADE:', 'yellow')}
  curl-runner upgrade           Upgrade to latest version (auto-detects install method)
  curl-runner upgrade --dry-run Preview upgrade command without executing
  curl-runner upgrade --force   Force reinstall even if up to date

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

  # Profile mode - run request 100 times for latency stats
  curl-runner api.yaml -P 100

  # Profile with 5 warmup iterations and histogram
  curl-runner api.yaml --profile 50 --profile-warmup 5 --profile-histogram

  # Profile with concurrent iterations and export
  curl-runner api.yaml -P 100 --profile-concurrency 10 --profile-export results.json

  # Snapshot testing - save and compare responses
  curl-runner api.yaml --snapshot

  # Update all snapshots
  curl-runner api.yaml -su

  # CI mode - fail if snapshot missing
  curl-runner api.yaml --snapshot --ci-snapshot

  # Response diffing - save baseline for staging
  curl-runner api.yaml --diff-save --diff-label staging

  # Compare current run against staging baseline
  curl-runner api.yaml --diff --diff-compare staging

  # Compare staging vs production baselines (offline)
  curl-runner diff staging production api.yaml

  # Auto-diff: creates baseline on first run, compares on subsequent runs
  curl-runner api.yaml --diff

  # Diff with JSON output for CI
  curl-runner api.yaml --diff --diff-compare staging --diff-output json

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
