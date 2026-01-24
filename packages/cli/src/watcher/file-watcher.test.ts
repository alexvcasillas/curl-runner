import { afterEach, beforeEach, describe, expect, mock, test } from 'bun:test';
import { FileWatcher, type WatcherOptions } from './file-watcher';

// Mock logger
const createMockLogger = () => ({
  logWatch: mock(() => {}),
  logWatchReady: mock(() => {}),
  logFileChanged: mock(() => {}),
  logError: mock(() => {}),
  logWarning: mock(() => {}),
});

// Helper to access private method for testing
const triggerFileChange = (watcher: FileWatcher, filename: string) => {
  // biome-ignore lint/complexity/useLiteralKeys: accessing private method for testing
  watcher['handleFileChange'](filename);
};

describe('FileWatcher', () => {
  let watcher: FileWatcher;
  let mockLogger: ReturnType<typeof createMockLogger>;
  let onRunMock: ReturnType<typeof mock>;

  beforeEach(() => {
    mockLogger = createMockLogger();
    onRunMock = mock(() => Promise.resolve());
  });

  afterEach(() => {
    if (watcher) {
      watcher.stop();
    }
  });

  test('should debounce rapid file changes', async () => {
    const options: WatcherOptions = {
      files: [],
      config: { debounce: 50 },
      onRun: onRunMock,
      logger: mockLogger as unknown as WatcherOptions['logger'],
    };

    watcher = new FileWatcher(options);

    // Simulate rapid file changes
    triggerFileChange(watcher, 'test.yaml');
    triggerFileChange(watcher, 'test.yaml');
    triggerFileChange(watcher, 'test.yaml');

    // Wait for debounce to settle
    await Bun.sleep(100);

    // Should only have been called once due to debouncing
    // Initial run doesn't happen because we didn't call start()
    expect(onRunMock).toHaveBeenCalledTimes(1);
  });

  test('should queue runs during execution', async () => {
    let runCount = 0;
    const slowOnRun = mock(async () => {
      runCount++;
      await Bun.sleep(100);
    });

    const options: WatcherOptions = {
      files: [],
      config: { debounce: 10, clear: false },
      onRun: slowOnRun,
      logger: mockLogger as unknown as WatcherOptions['logger'],
    };

    watcher = new FileWatcher(options);

    // Start first run
    triggerFileChange(watcher, 'test.yaml');
    await Bun.sleep(20); // Let debounce fire

    // Change during run (should be queued)
    triggerFileChange(watcher, 'test.yaml');

    // Wait for both runs to complete
    await Bun.sleep(300);

    expect(runCount).toBe(2);
  });

  test('should clear debounce timer on stop', () => {
    const options: WatcherOptions = {
      files: [],
      config: { debounce: 1000 },
      onRun: onRunMock,
      logger: mockLogger as unknown as WatcherOptions['logger'],
    };

    watcher = new FileWatcher(options);

    // Trigger a change to start debounce timer
    triggerFileChange(watcher, 'test.yaml');

    // Stop should clear the timer
    watcher.stop();

    // biome-ignore lint/complexity/useLiteralKeys: accessing private field for testing
    expect(watcher['debounceTimer']).toBeNull();
    // biome-ignore lint/complexity/useLiteralKeys: accessing private field for testing
    expect(watcher['watchers']).toHaveLength(0);
  });

  test('should use default debounce of 300ms', async () => {
    const options: WatcherOptions = {
      files: [],
      config: {}, // No debounce specified
      onRun: onRunMock,
      logger: mockLogger as unknown as WatcherOptions['logger'],
    };

    watcher = new FileWatcher(options);

    const startTime = performance.now();
    triggerFileChange(watcher, 'test.yaml');

    // Wait less than default debounce
    await Bun.sleep(100);
    expect(onRunMock).not.toHaveBeenCalled();

    // Wait for full debounce
    await Bun.sleep(250);
    expect(onRunMock).toHaveBeenCalledTimes(1);

    const elapsed = performance.now() - startTime;
    expect(elapsed).toBeGreaterThanOrEqual(300);
  });

  test('should handle onRun errors gracefully', async () => {
    const errorOnRun = mock(async () => {
      throw new Error('Test error');
    });

    const options: WatcherOptions = {
      files: [],
      config: { debounce: 10 },
      onRun: errorOnRun,
      logger: mockLogger as unknown as WatcherOptions['logger'],
    };

    watcher = new FileWatcher(options);

    // Should not throw
    triggerFileChange(watcher, 'test.yaml');
    await Bun.sleep(50);

    expect(mockLogger.logError).toHaveBeenCalledWith('Test error');
  });

  test('should log file changed with filename', async () => {
    const options: WatcherOptions = {
      files: [],
      config: { debounce: 10, clear: false },
      onRun: onRunMock,
      logger: mockLogger as unknown as WatcherOptions['logger'],
    };

    watcher = new FileWatcher(options);

    triggerFileChange(watcher, 'my-api.yaml');
    await Bun.sleep(50);

    expect(mockLogger.logFileChanged).toHaveBeenCalledWith('my-api.yaml');
  });

  test('should call logWatchReady after run completes', async () => {
    const options: WatcherOptions = {
      files: [],
      config: { debounce: 10, clear: false },
      onRun: onRunMock,
      logger: mockLogger as unknown as WatcherOptions['logger'],
    };

    watcher = new FileWatcher(options);

    triggerFileChange(watcher, 'test.yaml');
    await Bun.sleep(50);

    expect(mockLogger.logWatchReady).toHaveBeenCalled();
  });
});
