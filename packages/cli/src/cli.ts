#!/usr/bin/env bun

import { Glob } from 'bun';
import { showUpgradeHelp, UpgradeCommand } from './commands/upgrade';
import {
  buildProfileConfig,
  buildWatchConfig,
  type CLIOptions,
  mergeGlobalConfigs,
  resolveConfig,
} from './core/config';
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
} from './types/config';
import { Logger } from './utils/logger';
import { exportToCSV, exportToJSON } from './utils/stats';
import { VersionChecker } from './utils/version-checker';
import { getVersion } from './version';
import { FileWatcher } from './watcher/file-watcher';

class CurlRunnerCLI {
  private logger = new Logger();

  async run(args: string[]): Promise<void> {
    try {
      // Resolve config from all sources (CLI, env, file)
      const resolved = await resolveConfig(args, {
        onInfo: (msg) => this.logger.logInfo(msg),
        onWarning: (msg) => this.logger.logWarning(msg),
      });

      const { config, cliOptions, mode, rawArgs } = resolved;

      // Check for updates in background (non-blocking)
      if (mode !== 'version' && mode !== 'help') {
        new VersionChecker().checkForUpdates().catch(() => {});
      }

      // Handle execution modes
      switch (mode) {
        case 'help':
          this.showHelp();
          return;

        case 'version':
          console.log(`curl-runner v${getVersion()}`);
          return;

        case 'upgrade':
          if (rawArgs.includes('--help') || rawArgs.includes('-h')) {
            showUpgradeHelp();
            return;
          }
          await new UpgradeCommand().run(rawArgs.slice(1));
          return;

        case 'diff-subcommand':
          await this.executeDiffSubcommand(rawArgs.slice(1), cliOptions);
          return;

        case 'profile':
        case 'watch':
        case 'normal':
          await this.executeMain(cliOptions, config, mode);
          return;
      }
    } catch (error) {
      this.logger.logError(error instanceof Error ? error.message : String(error));
      process.exit(1);
    }
  }

  private async executeMain(
    cliOptions: CLIOptions,
    globalConfig: GlobalConfig,
    mode: 'normal' | 'watch' | 'profile',
  ): Promise<void> {
    // Find YAML files
    const yamlFiles = await this.findYamlFiles(cliOptions.files, cliOptions.all);

    if (yamlFiles.length === 0) {
      this.logger.logError('No YAML files found');
      process.exit(1);
    }

    this.logger.logInfo(`Found ${yamlFiles.length} YAML file(s)`);

    // Process YAML files and collect requests
    const allRequests: RequestConfig[] = [];
    let mergedConfig = globalConfig;

    for (const file of yamlFiles) {
      this.logger.logInfo(`Processing: ${file}`);
      const { requests, config } = await this.processYamlFile(file);

      const fileOutputConfig = config?.output || {};
      const requestsWithSource = requests.map((request) => ({
        ...request,
        sourceOutputConfig: fileOutputConfig,
        sourceFile: file,
      }));

      if (config) {
        mergedConfig = mergeGlobalConfigs(mergedConfig, config);
      }

      allRequests.push(...requestsWithSource);
    }

    if (allRequests.length === 0) {
      this.logger.logError('No requests found in YAML files');
      process.exit(1);
    }

    // Execute based on mode
    if (mode === 'profile') {
      const profileConfig = buildProfileConfig(cliOptions, mergedConfig);
      await this.executeProfileMode(allRequests, mergedConfig, profileConfig);
    } else if (mode === 'watch') {
      const watchConfig = buildWatchConfig(cliOptions, mergedConfig);
      const watcher = new FileWatcher({
        files: yamlFiles,
        config: watchConfig,
        logger: this.logger,
        onRun: async () => {
          await this.executeRequests(yamlFiles, mergedConfig);
        },
      });
      await watcher.start();
    } else {
      const summary = await this.executeRequests(yamlFiles, mergedConfig);
      const exitCode = this.determineExitCode(summary, mergedConfig);
      process.exit(exitCode);
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

    if (profileConfig.exportFile) {
      const exportData: string[] = [];
      const isCSV = profileConfig.exportFile.endsWith('.csv');

      for (const result of results) {
        const name = result.request.name || result.request.url;
        exportData.push(isCSV ? exportToCSV(result.stats, name) : exportToJSON(result.stats, name));
      }

      const content = isCSV ? exportData.join('\n\n') : `[${exportData.join(',\n')}]`;
      await Bun.write(profileConfig.exportFile, content);
      this.logger.logInfo(`Profile results exported to ${profileConfig.exportFile}`);
    }

    const totalFailures = results.reduce((sum, r) => sum + r.stats.failures, 0);
    const totalIterations = results.reduce(
      (sum, r) => sum + r.stats.iterations + r.stats.warmup,
      0,
    );

    process.exit(totalFailures > 0 && totalFailures / totalIterations > 0.5 ? 1 : 0);
  }

  private async executeRequests(
    yamlFiles: string[],
    globalConfig: GlobalConfig,
  ): Promise<ExecutionSummary> {
    const fileGroups: Array<{ file: string; requests: RequestConfig[]; config?: GlobalConfig }> =
      [];
    const allRequests: RequestConfig[] = [];

    for (const file of yamlFiles) {
      const { requests, config } = await this.processYamlFile(file);

      const fileOutputConfig = config?.output || {};
      const requestsWithSource = requests.map((request) => ({
        ...request,
        sourceOutputConfig: fileOutputConfig,
        sourceFile: file,
      }));

      fileGroups.push({ file, requests: requestsWithSource, config });
      allRequests.push(...requestsWithSource);
    }

    const executor = new RequestExecutor(globalConfig);
    let summary: ExecutionSummary;

    if (fileGroups.length > 1) {
      const allResults: ExecutionResult[] = [];
      let totalDuration = 0;

      for (let i = 0; i < fileGroups.length; i++) {
        const group = fileGroups[i];
        this.logger.logFileHeader(group.file, group.requests.length);

        const fileSummary = await executor.execute(group.requests);
        allResults.push(...fileSummary.results);
        totalDuration += fileSummary.duration;

        if (i < fileGroups.length - 1) {
          console.log();
        }
      }

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
      this.logger.logSummary(summary, true);
    } else {
      summary = await executor.execute(allRequests);
    }

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

    if (diffConfig.save) {
      await orchestrator.saveBaseline(yamlPath, currentLabel, results, config);
      this.logger.logInfo(`Baseline saved as '${currentLabel}'`);
    }

    if (compareLabel) {
      const diffSummary = await orchestrator.compareWithBaseline(
        yamlPath,
        results,
        currentLabel,
        compareLabel,
        config,
      );

      if (diffSummary.newBaselines === diffSummary.totalRequests) {
        this.logger.logWarning(
          `No baseline '${compareLabel}' found. Saving current run as baseline.`,
        );
        await orchestrator.saveBaseline(yamlPath, compareLabel, results, config);
        return;
      }

      console.log(formatter.formatSummary(diffSummary, compareLabel, currentLabel));

      if (diffConfig.save) {
        await orchestrator.saveBaseline(yamlPath, currentLabel, results, config);
      }
    } else if (diffConfig.enabled && !diffConfig.save) {
      const labels = await orchestrator.listLabels(yamlPath);

      if (labels.length === 0) {
        await orchestrator.saveBaseline(yamlPath, 'baseline', results, config);
        this.logger.logInfo(`No baselines found. Saved current run as 'baseline'.`);
      } else if (labels.length === 1) {
        const diffSummary = await orchestrator.compareWithBaseline(
          yamlPath,
          results,
          currentLabel,
          labels[0],
          config,
        );
        console.log(formatter.formatSummary(diffSummary, labels[0], currentLabel));
      } else {
        this.logger.logInfo(`Available baselines: ${labels.join(', ')}`);
        this.logger.logInfo(`Use --diff-compare <label> to compare against a specific baseline.`);
      }
    }
  }

  private async executeDiffSubcommand(args: string[], options: CLIOptions): Promise<void> {
    const label1 = args[0];
    const label2 = args[1];
    let yamlFile = args[2];

    if (!yamlFile) {
      const yamlFiles = await this.findYamlFiles([], options.all);
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
      dir: options.diffDir || '__baselines__',
      outputFormat: options.diffOutput || 'terminal',
    };

    const orchestrator = new DiffOrchestrator(diffConfig);
    const formatter = new DiffFormatter(diffConfig.outputFormat || 'terminal');
    const config: DiffConfig = { exclude: [], match: {} };

    try {
      const diffSummary = await orchestrator.compareTwoBaselines(yamlFile, label1, label2, config);
      console.log(formatter.formatSummary(diffSummary, label1, label2));
      process.exit(diffSummary.changed > 0 ? 1 : 0);
    } catch (error) {
      this.logger.logError(error instanceof Error ? error.message : String(error));
      process.exit(1);
    }
  }

  private async findYamlFiles(patterns: string[], all?: boolean): Promise<string[]> {
    const files: Set<string> = new Set();
    let searchPatterns: string[] = [];

    if (patterns.length === 0) {
      searchPatterns = all ? ['**/*.yaml', '**/*.yml'] : ['*.yaml', '*.yml'];
    } else {
      for (const pattern of patterns) {
        try {
          const fs = await import('node:fs/promises');
          const stat = await fs.stat(pattern);

          if (stat.isDirectory()) {
            searchPatterns.push(`${pattern}/*.yaml`, `${pattern}/*.yml`);
            if (all) {
              searchPatterns.push(`${pattern}/**/*.yaml`, `${pattern}/**/*.yml`);
            }
          } else if (stat.isFile()) {
            searchPatterns.push(pattern);
          }
        } catch {
          searchPatterns.push(pattern);
        }
      }
    }

    for (const pattern of searchPatterns) {
      const globber = new Glob(pattern);
      for await (const file of globber.scan('.')) {
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

    const variables = { ...yamlContent.global?.variables, ...yamlContent.collection?.variables };
    const defaults = { ...yamlContent.global?.defaults, ...yamlContent.collection?.defaults };

    if (yamlContent.request) {
      requests.push(this.prepareRequest(yamlContent.request, variables, defaults));
    }

    if (yamlContent.requests) {
      for (const req of yamlContent.requests) {
        requests.push(this.prepareRequest(req, variables, defaults));
      }
    }

    if (yamlContent.collection?.requests) {
      for (const req of yamlContent.collection.requests) {
        requests.push(this.prepareRequest(req, variables, defaults));
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

  private determineExitCode(summary: ExecutionSummary, config: GlobalConfig): number {
    const { failed, total } = summary;
    const ci = config.ci;

    if (failed === 0) {
      return 0;
    }

    if (ci) {
      if (ci.strictExit) {
        return 1;
      }
      if (ci.failOn !== undefined && failed > ci.failOn) {
        return 1;
      }
      if (ci.failOnPercentage !== undefined && total > 0) {
        if ((failed / total) * 100 > ci.failOnPercentage) {
          return 1;
        }
      }
      if (ci.failOn !== undefined || ci.failOnPercentage !== undefined) {
        return 0;
      }
    }

    return !config.continueOnError ? 1 : 0;
  }

  private showHelp(): void {
    console.log(`
${this.logger.color('🚀 CURL RUNNER', 'bright')}

${this.logger.color('USAGE:', 'yellow')}
  curl-runner [files...] [options]

${this.logger.color('OPTIONS:', 'yellow')}
  -h, --help                    Show this help message
  -n, --dry-run                 Show curl commands without executing
  --http2                       Use HTTP/2 protocol with multiplexing
  --connection-pool             Enable TCP connection pooling with HTTP/2 multiplexing
  --max-streams <n>             Max concurrent streams per host (default: 10)
  --keepalive-time <sec>        TCP keepalive time in seconds (default: 60)
  --connect-timeout <sec>       Connection timeout in seconds (default: 30)
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

  # Dry run - show curl commands without executing
  curl-runner api.yaml --dry-run

  # Use HTTP/2 for all requests
  curl-runner api.yaml --http2

  # Enable connection pooling with HTTP/2 multiplexing
  curl-runner api.yaml -p --connection-pool

  # Connection pooling with custom settings
  curl-runner api.yaml -p --connection-pool --max-streams 20

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
