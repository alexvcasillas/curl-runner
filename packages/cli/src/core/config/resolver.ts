/**
 * Configuration resolver that merges all config sources.
 * Priority order: CLI options > Environment variables > Config file > Defaults
 */

import type { GlobalConfig, ProfileConfig, WatchConfig } from '../../types/config';
import { getGlobalRedactor } from '../../utils/secret-redactor';
import { type CLIOptions, detectEarlyExit, detectSubcommand, parseCliArgs } from './cli-parser';
import { loadEnvFiles } from './env-file-loader';
import { loadEnvironmentConfig } from './env-loader';
import { loadConfigFile } from './file-loader';

export type ExecutionMode =
  | 'normal'
  | 'watch'
  | 'profile'
  | 'diff-subcommand'
  | 'upgrade'
  | 'init'
  | 'edit'
  | 'validate'
  | 'help'
  | 'version';

export interface ResolvedConfig {
  /** Merged global configuration */
  config: GlobalConfig;
  /** CLI options (typed) */
  cliOptions: CLIOptions;
  /** Files to process */
  files: string[];
  /** Execution mode determined from options */
  mode: ExecutionMode;
  /** Raw args for subcommand passthrough */
  rawArgs: string[];
  /** Keys marked as secrets (for redaction) */
  secretKeys: string[];
}

export interface ResolverCallbacks {
  onInfo?: (msg: string) => void;
  onWarning?: (msg: string) => void;
}

/**
 * Resolves configuration from all sources (CLI, env, file) and determines execution mode.
 */
export async function resolveConfig(
  args: string[],
  callbacks?: ResolverCallbacks,
): Promise<ResolvedConfig> {
  // Check for subcommands first (bypass normal config loading)
  const subcommand = detectSubcommand(args);
  if (subcommand === 'upgrade') {
    return {
      config: {},
      cliOptions: { files: [] },
      files: [],
      mode: 'upgrade',
      rawArgs: args,
      secretKeys: [],
    };
  }
  if (subcommand === 'init') {
    return {
      config: {},
      cliOptions: { files: [] },
      files: [],
      mode: 'init',
      rawArgs: args,
      secretKeys: [],
    };
  }
  if (subcommand === 'edit') {
    return {
      config: {},
      cliOptions: { files: [] },
      files: [],
      mode: 'edit',
      rawArgs: args,
      secretKeys: [],
    };
  }
  if (subcommand === 'validate') {
    return {
      config: {},
      cliOptions: { files: [] },
      files: [],
      mode: 'validate',
      rawArgs: args,
      secretKeys: [],
    };
  }

  // Parse CLI arguments
  const cliOptions = parseCliArgs(args);

  // Check for early exit (help/version)
  const earlyExit = detectEarlyExit(cliOptions);
  if (earlyExit) {
    return {
      config: {},
      cliOptions,
      files: cliOptions.files,
      mode: earlyExit,
      rawArgs: args,
      secretKeys: [],
    };
  }

  // Handle diff subcommand
  if (subcommand === 'diff-subcommand') {
    return {
      config: {},
      cliOptions,
      files: args.slice(1), // Pass remaining args (label1, label2, optional file)
      mode: 'diff-subcommand',
      rawArgs: args,
      secretKeys: [],
    };
  }

  // Load config from all sources
  const envConfig = loadEnvironmentConfig();
  const { config: fileConfig } = await loadConfigFile(callbacks?.onInfo, callbacks?.onWarning);

  // Merge configs: file -> env -> CLI (CLI wins)
  let globalConfig = mergeGlobalConfigs(fileConfig, envConfig);
  globalConfig = applyCliOptionsToConfig(globalConfig, cliOptions);

  // Load .env files
  const environment = cliOptions.env ?? globalConfig.env?.environment;
  const { variables: envFileVars, secretKeys } = await loadEnvFiles(process.cwd(), environment);

  // Merge env file variables into global config (lower priority than explicit variables)
  globalConfig.variables = { ...envFileVars, ...globalConfig.variables };

  // Set up secret redaction (unless disabled)
  const redactSecrets = !cliOptions.noRedact && globalConfig.env?.redactSecrets !== false;
  if (redactSecrets && secretKeys.length > 0) {
    const redactor = getGlobalRedactor();
    const secretValues: Record<string, string> = {};
    for (const key of secretKeys) {
      if (globalConfig.variables?.[key]) {
        secretValues[key] = globalConfig.variables[key];
      }
    }
    redactor.addSecrets(secretValues);
  }

  // Determine execution mode
  const mode = determineExecutionMode(cliOptions, globalConfig);

  return {
    config: globalConfig,
    cliOptions,
    files: cliOptions.files,
    mode,
    rawArgs: args,
    secretKeys,
  };
}

/**
 * Merges two GlobalConfig objects with proper nested merging.
 */
export function mergeGlobalConfigs(
  base: Partial<GlobalConfig>,
  override: Partial<GlobalConfig>,
): GlobalConfig {
  return {
    ...base,
    ...override,
    env: { ...base.env, ...override.env },
    variables: { ...base.variables, ...override.variables },
    output: { ...base.output, ...override.output },
    defaults: { ...base.defaults, ...override.defaults },
    ci: { ...base.ci, ...override.ci },
    watch: { ...base.watch, ...override.watch },
    snapshot: { ...base.snapshot, ...override.snapshot },
    diff: { ...base.diff, ...override.diff },
    connectionPool: { ...base.connectionPool, ...override.connectionPool },
  } as GlobalConfig;
}

/**
 * Applies CLI options to global config (highest priority).
 */
function applyCliOptionsToConfig(config: GlobalConfig, options: CLIOptions): GlobalConfig {
  const result = { ...config };

  // Environment
  if (options.env) {
    result.env = { ...result.env, environment: options.env };
  }
  if (options.noRedact) {
    result.env = { ...result.env, redactSecrets: false };
  }

  // Execution
  if (options.execution) {
    result.execution = options.execution;
  }
  if (options.maxConcurrency !== undefined) {
    result.maxConcurrency = options.maxConcurrency;
  }
  if (options.continueOnError !== undefined) {
    result.continueOnError = options.continueOnError;
  }
  if (options.dryRun !== undefined) {
    result.dryRun = options.dryRun;
    result.output = { ...result.output, dryRun: options.dryRun };
  }
  if (options.http2 !== undefined) {
    result.http2 = options.http2;
    result.defaults = { ...result.defaults, http2: options.http2 };
  }

  // Connection pooling
  if (options.connectionPool !== undefined) {
    result.connectionPool = { ...result.connectionPool, enabled: options.connectionPool };
  }
  if (options.maxStreams !== undefined) {
    result.connectionPool = {
      ...result.connectionPool,
      enabled: true,
      maxStreamsPerHost: options.maxStreams,
    };
  }
  if (options.keepaliveTime !== undefined) {
    result.connectionPool = {
      ...result.connectionPool,
      enabled: true,
      keepaliveTime: options.keepaliveTime,
    };
  }
  if (options.connectTimeout !== undefined) {
    result.connectionPool = {
      ...result.connectionPool,
      enabled: true,
      connectTimeout: options.connectTimeout,
    };
  }

  // Output
  if (options.verbose !== undefined) {
    result.output = { ...result.output, verbose: options.verbose };
  }
  if (options.quiet) {
    result.output = { ...result.output, verbose: false };
  }
  if (options.output) {
    result.output = { ...result.output, saveToFile: options.output };
  }
  if (options.outputFormat) {
    result.output = { ...result.output, format: options.outputFormat };
  }
  if (options.prettyLevel) {
    result.output = { ...result.output, prettyLevel: options.prettyLevel };
  }
  if (options.showHeaders !== undefined) {
    result.output = { ...result.output, showHeaders: options.showHeaders };
  }
  if (options.showBody !== undefined) {
    result.output = { ...result.output, showBody: options.showBody };
  }
  if (options.showMetrics !== undefined) {
    result.output = { ...result.output, showMetrics: options.showMetrics };
  }

  // Timeouts & Retries
  if (options.timeout) {
    result.defaults = { ...result.defaults, timeout: options.timeout };
  }
  if (options.retries || options.noRetry) {
    const retryCount = options.noRetry ? 0 : options.retries || 0;
    result.defaults = {
      ...result.defaults,
      retry: { ...result.defaults?.retry, count: retryCount },
    };
  }
  if (options.retryDelay) {
    result.defaults = {
      ...result.defaults,
      retry: { ...result.defaults?.retry, delay: options.retryDelay },
    };
  }

  // CI
  if (options.strictExit !== undefined) {
    result.ci = { ...result.ci, strictExit: options.strictExit };
  }
  if (options.failOn !== undefined) {
    result.ci = { ...result.ci, failOn: options.failOn };
  }
  if (options.failOnPercentage !== undefined) {
    result.ci = { ...result.ci, failOnPercentage: options.failOnPercentage };
  }

  // Snapshot
  if (options.snapshot !== undefined) {
    result.snapshot = { ...result.snapshot, enabled: options.snapshot };
  }
  if (options.snapshotUpdate !== undefined) {
    result.snapshot = { ...result.snapshot, enabled: true, updateMode: options.snapshotUpdate };
  }
  if (options.snapshotDir !== undefined) {
    result.snapshot = { ...result.snapshot, dir: options.snapshotDir };
  }
  if (options.snapshotCi !== undefined) {
    result.snapshot = { ...result.snapshot, ci: options.snapshotCi };
  }

  // Diff
  if (options.diff !== undefined) {
    result.diff = { ...result.diff, enabled: options.diff };
  }
  if (options.diffSave !== undefined) {
    result.diff = { ...result.diff, enabled: true, save: options.diffSave };
  }
  if (options.diffLabel !== undefined) {
    result.diff = { ...result.diff, label: options.diffLabel };
  }
  if (options.diffCompare !== undefined) {
    result.diff = { ...result.diff, enabled: true, compareWith: options.diffCompare };
  }
  if (options.diffDir !== undefined) {
    result.diff = { ...result.diff, dir: options.diffDir };
  }
  if (options.diffOutput !== undefined) {
    result.diff = { ...result.diff, outputFormat: options.diffOutput };
  }

  return result;
}

/**
 * Determines execution mode based on CLI options and global config.
 */
function determineExecutionMode(options: CLIOptions, config: GlobalConfig): ExecutionMode {
  const watchEnabled = options.watch || config.watch?.enabled;
  const profileIterations = options.profile ?? config.profile?.iterations;
  const profileEnabled = profileIterations && profileIterations > 0;

  if (profileEnabled) {
    return 'profile';
  }
  if (watchEnabled) {
    return 'watch';
  }
  return 'normal';
}

/**
 * Builds ProfileConfig from CLI options and global config.
 */
export function buildProfileConfig(options: CLIOptions, config: GlobalConfig): ProfileConfig {
  return {
    iterations: options.profile ?? config.profile?.iterations ?? 10,
    warmup: options.profileWarmup ?? config.profile?.warmup ?? 1,
    concurrency: options.profileConcurrency ?? config.profile?.concurrency ?? 1,
    histogram: options.profileHistogram ?? config.profile?.histogram ?? false,
    exportFile: options.profileExport ?? config.profile?.exportFile,
  };
}

/**
 * Builds WatchConfig from CLI options and global config.
 */
export function buildWatchConfig(options: CLIOptions, config: GlobalConfig): WatchConfig {
  return {
    enabled: true,
    debounce: options.watchDebounce ?? config.watch?.debounce ?? 300,
    clear: options.watchClear ?? config.watch?.clear ?? true,
  };
}
