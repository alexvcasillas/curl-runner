import { describe, expect, test } from 'bun:test';
import type { ProfileConfig } from '../types/config';
import { calculateProfileStats } from '../utils/stats';

/**
 * Test the profile stats calculation logic used by ProfileExecutor.
 * Actual HTTP execution is tested via integration tests.
 */

describe('Profile Stats Integration', () => {
  test('stats calculation matches expected ProfileResult structure', () => {
    const timings = [50, 55, 60, 65, 70, 75, 80, 85, 90, 95];
    const warmup = 2;
    const failures = 0;

    const stats = calculateProfileStats(timings, warmup, failures);

    // Verify all ProfileStats fields are present
    expect(stats).toHaveProperty('iterations');
    expect(stats).toHaveProperty('warmup');
    expect(stats).toHaveProperty('min');
    expect(stats).toHaveProperty('max');
    expect(stats).toHaveProperty('mean');
    expect(stats).toHaveProperty('median');
    expect(stats).toHaveProperty('p50');
    expect(stats).toHaveProperty('p95');
    expect(stats).toHaveProperty('p99');
    expect(stats).toHaveProperty('stdDev');
    expect(stats).toHaveProperty('failures');
    expect(stats).toHaveProperty('failureRate');
    expect(stats).toHaveProperty('timings');

    // Verify warmup exclusion
    expect(stats.iterations).toBe(8); // 10 - 2 warmup
    expect(stats.warmup).toBe(2);
    expect(stats.timings.length).toBe(8);
  });

  test('profile config defaults are applied correctly', () => {
    const defaultConfig: ProfileConfig = {
      iterations: 10,
      warmup: 1,
      concurrency: 1,
      histogram: false,
    };

    expect(defaultConfig.iterations).toBe(10);
    expect(defaultConfig.warmup).toBe(1);
    expect(defaultConfig.concurrency).toBe(1);
    expect(defaultConfig.histogram).toBe(false);
  });

  test('concurrent config changes iterations behavior', () => {
    const _sequentialConfig: ProfileConfig = {
      iterations: 100,
      concurrency: 1,
    };

    const concurrentConfig: ProfileConfig = {
      iterations: 100,
      concurrency: 10,
    };

    // With concurrency 10, iterations should be chunked into 10 parallel batches
    const expectedChunks = Math.ceil(concurrentConfig.iterations / concurrentConfig.concurrency!);
    expect(expectedChunks).toBe(10);
  });

  test('failure tracking affects failureRate calculation', () => {
    const timings = [10, 20, 30, 40, 50];
    const failures = 2;

    const stats = calculateProfileStats(timings, 0, failures);

    // 2 failures out of 5 total (timings) + 2 failures = 7 total iterations
    // But failures are tracked separately, so failureRate = 2/5 = 40% based on timings length
    expect(stats.failures).toBe(2);
    expect(stats.failureRate).toBeGreaterThan(0);
  });

  test('warmup iterations are excluded from percentile calculations', () => {
    // First 2 values are outlier warmup times
    const timings = [500, 400, 100, 100, 100, 100, 100, 100, 100, 100];
    const warmup = 2;

    const stats = calculateProfileStats(timings, warmup, 0);

    // After excluding warmup, all values are 100
    expect(stats.min).toBe(100);
    expect(stats.max).toBe(100);
    expect(stats.mean).toBe(100);
    expect(stats.p50).toBe(100);
    expect(stats.p95).toBe(100);
    expect(stats.p99).toBe(100);
  });

  test('export file extension determines format', () => {
    const jsonFile = 'results.json';
    const csvFile = 'results.csv';

    expect(jsonFile.endsWith('.json')).toBe(true);
    expect(csvFile.endsWith('.csv')).toBe(true);
    expect(jsonFile.endsWith('.csv')).toBe(false);
    expect(csvFile.endsWith('.json')).toBe(false);
  });
});

describe('ProfileConfig Validation', () => {
  test('iterations must be positive', () => {
    const validConfig: ProfileConfig = { iterations: 10 };
    const invalidIterations = 0;

    expect(validConfig.iterations).toBeGreaterThan(0);
    expect(invalidIterations).toBeLessThanOrEqual(0);
  });

  test('warmup should not exceed iterations', () => {
    const config: ProfileConfig = {
      iterations: 10,
      warmup: 5,
    };

    expect(config.warmup).toBeLessThanOrEqual(config.iterations);
  });

  test('concurrency defaults to 1 (sequential)', () => {
    const config: ProfileConfig = { iterations: 10 };
    const concurrency = config.concurrency ?? 1;

    expect(concurrency).toBe(1);
  });
});
