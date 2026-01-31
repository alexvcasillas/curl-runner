import { describe, expect, mock, test } from 'bun:test';
import { calculateBackoffDelay, withRetry } from './retry';

describe('calculateBackoffDelay', () => {
  test('returns base delay for first attempt', () => {
    expect(calculateBackoffDelay(1, 1000, 2)).toBe(1000);
  });

  test('applies backoff for subsequent attempts', () => {
    expect(calculateBackoffDelay(2, 1000, 2)).toBe(2000);
    expect(calculateBackoffDelay(3, 1000, 2)).toBe(4000);
    expect(calculateBackoffDelay(4, 1000, 2)).toBe(8000);
  });

  test('handles backoff multiplier of 1 (no backoff)', () => {
    expect(calculateBackoffDelay(1, 1000, 1)).toBe(1000);
    expect(calculateBackoffDelay(2, 1000, 1)).toBe(1000);
    expect(calculateBackoffDelay(3, 1000, 1)).toBe(1000);
  });

  test('handles fractional backoff', () => {
    expect(calculateBackoffDelay(2, 1000, 1.5)).toBe(1500);
    expect(calculateBackoffDelay(3, 1000, 1.5)).toBe(2250);
  });

  test('handles zero delay', () => {
    expect(calculateBackoffDelay(1, 0, 2)).toBe(0);
    expect(calculateBackoffDelay(5, 0, 2)).toBe(0);
  });
});

describe('withRetry', () => {
  describe('successful operations', () => {
    test('returns result on first attempt success', async () => {
      const operation = mock(() => Promise.resolve('success'));

      const result = await withRetry(operation);

      expect(result.success).toBe(true);
      expect(result.value).toBe('success');
      expect(result.attempts).toBe(1);
      expect(operation).toHaveBeenCalledTimes(1);
    });

    test('returns result with object value', async () => {
      const data = { id: 1, name: 'test' };
      const operation = mock(() => Promise.resolve(data));

      const result = await withRetry(operation);

      expect(result.success).toBe(true);
      expect(result.value).toEqual(data);
    });
  });

  describe('failed operations', () => {
    test('returns failure without retries', async () => {
      const operation = mock(() => Promise.reject(new Error('failed')));

      const result = await withRetry(operation);

      expect(result.success).toBe(false);
      expect(result.error).toBe('failed');
      expect(result.attempts).toBe(1);
      expect(operation).toHaveBeenCalledTimes(1);
    });

    test('retries on failure with count config', async () => {
      const operation = mock(() => Promise.reject(new Error('failed')));

      const result = await withRetry(operation, {
        config: { count: 2 },
      });

      expect(result.success).toBe(false);
      expect(result.attempts).toBe(3); // 1 initial + 2 retries
      expect(operation).toHaveBeenCalledTimes(3);
    });

    test('succeeds after retries', async () => {
      let callCount = 0;
      const operation = mock(() => {
        callCount++;
        if (callCount < 3) {
          return Promise.reject(new Error('temporary failure'));
        }
        return Promise.resolve('success');
      });

      const result = await withRetry(operation, {
        config: { count: 3 },
      });

      expect(result.success).toBe(true);
      expect(result.value).toBe('success');
      expect(result.attempts).toBe(3);
    });
  });

  describe('shouldRetry callback', () => {
    test('retries when shouldRetry returns true', async () => {
      let callCount = 0;
      const operation = mock(() => {
        callCount++;
        return Promise.resolve({ success: callCount >= 2 });
      });

      const result = await withRetry(operation, {
        config: { count: 3 },
        shouldRetry: (res) => !res.success,
      });

      expect(result.success).toBe(true);
      expect(result.value).toEqual({ success: true });
      expect(operation).toHaveBeenCalledTimes(2);
    });

    test('does not retry when shouldRetry returns false', async () => {
      const operation = mock(() => Promise.resolve({ status: 200 }));

      const result = await withRetry(operation, {
        config: { count: 3 },
        shouldRetry: (res) => res.status !== 200,
      });

      expect(result.success).toBe(true);
      expect(operation).toHaveBeenCalledTimes(1);
    });

    test('exhausts retries with shouldRetry', async () => {
      const operation = mock(() => Promise.resolve({ status: 500 }));

      const result = await withRetry(operation, {
        config: { count: 2 },
        shouldRetry: (res) => res.status === 500,
      });

      expect(result.success).toBe(false);
      expect(result.attempts).toBe(3);
      expect(result.error).toBe('Operation returned retriable result');
    });
  });

  describe('onRetry callback', () => {
    test('calls onRetry for each retry attempt', async () => {
      const onRetry = mock((_attempt: number, _max: number) => {});
      const operation = mock(() => Promise.reject(new Error('failed')));

      await withRetry(operation, {
        config: { count: 3 },
        onRetry,
      });

      expect(onRetry).toHaveBeenCalledTimes(3);
      expect(onRetry).toHaveBeenNthCalledWith(1, 1, 3);
      expect(onRetry).toHaveBeenNthCalledWith(2, 2, 3);
      expect(onRetry).toHaveBeenNthCalledWith(3, 3, 3);
    });

    test('does not call onRetry on first attempt', async () => {
      const onRetry = mock((_attempt: number, _max: number) => {});
      const operation = mock(() => Promise.resolve('success'));

      await withRetry(operation, {
        config: { count: 3 },
        onRetry,
      });

      expect(onRetry).not.toHaveBeenCalled();
    });
  });

  describe('delay and backoff', () => {
    test('respects delay between retries', async () => {
      const startTime = performance.now();
      const operation = mock(() => Promise.reject(new Error('failed')));

      await withRetry(operation, {
        config: { count: 2, delay: 50 },
      });

      const elapsed = performance.now() - startTime;
      // Should have waited ~100ms (50 + 50)
      expect(elapsed).toBeGreaterThanOrEqual(90);
    });

    test('applies exponential backoff', async () => {
      const startTime = performance.now();
      const operation = mock(() => Promise.reject(new Error('failed')));

      await withRetry(operation, {
        config: { count: 2, delay: 50, backoff: 2 },
      });

      const elapsed = performance.now() - startTime;
      // Should have waited ~150ms (50 + 100)
      expect(elapsed).toBeGreaterThanOrEqual(140);
    });
  });

  describe('edge cases', () => {
    test('handles non-Error exceptions', async () => {
      const operation = mock(() => Promise.reject('string error'));

      const result = await withRetry(operation);

      expect(result.success).toBe(false);
      expect(result.error).toBe('string error');
    });

    test('handles undefined config', async () => {
      const operation = mock(() => Promise.resolve('success'));

      const result = await withRetry(operation, {});

      expect(result.success).toBe(true);
      expect(result.value).toBe('success');
    });

    test('handles count of 0 (no retries)', async () => {
      const operation = mock(() => Promise.reject(new Error('failed')));

      const result = await withRetry(operation, {
        config: { count: 0 },
      });

      expect(result.success).toBe(false);
      expect(result.attempts).toBe(1);
      expect(operation).toHaveBeenCalledTimes(1);
    });
  });
});
