/**
 * Type-safe CLI argument parser.
 * Manual parsing to ensure compatibility with compiled Bun binaries (no external deps).
 */

export interface CLIOptions {
  // Files & Discovery
  files: string[];
  all?: boolean;

  // Execution Mode
  execution?: 'sequential' | 'parallel';
  maxConcurrency?: number;
  continueOnError?: boolean;
  dryRun?: boolean;
  http2?: boolean;

  // Connection Pool
  connectionPool?: boolean;
  maxStreams?: number;
  keepaliveTime?: number;
  connectTimeout?: number;

  // Output
  verbose?: boolean;
  quiet?: boolean;
  output?: string;
  outputFormat?: 'json' | 'pretty' | 'raw';
  prettyLevel?: 'minimal' | 'standard' | 'detailed';
  showHeaders?: boolean;
  showBody?: boolean;
  showMetrics?: boolean;

  // Timeouts & Retries
  timeout?: number;
  retries?: number;
  retryDelay?: number;
  noRetry?: boolean;

  // Watch Mode
  watch?: boolean;
  watchDebounce?: number;
  watchClear?: boolean;

  // Profile Mode
  profile?: number;
  profileWarmup?: number;
  profileConcurrency?: number;
  profileHistogram?: boolean;
  profileExport?: string;

  // Snapshot
  snapshot?: boolean;
  snapshotUpdate?: 'none' | 'all' | 'failing';
  snapshotDir?: string;
  snapshotCi?: boolean;

  // Diff
  diff?: boolean;
  diffSave?: boolean;
  diffLabel?: string;
  diffCompare?: string;
  diffDir?: string;
  diffOutput?: 'terminal' | 'json' | 'markdown';

  // CI Exit Codes
  strictExit?: boolean;
  failOn?: number;
  failOnPercentage?: number;

  // Meta
  help?: boolean;
  version?: boolean;
}

/**
 * Parses command-line arguments into typed CLIOptions.
 * Handles both long flags (--flag) and short flags (-f).
 */
export function parseCliArgs(args: string[]): CLIOptions {
  const options: CLIOptions = { files: [] };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    const nextArg = args[i + 1];
    const hasNextArg = nextArg !== undefined && !nextArg.startsWith('-');

    if (arg.startsWith('--')) {
      i += parseLongFlag(arg.slice(2), nextArg, hasNextArg, options);
    } else if (arg.startsWith('-')) {
      i += parseShortFlags(arg.slice(1), nextArg, hasNextArg, options);
    } else {
      options.files.push(arg);
    }
  }

  return options;
}

function parseLongFlag(
  key: string,
  nextArg: string | undefined,
  hasNextArg: boolean,
  options: CLIOptions,
): number {
  // Boolean flags (no value needed)
  switch (key) {
    case 'help':
      options.help = true;
      return 0;
    case 'version':
      options.version = true;
      return 0;
    case 'all':
      options.all = true;
      return 0;
    case 'dry-run':
      options.dryRun = true;
      return 0;
    case 'http2':
      options.http2 = true;
      return 0;
    case 'connection-pool':
      options.connectionPool = true;
      return 0;
    case 'quiet':
      options.quiet = true;
      return 0;
    case 'show-headers':
      options.showHeaders = true;
      return 0;
    case 'show-body':
      options.showBody = true;
      return 0;
    case 'show-metrics':
      options.showMetrics = true;
      return 0;
    case 'no-retry':
      options.noRetry = true;
      return 0;
    case 'watch':
      options.watch = true;
      return 0;
    case 'watch-clear':
      options.watchClear = true;
      return 0;
    case 'no-watch-clear':
      options.watchClear = false;
      return 0;
    case 'profile-histogram':
      options.profileHistogram = true;
      return 0;
    case 'snapshot':
      options.snapshot = true;
      return 0;
    case 'update-snapshots':
      options.snapshotUpdate = 'all';
      return 0;
    case 'update-failing':
      options.snapshotUpdate = 'failing';
      return 0;
    case 'ci-snapshot':
      options.snapshotCi = true;
      return 0;
    case 'diff':
      options.diff = true;
      return 0;
    case 'diff-save':
      options.diffSave = true;
      return 0;
    case 'strict-exit':
      options.strictExit = true;
      return 0;
  }

  // Flags that require a value
  if (!hasNextArg) {
    // Handle boolean-like flags that can be used without value
    if (key === 'verbose') {
      options.verbose = true;
      return 0;
    }
    if (key === 'continue-on-error') {
      options.continueOnError = true;
      return 0;
    }
    return 0;
  }

  switch (key) {
    // Execution
    case 'execution':
      if (nextArg === 'sequential' || nextArg === 'parallel') {
        options.execution = nextArg;
      }
      return 1;
    case 'max-concurrent': {
      const maxConcurrent = Number.parseInt(nextArg!, 10);
      if (maxConcurrent > 0) {
        options.maxConcurrency = maxConcurrent;
      }
      return 1;
    }

    // Connection Pool
    case 'max-streams': {
      const maxStreams = Number.parseInt(nextArg!, 10);
      if (maxStreams > 0) {
        options.maxStreams = maxStreams;
      }
      return 1;
    }
    case 'keepalive-time':
      options.keepaliveTime = Number.parseInt(nextArg!, 10);
      return 1;
    case 'connect-timeout':
      options.connectTimeout = Number.parseInt(nextArg!, 10);
      return 1;

    // Output
    case 'verbose':
      options.verbose = nextArg === 'true';
      return 1;
    case 'output':
      options.output = nextArg;
      return 1;
    case 'output-format':
      if (['json', 'pretty', 'raw'].includes(nextArg!)) {
        options.outputFormat = nextArg as 'json' | 'pretty' | 'raw';
      }
      return 1;
    case 'pretty-level':
      if (['minimal', 'standard', 'detailed'].includes(nextArg!)) {
        options.prettyLevel = nextArg as 'minimal' | 'standard' | 'detailed';
      }
      return 1;

    // Timeouts & Retries
    case 'timeout':
      options.timeout = Number.parseInt(nextArg!, 10);
      return 1;
    case 'retries':
      options.retries = Number.parseInt(nextArg!, 10);
      return 1;
    case 'retry-delay':
      options.retryDelay = Number.parseInt(nextArg!, 10);
      return 1;
    case 'continue-on-error':
      options.continueOnError = nextArg === 'true';
      return 1;

    // Watch
    case 'watch-debounce':
      options.watchDebounce = Number.parseInt(nextArg!, 10);
      return 1;

    // Profile
    case 'profile':
      options.profile = Number.parseInt(nextArg!, 10);
      return 1;
    case 'profile-warmup':
      options.profileWarmup = Number.parseInt(nextArg!, 10);
      return 1;
    case 'profile-concurrency':
      options.profileConcurrency = Number.parseInt(nextArg!, 10);
      return 1;
    case 'profile-export':
      options.profileExport = nextArg;
      return 1;

    // Snapshot
    case 'snapshot-dir':
      options.snapshotDir = nextArg;
      return 1;

    // Diff
    case 'diff-label':
      options.diffLabel = nextArg;
      return 1;
    case 'diff-compare':
      options.diffCompare = nextArg;
      return 1;
    case 'diff-dir':
      options.diffDir = nextArg;
      return 1;
    case 'diff-output':
      if (['terminal', 'json', 'markdown'].includes(nextArg!)) {
        options.diffOutput = nextArg as 'terminal' | 'json' | 'markdown';
      }
      return 1;

    // CI
    case 'fail-on':
      options.failOn = Number.parseInt(nextArg!, 10);
      return 1;
    case 'fail-on-percentage': {
      const percentage = Number.parseFloat(nextArg!);
      if (percentage >= 0 && percentage <= 100) {
        options.failOnPercentage = percentage;
      }
      return 1;
    }

    default:
      // Unknown flag with value - skip the value
      return 1;
  }
}

function parseShortFlags(
  flags: string,
  nextArg: string | undefined,
  hasNextArg: boolean,
  options: CLIOptions,
): number {
  let consumed = 0;

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
        // Continue on error - handled at resolver level
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
      case 'n':
        options.dryRun = true;
        break;
      case 'o':
        if (hasNextArg) {
          options.output = nextArg;
          consumed = 1;
        }
        break;
      case 'P':
        if (hasNextArg) {
          options.profile = Number.parseInt(nextArg!, 10);
          consumed = 1;
        }
        break;
    }
  }

  return consumed;
}

/**
 * Checks if the args indicate a subcommand that should bypass normal config resolution.
 */
export function detectSubcommand(args: string[]): 'upgrade' | 'diff-subcommand' | null {
  if (args[0] === 'upgrade') {
    return 'upgrade';
  }
  if (args[0] === 'diff' && args.length >= 3) {
    return 'diff-subcommand';
  }
  return null;
}

/**
 * Checks if help or version flags are present (early exit scenarios).
 */
export function detectEarlyExit(options: CLIOptions): 'help' | 'version' | null {
  if (options.help) {
    return 'help';
  }
  if (options.version) {
    return 'version';
  }
  return null;
}
