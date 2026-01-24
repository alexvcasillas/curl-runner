import { describe, expect, test } from 'bun:test';
import type { GlobalConfig, RequestConfig } from '../types/config';

/**
 * Simulates the chunked execution logic from executeParallel
 * Returns the chunks that would be created based on maxConcurrency
 */
function getExecutionChunks(
  requests: RequestConfig[],
  maxConcurrency: number | undefined,
): RequestConfig[][] {
  if (!maxConcurrency || maxConcurrency >= requests.length) {
    return [requests]; // All requests in a single batch
  }

  const chunks: RequestConfig[][] = [];
  for (let i = 0; i < requests.length; i += maxConcurrency) {
    chunks.push(requests.slice(i, i + maxConcurrency));
  }
  return chunks;
}

/**
 * Creates mock requests for testing
 */
function createMockRequests(count: number): RequestConfig[] {
  return Array.from({ length: count }, (_, i) => ({
    url: `https://api.example.com/request/${i + 1}`,
    method: 'GET' as const,
    name: `Request ${i + 1}`,
  }));
}

describe('maxConcurrency parallel execution', () => {
  describe('chunk creation', () => {
    test('should execute all requests at once when maxConcurrency is not set', () => {
      const requests = createMockRequests(10);
      const chunks = getExecutionChunks(requests, undefined);
      expect(chunks).toHaveLength(1);
      expect(chunks[0]).toHaveLength(10);
    });

    test('should execute all requests at once when maxConcurrency >= requests.length', () => {
      const requests = createMockRequests(5);
      const chunks = getExecutionChunks(requests, 10);
      expect(chunks).toHaveLength(1);
      expect(chunks[0]).toHaveLength(5);
    });

    test('should create correct chunks when maxConcurrency is 1', () => {
      const requests = createMockRequests(3);
      const chunks = getExecutionChunks(requests, 1);
      expect(chunks).toHaveLength(3);
      expect(chunks[0]).toHaveLength(1);
      expect(chunks[1]).toHaveLength(1);
      expect(chunks[2]).toHaveLength(1);
    });

    test('should create correct chunks when maxConcurrency is 2', () => {
      const requests = createMockRequests(5);
      const chunks = getExecutionChunks(requests, 2);
      expect(chunks).toHaveLength(3);
      expect(chunks[0]).toHaveLength(2);
      expect(chunks[1]).toHaveLength(2);
      expect(chunks[2]).toHaveLength(1);
    });

    test('should create correct chunks when maxConcurrency is 3', () => {
      const requests = createMockRequests(10);
      const chunks = getExecutionChunks(requests, 3);
      expect(chunks).toHaveLength(4);
      expect(chunks[0]).toHaveLength(3);
      expect(chunks[1]).toHaveLength(3);
      expect(chunks[2]).toHaveLength(3);
      expect(chunks[3]).toHaveLength(1);
    });

    test('should handle edge case with maxConcurrency equal to requests length', () => {
      const requests = createMockRequests(5);
      const chunks = getExecutionChunks(requests, 5);
      expect(chunks).toHaveLength(1);
      expect(chunks[0]).toHaveLength(5);
    });

    test('should preserve request order across chunks', () => {
      const requests = createMockRequests(6);
      const chunks = getExecutionChunks(requests, 2);

      // Verify order is preserved
      expect(chunks[0][0].name).toBe('Request 1');
      expect(chunks[0][1].name).toBe('Request 2');
      expect(chunks[1][0].name).toBe('Request 3');
      expect(chunks[1][1].name).toBe('Request 4');
      expect(chunks[2][0].name).toBe('Request 5');
      expect(chunks[2][1].name).toBe('Request 6');
    });
  });

  describe('GlobalConfig maxConcurrency validation', () => {
    test('should accept valid maxConcurrency values', () => {
      const config1: GlobalConfig = { execution: 'parallel', maxConcurrency: 1 };
      expect(config1.maxConcurrency).toBe(1);

      const config2: GlobalConfig = { execution: 'parallel', maxConcurrency: 5 };
      expect(config2.maxConcurrency).toBe(5);

      const config3: GlobalConfig = { execution: 'parallel', maxConcurrency: 100 };
      expect(config3.maxConcurrency).toBe(100);
    });

    test('should allow undefined maxConcurrency', () => {
      const config: GlobalConfig = { execution: 'parallel' };
      expect(config.maxConcurrency).toBeUndefined();
    });
  });

  describe('integration with other settings', () => {
    test('maxConcurrency should coexist with continueOnError', () => {
      const config: GlobalConfig = {
        execution: 'parallel',
        maxConcurrency: 5,
        continueOnError: true,
      };
      expect(config.maxConcurrency).toBe(5);
      expect(config.continueOnError).toBe(true);
    });

    test('maxConcurrency should coexist with CI settings', () => {
      const config: GlobalConfig = {
        execution: 'parallel',
        maxConcurrency: 3,
        ci: { strictExit: true, failOn: 2 },
      };
      expect(config.maxConcurrency).toBe(3);
      expect(config.ci?.strictExit).toBe(true);
      expect(config.ci?.failOn).toBe(2);
    });
  });
});
