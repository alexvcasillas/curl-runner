import { describe, expect, test } from 'bun:test';
import {
  calculateMean,
  calculatePercentile,
  calculateProfileStats,
  calculateStdDev,
  exportToCSV,
  exportToJSON,
  generateHistogram,
} from './stats';

describe('calculatePercentile', () => {
  test('returns 0 for empty array', () => {
    expect(calculatePercentile([], 50)).toBe(0);
  });

  test('returns single value for array of 1', () => {
    expect(calculatePercentile([100], 50)).toBe(100);
    expect(calculatePercentile([100], 99)).toBe(100);
  });

  test('calculates p50 (median) correctly', () => {
    expect(calculatePercentile([1, 2, 3, 4, 5], 50)).toBe(3);
    expect(calculatePercentile([1, 2, 3, 4], 50)).toBe(2.5);
  });

  test('calculates p95 correctly', () => {
    const values = Array.from({ length: 100 }, (_, i) => i + 1);
    expect(calculatePercentile(values, 95)).toBeCloseTo(95.05, 1);
  });

  test('calculates p99 correctly', () => {
    const values = Array.from({ length: 100 }, (_, i) => i + 1);
    expect(calculatePercentile(values, 99)).toBeCloseTo(99.01, 1);
  });

  test('handles unsorted input (requires pre-sorting)', () => {
    // Note: function expects sorted input
    const sorted = [10, 20, 30, 40, 50].sort((a, b) => a - b);
    expect(calculatePercentile(sorted, 50)).toBe(30);
  });
});

describe('calculateMean', () => {
  test('returns 0 for empty array', () => {
    expect(calculateMean([])).toBe(0);
  });

  test('calculates mean correctly', () => {
    expect(calculateMean([1, 2, 3, 4, 5])).toBe(3);
    expect(calculateMean([10, 20, 30])).toBe(20);
    expect(calculateMean([100])).toBe(100);
  });
});

describe('calculateStdDev', () => {
  test('returns 0 for empty array', () => {
    expect(calculateStdDev([], 0)).toBe(0);
  });

  test('returns 0 for single value', () => {
    expect(calculateStdDev([100], 100)).toBe(0);
  });

  test('calculates standard deviation correctly', () => {
    const values = [2, 4, 4, 4, 5, 5, 7, 9];
    const mean = calculateMean(values);
    expect(calculateStdDev(values, mean)).toBeCloseTo(2, 0);
  });
});

describe('calculateProfileStats', () => {
  test('calculates stats correctly with no warmup', () => {
    const timings = [10, 20, 30, 40, 50];
    const stats = calculateProfileStats(timings, 0, 0);

    expect(stats.iterations).toBe(5);
    expect(stats.warmup).toBe(0);
    expect(stats.min).toBe(10);
    expect(stats.max).toBe(50);
    expect(stats.mean).toBe(30);
    expect(stats.failures).toBe(0);
    expect(stats.failureRate).toBe(0);
  });

  test('excludes warmup iterations from stats', () => {
    const timings = [100, 10, 20, 30, 40]; // First value is warmup outlier
    const stats = calculateProfileStats(timings, 1, 0);

    expect(stats.iterations).toBe(4);
    expect(stats.warmup).toBe(1);
    expect(stats.min).toBe(10);
    expect(stats.max).toBe(40);
    expect(stats.mean).toBe(25);
  });

  test('calculates failure rate correctly', () => {
    const timings = [10, 20, 30];
    const stats = calculateProfileStats(timings, 0, 2);

    expect(stats.failures).toBe(2);
    expect(stats.failureRate).toBeCloseTo(66.67, 1);
  });

  test('handles empty timings', () => {
    const stats = calculateProfileStats([], 0, 0);

    expect(stats.iterations).toBe(0);
    expect(stats.min).toBe(0);
    expect(stats.max).toBe(0);
    expect(stats.mean).toBe(0);
  });
});

describe('generateHistogram', () => {
  test('returns "No data" for empty array', () => {
    const result = generateHistogram([]);
    expect(result).toEqual(['No data']);
  });

  test('generates histogram with correct bucket count', () => {
    const timings = Array.from({ length: 100 }, (_, i) => i);
    const result = generateHistogram(timings, 5, 20);

    expect(result.length).toBe(5);
  });

  test('histogram lines contain bucket ranges', () => {
    const timings = [10, 20, 30, 40, 50];
    const result = generateHistogram(timings, 2, 10);

    expect(result[0]).toContain('ms -');
    expect(result[0]).toContain('ms │');
  });
});

describe('exportToCSV', () => {
  test('exports stats to CSV format', () => {
    const stats = calculateProfileStats([10, 20, 30], 0, 0);
    const csv = exportToCSV(stats, 'Test Request');

    expect(csv).toContain('iteration,latency_ms');
    expect(csv).toContain('1,10');
    expect(csv).toContain('2,20');
    expect(csv).toContain('3,30');
  });
});

describe('exportToJSON', () => {
  test('exports stats to JSON format', () => {
    const stats = calculateProfileStats([10, 20, 30], 0, 0);
    const json = exportToJSON(stats, 'Test Request');
    const parsed = JSON.parse(json);

    expect(parsed.request).toBe('Test Request');
    expect(parsed.summary.iterations).toBe(3);
    expect(parsed.summary.min).toBe(10);
    expect(parsed.summary.max).toBe(30);
    expect(parsed.timings).toEqual([10, 20, 30]);
  });
});
