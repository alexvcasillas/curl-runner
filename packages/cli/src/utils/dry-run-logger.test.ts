import { afterEach, beforeEach, describe, expect, spyOn, test } from 'bun:test';
import type { ExecutionResult, ExecutionSummary, GlobalConfig } from '../types/config';
import { Logger } from './logger';

/**
 * Tests for Logger dry-run output formatting.
 */

describe('Logger Dry Run Output', () => {
  let consoleLogSpy: ReturnType<typeof spyOn>;
  let consoleErrorSpy: ReturnType<typeof spyOn>;
  let logOutput: string[];

  beforeEach(() => {
    logOutput = [];
    consoleLogSpy = spyOn(console, 'log').mockImplementation((...args) => {
      logOutput.push(args.join(' '));
    });
    consoleErrorSpy = spyOn(console, 'error').mockImplementation((...args) => {
      logOutput.push(args.join(' '));
    });
  });

  afterEach(() => {
    consoleLogSpy.mockRestore();
    consoleErrorSpy.mockRestore();
  });

  describe('logCommand in dry-run mode', () => {
    test('always shows command when dryRun is true', () => {
      const config: GlobalConfig['output'] = { dryRun: true };
      const logger = new Logger(config);

      logger.logCommand('curl -X GET https://api.example.com');

      const output = logOutput.join('\n');
      expect(output).toContain('Command');
      expect(output).toContain('curl -X GET https://api.example.com');
    });

    test('shows command with verbose false but dryRun true', () => {
      const config: GlobalConfig['output'] = { dryRun: true, verbose: false };
      const logger = new Logger(config);

      logger.logCommand('curl -X POST https://api.example.com');

      const output = logOutput.join('\n');
      expect(output).toContain('Command');
      expect(output).toContain('curl -X POST');
    });
  });

  describe('logRequestComplete with dry-run result', () => {
    test('displays DRY-RUN status in minimal pretty format', () => {
      const config: GlobalConfig['output'] = { format: 'pretty', prettyLevel: 'minimal' };
      const logger = new Logger(config);

      const result: ExecutionResult = {
        request: { url: 'https://api.example.com', method: 'GET', name: 'Test' },
        success: true,
        dryRun: true,
        metrics: { duration: 0 },
      };

      logger.logRequestComplete(result);

      const output = logOutput.join('\n');
      expect(output).toContain('DRY-RUN');
    });

    test('displays DRY-RUN status in standard pretty format', () => {
      const config: GlobalConfig['output'] = { format: 'pretty', prettyLevel: 'standard' };
      const logger = new Logger(config);

      const result: ExecutionResult = {
        request: { url: 'https://api.example.com', method: 'GET', name: 'Test' },
        success: true,
        dryRun: true,
        metrics: { duration: 0 },
      };

      logger.logRequestComplete(result);

      const output = logOutput.join('\n');
      expect(output).toContain('DRY-RUN');
    });

    test('displays DRY-RUN status in detailed pretty format', () => {
      const config: GlobalConfig['output'] = { format: 'pretty', prettyLevel: 'detailed' };
      const logger = new Logger(config);

      const result: ExecutionResult = {
        request: { url: 'https://api.example.com', method: 'GET', name: 'Test' },
        success: true,
        dryRun: true,
        metrics: { duration: 0 },
      };

      logger.logRequestComplete(result);

      const output = logOutput.join('\n');
      expect(output).toContain('DRY-RUN');
    });

    test('includes dryRun field in JSON format output', () => {
      const config: GlobalConfig['output'] = { format: 'json' };
      const logger = new Logger(config);

      const result: ExecutionResult = {
        request: { url: 'https://api.example.com', method: 'GET', name: 'Test' },
        success: true,
        dryRun: true,
        metrics: { duration: 0 },
      };

      logger.logRequestComplete(result);

      const output = logOutput.join('\n');
      const jsonOutput = JSON.parse(output);
      expect(jsonOutput.dryRun).toBe(true);
      expect(jsonOutput.status).toBeUndefined();
    });

    test('shows success checkmark for dry-run results', () => {
      const config: GlobalConfig['output'] = { format: 'pretty', prettyLevel: 'minimal' };
      const logger = new Logger(config);

      const result: ExecutionResult = {
        request: { url: 'https://api.example.com', method: 'GET', name: 'Test' },
        success: true,
        dryRun: true,
        metrics: { duration: 0 },
      };

      logger.logRequestComplete(result);

      const output = logOutput.join('\n');
      expect(output).toContain('✓');
    });

    test('does not show status code for dry-run results', () => {
      const config: GlobalConfig['output'] = { format: 'pretty', prettyLevel: 'minimal' };
      const logger = new Logger(config);

      const result: ExecutionResult = {
        request: { url: 'https://api.example.com', method: 'GET', name: 'Test' },
        success: true,
        dryRun: true,
        metrics: { duration: 0 },
      };

      logger.logRequestComplete(result);

      const output = logOutput.join('\n');
      // Should show DRY-RUN instead of a numeric status
      expect(output).toContain('DRY-RUN');
      expect(output).not.toMatch(/Status.*\d{3}/); // No 3-digit status code
    });

    test('does not show body for dry-run results', () => {
      const config: GlobalConfig['output'] = {
        format: 'pretty',
        prettyLevel: 'standard',
        showBody: true,
      };
      const logger = new Logger(config);

      const result: ExecutionResult = {
        request: { url: 'https://api.example.com', method: 'GET', name: 'Test' },
        success: true,
        dryRun: true,
        metrics: { duration: 0 },
        // No body field since dry-run doesn't execute
      };

      logger.logRequestComplete(result);

      const output = logOutput.join('\n');
      expect(output).not.toContain('Response Body');
    });
  });

  describe('logSummary with dry-run results', () => {
    test('includes dryRun in JSON summary for dry-run results', () => {
      const config: GlobalConfig['output'] = { format: 'json' };
      const logger = new Logger(config);

      const summary: ExecutionSummary = {
        total: 2,
        successful: 2,
        failed: 0,
        skipped: 0,
        duration: 0,
        results: [
          {
            request: { url: 'https://api.example.com/1', method: 'GET' },
            success: true,
            dryRun: true,
            metrics: { duration: 0 },
          },
          {
            request: { url: 'https://api.example.com/2', method: 'GET' },
            success: true,
            dryRun: true,
            metrics: { duration: 0 },
          },
        ],
      };

      logger.logSummary(summary);

      const output = logOutput.join('\n');
      const jsonOutput = JSON.parse(output);

      expect(jsonOutput.results[0].dryRun).toBe(true);
      expect(jsonOutput.results[1].dryRun).toBe(true);
    });

    test('shows all successful in pretty summary for dry-run', () => {
      const config: GlobalConfig['output'] = { format: 'pretty', prettyLevel: 'minimal' };
      const logger = new Logger(config);

      const summary: ExecutionSummary = {
        total: 3,
        successful: 3,
        failed: 0,
        skipped: 0,
        duration: 0,
        results: [
          {
            request: { url: 'https://api.example.com/1', method: 'GET' },
            success: true,
            dryRun: true,
            metrics: { duration: 0 },
          },
          {
            request: { url: 'https://api.example.com/2', method: 'GET' },
            success: true,
            dryRun: true,
            metrics: { duration: 0 },
          },
          {
            request: { url: 'https://api.example.com/3', method: 'GET' },
            success: true,
            dryRun: true,
            metrics: { duration: 0 },
          },
        ],
      };

      logger.logSummary(summary);

      const output = logOutput.join('\n');
      expect(output).toContain('3 requests completed successfully');
    });
  });

  describe('raw format with dry-run', () => {
    test('does not output anything for dry-run in raw format', () => {
      const config: GlobalConfig['output'] = { format: 'raw' };
      const logger = new Logger(config);

      const result: ExecutionResult = {
        request: { url: 'https://api.example.com', method: 'GET', name: 'Test' },
        success: true,
        dryRun: true,
        metrics: { duration: 0 },
      };

      logger.logRequestComplete(result);

      // Raw format should not output anything since there's no body
      expect(logOutput.length).toBe(0);
    });
  });
});

describe('Logger Color Formatting for Dry-Run', () => {
  test('uses cyan color for DRY-RUN status', () => {
    const logger = new Logger({ format: 'pretty', prettyLevel: 'minimal' });

    // Test that color method works
    const cyanText = logger.color('DRY-RUN', 'cyan');
    expect(cyanText).toContain('\x1b[36m'); // cyan ANSI code
    expect(cyanText).toContain('DRY-RUN');
    expect(cyanText).toContain('\x1b[0m'); // reset code
  });
});
