# Watch Mode Implementation Plan

## Overview

Watch mode enables automatic re-execution of requests when YAML files change. This provides a tight feedback loop during API development and testing.

```bash
curl-runner api.yaml --watch
curl-runner tests/ -w
```

---

## User Experience

### Basic Usage

```bash
# Watch single file
curl-runner api.yaml --watch

# Watch directory
curl-runner tests/ --watch

# Watch with other options
curl-runner api.yaml -w -p --max-concurrent 5

# Watch with debounce customization
curl-runner api.yaml -w --watch-debounce 500
```

### Output Example

```
👀 Watching for changes... (press Ctrl+C to stop)
   Files: api.yaml

─────────────────────────────────────────────────
[12:34:56] File changed: api.yaml

✓ Get Users [api]
   ├─ GET https://api.example.com/users
   ├─ ✓ Status: 200
   └─ Duration: 45ms | 1.2 KB

Summary: 1 request completed successfully

👀 Watching for changes...
```

---

## CLI Interface

### New Flags

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--watch` | `-w` | boolean | false | Enable watch mode |
| `--watch-debounce` | | number | 300 | Debounce delay in ms |
| `--watch-clear` | | boolean | true | Clear screen between runs |

### Environment Variables

| Variable | Description |
|----------|-------------|
| `CURL_RUNNER_WATCH` | Enable watch mode ("true") |
| `CURL_RUNNER_WATCH_DEBOUNCE` | Debounce delay in ms |
| `CURL_RUNNER_WATCH_CLEAR` | Clear screen between runs |

### YAML Configuration

```yaml
global:
  watch:
    enabled: true
    debounce: 300
    clear: true
```

---

## Architecture

### New Files

```
packages/cli/src/
├── watcher/
│   ├── file-watcher.ts      # Core file watching logic
│   └── watcher.test.ts      # Unit tests
└── types/config.ts          # Updated with WatchConfig
```

### Modified Files

```
packages/cli/src/
├── cli.ts                   # Add watch mode handling
├── types/config.ts          # Add WatchConfig interface
└── utils/logger.ts          # Add watch-specific logging
```

---

## Implementation Details

### 1. Types (`packages/cli/src/types/config.ts`)

```typescript
// Add to existing types

export interface WatchConfig {
  /** Enable watch mode */
  enabled?: boolean;
  /** Debounce delay in milliseconds. Default: 300 */
  debounce?: number;
  /** Clear screen between runs. Default: true */
  clear?: boolean;
}

// Update GlobalConfig
export interface GlobalConfig {
  // ... existing fields
  watch?: WatchConfig;
}
```

### 2. File Watcher (`packages/cli/src/watcher/file-watcher.ts`)

```typescript
import { watch, type WatchEventType } from 'fs';
import type { WatchConfig } from '../types/config';
import { Logger } from '../utils/logger';

export interface WatcherOptions {
  files: string[];
  config: WatchConfig;
  onRun: () => Promise<void>;
  logger: Logger;
}

export class FileWatcher {
  private watchers: ReturnType<typeof watch>[] = [];
  private debounceTimer: Timer | null = null;
  private isRunning = false;
  private pendingRun = false;
  private options: WatcherOptions;

  constructor(options: WatcherOptions) {
    this.options = options;
  }

  async start(): Promise<void> {
    const { files, config, logger } = this.options;

    // Initial run
    await this.runRequests();

    // Setup watchers
    for (const file of files) {
      const watcher = watch(file, (event: WatchEventType, filename) => {
        if (event === 'change') {
          this.handleFileChange(filename || file);
        }
      });
      this.watchers.push(watcher);
    }

    // Log watching status
    logger.logWatch(files);

    // Handle graceful shutdown
    this.setupSignalHandlers();

    // Keep process alive
    await this.keepAlive();
  }

  private handleFileChange(filename: string): void {
    const { config, logger } = this.options;
    const debounce = config.debounce ?? 300;

    // Clear existing debounce timer
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
    }

    // Debounce the run
    this.debounceTimer = setTimeout(async () => {
      // If already running, queue for after completion
      if (this.isRunning) {
        this.pendingRun = true;
        return;
      }

      logger.logFileChanged(filename);
      await this.runRequests();
    }, debounce);
  }

  private async runRequests(): Promise<void> {
    const { config, onRun, logger } = this.options;

    this.isRunning = true;

    // Clear screen if configured
    if (config.clear !== false) {
      console.clear();
    }

    try {
      await onRun();
    } catch (error) {
      logger.logError(error instanceof Error ? error.message : String(error));
    }

    this.isRunning = false;

    // Check for pending runs
    if (this.pendingRun) {
      this.pendingRun = false;
      await this.runRequests();
    } else {
      logger.logWatchReady();
    }
  }

  private setupSignalHandlers(): void {
    const cleanup = () => {
      this.stop();
      process.exit(0);
    };

    process.on('SIGINT', cleanup);
    process.on('SIGTERM', cleanup);
  }

  private async keepAlive(): Promise<never> {
    // Use Bun.sleep in an infinite loop to keep process alive
    while (true) {
      await Bun.sleep(1000);
    }
  }

  stop(): void {
    for (const watcher of this.watchers) {
      watcher.close();
    }
    this.watchers = [];

    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer);
      this.debounceTimer = null;
    }
  }
}
```

### 3. Logger Updates (`packages/cli/src/utils/logger.ts`)

```typescript
// Add to Logger class

logWatch(files: string[]): void {
  console.log();
  console.log(this.color('👀 Watching for changes...', 'cyan') +
              this.color(' (press Ctrl+C to stop)', 'dim'));
  console.log(this.color(`   Files: ${files.join(', ')}`, 'dim'));
  console.log();
}

logWatchReady(): void {
  console.log();
  console.log(this.color('👀 Watching for changes...', 'cyan'));
}

logFileChanged(filename: string): void {
  const timestamp = new Date().toLocaleTimeString('en-US', { hour12: false });
  console.log(this.color('─'.repeat(50), 'dim'));
  console.log(this.color(`[${timestamp}]`, 'dim') +
              ` File changed: ${this.color(filename, 'yellow')}`);
  console.log();
}
```

### 4. CLI Integration (`packages/cli/src/cli.ts`)

#### 4.1 Update `parseArguments`

```typescript
// Add to parseArguments method

// In the --flag section:
} else if (key === 'watch') {
  options.watch = true;
} else if (key === 'watch-debounce') {
  options.watchDebounce = Number.parseInt(nextArg, 10);
  i++;
} else if (key === 'watch-clear') {
  options.watchClear = nextArg !== 'false';
  i++;

// In the -flag section:
case 'w':
  options.watch = true;
  break;
```

#### 4.2 Update `loadEnvironmentVariables`

```typescript
// Add to loadEnvironmentVariables method

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
```

#### 4.3 Update `run` method

```typescript
// Replace process.exit(exitCode) at end of run() with:

// Check if watch mode is enabled
const watchEnabled = options.watch || globalConfig.watch?.enabled;

if (watchEnabled) {
  const watchConfig: WatchConfig = {
    enabled: true,
    debounce: options.watchDebounce ?? globalConfig.watch?.debounce ?? 300,
    clear: options.watchClear ?? globalConfig.watch?.clear ?? true,
  };

  const watcher = new FileWatcher({
    files: yamlFiles,
    config: watchConfig,
    logger: this.logger,
    onRun: async () => {
      // Re-process files and execute
      await this.executeFiles(yamlFiles, globalConfig, options);
    },
  });

  await watcher.start();
} else {
  const exitCode = this.determineExitCode(summary, globalConfig);
  process.exit(exitCode);
}
```

#### 4.4 Extract execution logic into `executeFiles` method

```typescript
private async executeFiles(
  yamlFiles: string[],
  globalConfig: GlobalConfig,
  options: Record<string, unknown>,
): Promise<ExecutionSummary> {
  // Move the file processing and execution logic here
  // This is lines 178-334 of current cli.ts
  // Return the summary instead of calling process.exit
}
```

#### 4.5 Update `showHelp`

```typescript
// Add to OPTIONS section:
  -w, --watch                   Watch files and re-run on changes
  --watch-debounce <ms>         Debounce delay for watch mode (default: 300)
  --watch-clear                 Clear screen between watch runs (default: true)

// Add to EXAMPLES section:
  # Watch mode - re-run on file changes
  curl-runner api.yaml --watch

  # Watch with custom debounce
  curl-runner tests/ -w --watch-debounce 500
```

---

## Implementation Steps

### Phase 1: Core Types & Watcher

1. **Update types** (`types/config.ts`)
   - Add `WatchConfig` interface
   - Update `GlobalConfig` with watch field

2. **Create FileWatcher** (`watcher/file-watcher.ts`)
   - Implement `FileWatcher` class
   - File change detection using `fs.watch`
   - Debouncing logic
   - Signal handling (Ctrl+C)

### Phase 2: Logger & CLI

3. **Update Logger** (`utils/logger.ts`)
   - Add `logWatch()` method
   - Add `logWatchReady()` method
   - Add `logFileChanged()` method

4. **Update CLI** (`cli.ts`)
   - Add flag parsing (`-w`, `--watch`, etc.)
   - Add env var loading
   - Extract `executeFiles()` method
   - Integrate `FileWatcher`
   - Update help text

### Phase 3: Testing & Polish

5. **Write tests** (`watcher/watcher.test.ts`)
   - Debounce behavior
   - Multiple file changes
   - Cleanup on stop

6. **Documentation** (`packages/docs/`)
   - Add watch mode page
   - Update CLI reference

---

## Edge Cases

| Case | Behavior |
|------|----------|
| File deleted during watch | Log warning, continue watching other files |
| Invalid YAML on save | Show parse error, wait for next change |
| Rapid consecutive saves | Debounce, run only once |
| Change during execution | Queue run, execute after current completes |
| Permission error on file | Log error, skip file |
| New file added to watched dir | Not auto-added (explicit files only) |

---

## Testing Strategy

### Unit Tests

```typescript
// watcher/watcher.test.ts

import { describe, expect, test, mock, beforeEach, afterEach } from 'bun:test';
import { FileWatcher } from './file-watcher';

describe('FileWatcher', () => {
  test('debounces rapid changes', async () => {
    const onRun = mock(() => Promise.resolve());
    const watcher = new FileWatcher({
      files: ['test.yaml'],
      config: { debounce: 100 },
      onRun,
      logger: mockLogger,
    });

    // Simulate rapid changes
    watcher['handleFileChange']('test.yaml');
    watcher['handleFileChange']('test.yaml');
    watcher['handleFileChange']('test.yaml');

    await Bun.sleep(150);
    expect(onRun).toHaveBeenCalledTimes(1);
  });

  test('queues run during execution', async () => {
    let runCount = 0;
    const onRun = mock(async () => {
      runCount++;
      await Bun.sleep(100);
    });

    const watcher = new FileWatcher({
      files: ['test.yaml'],
      config: { debounce: 10 },
      onRun,
      logger: mockLogger,
    });

    // Start first run
    watcher['handleFileChange']('test.yaml');
    await Bun.sleep(20);

    // Change during run
    watcher['handleFileChange']('test.yaml');
    await Bun.sleep(150);

    expect(runCount).toBe(2);
  });

  test('stops cleanly', () => {
    const watcher = new FileWatcher({...});
    watcher['watchers'] = [{ close: mock() }];

    watcher.stop();

    expect(watcher['watchers']).toHaveLength(0);
  });
});
```

### Integration Tests

```bash
# Manual testing scenarios

# 1. Basic watch
curl-runner examples/simple-get.yaml -w

# 2. Edit file and verify re-run
# (edit examples/simple-get.yaml in another terminal)

# 3. Watch directory
curl-runner examples/ -w

# 4. Ctrl+C graceful exit
# (press Ctrl+C and verify clean shutdown)

# 5. Invalid YAML handling
# (save invalid YAML and verify error shown)
```

---

## Performance Considerations

1. **Debouncing**: Default 300ms prevents excessive runs during rapid saves
2. **Single watcher per file**: Efficient resource usage
3. **No polling**: Uses native fs events via `fs.watch`
4. **Queued execution**: Prevents overlapping runs

---

## Future Enhancements (Out of Scope)

- [ ] Watch glob patterns (`tests/**/*.yaml`)
- [ ] Hot reload specific requests only
- [ ] Browser notification on failure
- [ ] Sound notification option
- [ ] Watch config files too (curl-runner.yaml)

---

## Unresolved Questions

1. **Dir watching**: watch dir or only explicit files passed? → **Files only** (simpler, predictable)
2. **Recursive dir**: if `tests/` passed, watch `tests/**/*.yaml`? → **No**, use `--all` to find files initially
3. **Exit code in watch**: always 0 or last run's code? → **N/A**, process stays alive
4. **Clear screen default**: true or false? → **true** (cleaner UX, can disable)
5. **Bun vs Node API**: `Bun.file().watch()` or `fs.watch`? → **`fs.watch`** (more reliable, cross-platform)
