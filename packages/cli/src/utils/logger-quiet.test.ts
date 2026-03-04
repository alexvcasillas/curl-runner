import { afterEach, beforeEach, describe, expect, spyOn, test } from 'bun:test';
import type { ExecutionResult, ExecutionSummary } from '../types/config';
import { Logger } from './logger';

describe('Logger quiet mode', () => {
  let consoleLogSpy: ReturnType<typeof spyOn>;
  let consoleErrorSpy: ReturnType<typeof spyOn>;
  let consoleWarnSpy: ReturnType<typeof spyOn>;
  let logOutput: string[];
  let errorOutput: string[];
  let warnOutput: string[];

  beforeEach(() => {
    logOutput = [];
    errorOutput = [];
    warnOutput = [];
    consoleLogSpy = spyOn(console, 'log').mockImplementation((...args) => {
      logOutput.push(args.join(' '));
    });
    consoleErrorSpy = spyOn(console, 'error').mockImplementation((...args) => {
      errorOutput.push(args.join(' '));
    });
    consoleWarnSpy = spyOn(console, 'warn').mockImplementation((...args) => {
      warnOutput.push(args.join(' '));
    });
  });

  afterEach(() => {
    consoleLogSpy.mockRestore();
    consoleErrorSpy.mockRestore();
    consoleWarnSpy.mockRestore();
  });

  const quietPrettyLogger = () => new Logger({ quiet: true, format: 'pretty' });
  const quietJsonLogger = () => new Logger({ quiet: true, format: 'json' });
  const quietRawLogger = () => new Logger({ quiet: true, format: 'raw' });

  const makeResult = (overrides: Partial<ExecutionResult> = {}): ExecutionResult => ({
    request: { url: 'https://example.com', method: 'GET', name: 'Test' },
    success: true,
    status: 200,
    body: '{"ok":true}',
    headers: { 'content-type': 'application/json' },
    metrics: { duration: 100, size: 50 },
    ...overrides,
  });

  const makeSummary = (): ExecutionSummary => ({
    total: 1,
    successful: 1,
    failed: 0,
    skipped: 0,
    duration: 100,
    results: [makeResult()],
  });

  describe('shouldShowOutput returns false when quiet', () => {
    test('quiet + pretty format suppresses output', () => {
      const logger = quietPrettyLogger();
      logger.logExecutionStart(1, 'sequential');
      expect(logOutput).toHaveLength(0);
    });

    test('quiet + json format suppresses output', () => {
      const logger = quietJsonLogger();
      logger.logExecutionStart(1, 'sequential');
      expect(logOutput).toHaveLength(0);
    });

    test('quiet + raw format suppresses output', () => {
      const logger = quietRawLogger();
      logger.logExecutionStart(1, 'sequential');
      expect(logOutput).toHaveLength(0);
    });
  });

  describe('suppressed methods when quiet', () => {
    test('logExecutionStart suppressed', () => {
      quietPrettyLogger().logExecutionStart(3, 'parallel');
      expect(logOutput).toHaveLength(0);
    });

    test('logRequestComplete suppressed in pretty format', () => {
      quietPrettyLogger().logRequestComplete(makeResult());
      expect(logOutput).toHaveLength(0);
    });

    test('logSummary suppressed', () => {
      quietPrettyLogger().logSummary(makeSummary());
      expect(logOutput).toHaveLength(0);
    });

    test('logInfo suppressed', () => {
      quietPrettyLogger().logInfo('some info');
      expect(logOutput).toHaveLength(0);
    });

    test('logSuccess suppressed', () => {
      quietPrettyLogger().logSuccess('done');
      expect(logOutput).toHaveLength(0);
    });

    test('logWarning suppressed', () => {
      quietPrettyLogger().logWarning('careful');
      expect(warnOutput).toHaveLength(0);
    });

    test('logRetry suppressed', () => {
      quietPrettyLogger().logRetry(1, 3);
      expect(logOutput).toHaveLength(0);
    });

    test('logSkipped suppressed', () => {
      quietPrettyLogger().logSkipped(
        { url: 'https://example.com', method: 'GET', name: 'Test' },
        1,
        'condition not met',
      );
      expect(logOutput).toHaveLength(0);
    });

    test('logFileHeader suppressed', () => {
      quietPrettyLogger().logFileHeader('test.yaml', 3);
      expect(logOutput).toHaveLength(0);
    });

    test('logWatch suppressed', () => {
      quietPrettyLogger().logWatch(['a.yaml', 'b.yaml']);
      expect(logOutput).toHaveLength(0);
    });

    test('logWatchReady suppressed', () => {
      quietPrettyLogger().logWatchReady();
      expect(logOutput).toHaveLength(0);
    });

    test('logFileChanged suppressed', () => {
      quietPrettyLogger().logFileChanged('test.yaml');
      expect(logOutput).toHaveLength(0);
    });

    test('logProfileStart suppressed', () => {
      quietPrettyLogger().logProfileStart('Test', 100, 5, 1);
      expect(logOutput).toHaveLength(0);
    });

    test('logProfileSummary suppressed', () => {
      quietPrettyLogger().logProfileSummary([]);
      expect(logOutput).toHaveLength(0);
    });
  });

  describe('logError always shows when quiet', () => {
    test('logError still outputs in quiet mode', () => {
      quietPrettyLogger().logError('something broke');
      expect(errorOutput.length).toBeGreaterThan(0);
      expect(errorOutput.some((line) => line.includes('something broke'))).toBe(true);
    });
  });

  describe('raw format still outputs body when quiet', () => {
    test('logRequestComplete with raw format outputs body', () => {
      const logger = quietRawLogger();
      logger.logRequestComplete(makeResult({ body: '{"data":"value"}' }));
      // Raw format body output is handled before shouldShowOutput check
      // but quiet + raw = shouldShowOutput returns false, and raw check comes first
      // Actually raw format has its own early return that outputs body
      // Let's verify: raw format outputs body regardless of quiet
      // Looking at the code: raw format check is BEFORE the shouldShowOutput check
      // But shouldShowOutput now returns false for quiet before the raw check
      // Wait - the raw format in logRequestComplete has its own early return at the top
      // that doesn't go through shouldShowOutput. Let me check...
      // Actually logRequestComplete checks format === 'raw' first (line 245),
      // then format === 'json' (line 254), then shouldShowOutput for pretty.
      // So raw format body output is NOT affected by shouldShowOutput.
      // But wait - with quiet: true, shouldShowOutput returns false before
      // checking raw. However, logRequestComplete doesn't use shouldShowOutput
      // for raw format - it has its own direct format checks.
      // The raw format block just checks showBody, not shouldShowOutput.
      expect(logOutput.some((line) => line.includes('data'))).toBe(true);
    });
  });

  describe('dry-run command still shows when quiet', () => {
    test('logCommand in dry-run mode still shows when quiet', () => {
      const logger = new Logger({ quiet: true, format: 'pretty', dryRun: true });
      logger.logCommand('curl https://example.com');
      expect(logOutput.length).toBeGreaterThan(0);
      expect(logOutput.some((line) => line.includes('curl'))).toBe(true);
    });
  });
});
