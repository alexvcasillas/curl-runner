/**
 * Configuration module for curl-runner.
 * Handles CLI parsing, environment variables, config files, and merging.
 */

// CLI Parser
export { type CLIOptions, detectEarlyExit, detectSubcommand, parseCliArgs } from './cli-parser';
// Defaults
export { CONFIG_DEFAULTS, type ConfigDefaults } from './defaults';
// Environment Loader
export { loadEnvironmentConfig } from './env-loader';
// File Loader
export { CONFIG_FILE_PATHS, type ConfigFileResult, loadConfigFile } from './file-loader';
// Resolver
export {
  buildProfileConfig,
  buildWatchConfig,
  type ExecutionMode,
  mergeGlobalConfigs,
  type ResolvedConfig,
  type ResolverCallbacks,
  resolveConfig,
} from './resolver';
