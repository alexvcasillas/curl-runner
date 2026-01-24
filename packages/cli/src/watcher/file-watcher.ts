import { type FSWatcher, watch } from 'node:fs';
import type { WatchConfig } from '../types/config';
import type { Logger } from '../utils/logger';

export interface WatcherOptions {
  files: string[];
  config: WatchConfig;
  onRun: () => Promise<void>;
  logger: Logger;
}

export class FileWatcher {
  private watchers: FSWatcher[] = [];
  private debounceTimer: Timer | null = null;
  private isRunning = false;
  private pendingRun = false;
  private options: WatcherOptions;

  constructor(options: WatcherOptions) {
    this.options = options;
  }

  async start(): Promise<void> {
    const { files, logger } = this.options;

    // Initial run
    await this.runRequests();

    // Setup watchers for each file
    for (const file of files) {
      try {
        const watcher = watch(file, (event) => {
          if (event === 'change') {
            this.handleFileChange(file);
          }
        });

        watcher.on('error', (error) => {
          logger.logWarning(`Watch error on ${file}: ${error.message}`);
        });

        this.watchers.push(watcher);
      } catch (error) {
        logger.logWarning(
          `Failed to watch ${file}: ${error instanceof Error ? error.message : String(error)}`,
        );
      }
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

      if (config.clear !== false) {
        console.clear();
      }

      logger.logFileChanged(filename);
      await this.runRequests();
    }, debounce);
  }

  private async runRequests(): Promise<void> {
    const { onRun, logger } = this.options;

    this.isRunning = true;

    try {
      await onRun();
    } catch (error) {
      logger.logError(error instanceof Error ? error.message : String(error));
    }

    this.isRunning = false;

    // Check for pending runs
    if (this.pendingRun) {
      this.pendingRun = false;
      const { config } = this.options;
      if (config.clear !== false) {
        console.clear();
      }
      logger.logFileChanged('(queued change)');
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
