import type { ProfileStats } from '../types/config';

/**
 * Calculate percentile from sorted array.
 * Uses linear interpolation for non-integer indices.
 */
export function calculatePercentile(sorted: number[], percentile: number): number {
  if (sorted.length === 0) {
    return 0;
  }
  if (sorted.length === 1) {
    return sorted[0];
  }

  const index = (percentile / 100) * (sorted.length - 1);
  const lower = Math.floor(index);
  const upper = Math.ceil(index);
  const fraction = index - lower;

  if (lower === upper) {
    return sorted[lower];
  }
  return sorted[lower] * (1 - fraction) + sorted[upper] * fraction;
}

/**
 * Calculate arithmetic mean.
 */
export function calculateMean(values: number[]): number {
  if (values.length === 0) {
    return 0;
  }
  return values.reduce((sum, v) => sum + v, 0) / values.length;
}

/**
 * Calculate standard deviation.
 */
export function calculateStdDev(values: number[], mean: number): number {
  if (values.length <= 1) {
    return 0;
  }
  const squaredDiffs = values.map((v) => (v - mean) ** 2);
  const variance = squaredDiffs.reduce((sum, v) => sum + v, 0) / values.length;
  return Math.sqrt(variance);
}

/**
 * Calculate profile statistics from raw timings.
 */
export function calculateProfileStats(
  timings: number[],
  warmup: number,
  failures: number,
): ProfileStats {
  // Exclude warmup iterations
  const effectiveTimings = timings.slice(warmup);
  const sorted = [...effectiveTimings].sort((a, b) => a - b);

  const mean = calculateMean(sorted);
  const totalIterations = timings.length;
  const effectiveIterations = effectiveTimings.length;

  return {
    iterations: effectiveIterations,
    warmup,
    min: sorted.length > 0 ? sorted[0] : 0,
    max: sorted.length > 0 ? sorted[sorted.length - 1] : 0,
    mean: Math.round(mean * 100) / 100,
    median: Math.round(calculatePercentile(sorted, 50) * 100) / 100,
    p50: Math.round(calculatePercentile(sorted, 50) * 100) / 100,
    p95: Math.round(calculatePercentile(sorted, 95) * 100) / 100,
    p99: Math.round(calculatePercentile(sorted, 99) * 100) / 100,
    stdDev: Math.round(calculateStdDev(sorted, mean) * 100) / 100,
    failures,
    failureRate: totalIterations > 0 ? Math.round((failures / totalIterations) * 10000) / 100 : 0,
    timings: effectiveTimings,
  };
}

/**
 * Generate ASCII histogram for latency distribution.
 */
export function generateHistogram(timings: number[], buckets = 10, width = 40): string[] {
  if (timings.length === 0) {
    return ['No data'];
  }

  const min = Math.min(...timings);
  const max = Math.max(...timings);
  const range = max - min || 1;
  const bucketSize = range / buckets;

  // Count values per bucket
  const counts = new Array(buckets).fill(0);
  for (const t of timings) {
    const bucket = Math.min(Math.floor((t - min) / bucketSize), buckets - 1);
    counts[bucket]++;
  }

  const maxCount = Math.max(...counts);
  const lines: string[] = [];

  for (let i = 0; i < buckets; i++) {
    const bucketMin = min + i * bucketSize;
    const bucketMax = min + (i + 1) * bucketSize;
    const barLength = maxCount > 0 ? Math.round((counts[i] / maxCount) * width) : 0;
    const bar = '█'.repeat(barLength);
    const label = `${bucketMin.toFixed(0).padStart(6)}ms - ${bucketMax.toFixed(0).padStart(6)}ms`;
    lines.push(`${label} │${bar} ${counts[i]}`);
  }

  return lines;
}

/**
 * Export stats to CSV format.
 */
export function exportToCSV(stats: ProfileStats, _requestName: string): string {
  const headers = ['iteration', 'latency_ms'];
  const rows = stats.timings.map((t, i) => `${i + 1},${t}`);
  return [headers.join(','), ...rows].join('\n');
}

/**
 * Export stats to JSON format.
 */
export function exportToJSON(stats: ProfileStats, requestName: string): string {
  return JSON.stringify(
    {
      request: requestName,
      summary: {
        iterations: stats.iterations,
        warmup: stats.warmup,
        failures: stats.failures,
        failureRate: stats.failureRate,
        min: stats.min,
        max: stats.max,
        mean: stats.mean,
        median: stats.median,
        p50: stats.p50,
        p95: stats.p95,
        p99: stats.p99,
        stdDev: stats.stdDev,
      },
      timings: stats.timings,
    },
    null,
    2,
  );
}
