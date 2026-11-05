import { describe, expect, it } from 'vitest';
import { ApiError, messageFor } from '@/lib/errors';

describe('messageFor', () => {
  it('uses the API message', () => {
    expect(messageFor(new ApiError(409, 'conflict', 'That slug is taken.'))).toBe(
      'That slug is taken.',
    );
  });

  it('prefers a page override for the code', () => {
    const e = new ApiError(403, 'forbidden', 'Forbidden.');
    expect(messageFor(e, { forbidden: 'Only team owners can do this.' })).toBe(
      'Only team owners can do this.',
    );
  });

  it('falls back to copy for the code without a message', () => {
    expect(messageFor(new ApiError(503, 'unavailable', ''))).toBe(
      'Cloud isn’t answering right now. Try again in a minute.',
    );
  });

  it('names the wait for rate limits', () => {
    expect(messageFor(new ApiError(429, 'rate_limited', 'Slow down.', 720))).toBe(
      'Too many tries. Try again in 12 minutes.',
    );
    expect(messageFor(new ApiError(429, 'rate_limited', 'Slow down.', 30))).toBe(
      'Too many tries. Try again in 30 seconds.',
    );
    expect(messageFor(new ApiError(429, 'rate_limited', 'Slow down.', 60))).toBe(
      'Too many tries. Try again in 1 minute.',
    );
  });
});
