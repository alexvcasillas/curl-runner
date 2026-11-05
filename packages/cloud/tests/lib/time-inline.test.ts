import { describe, expect, it } from 'vitest';
import { dateThisYear, relativeTimeInline } from '@/lib/time';

const now = new Date('2026-11-05T12:00:00Z');

describe('relativeTimeInline', () => {
  it('lowercases words but keeps dates', () => {
    expect(relativeTimeInline('2026-11-04T10:00:00Z', now)).toBe('yesterday');
    expect(relativeTimeInline('2026-11-05T11:59:50Z', now)).toBe('just now');
    expect(relativeTimeInline('2026-10-30T10:00:00Z', now)).toBe('Oct 30');
    expect(relativeTimeInline('2026-11-05T10:00:00Z', now)).toBe('2 hours ago');
  });
});

describe('dateThisYear', () => {
  it('drops the year for this year and keeps it for any other', () => {
    expect(dateThisYear('2026-03-02T10:00:00Z', now)).toBe('Mar 2');
    expect(dateThisYear('2025-10-30T10:00:00Z', now)).toBe('Oct 30, 2025');
    expect(dateThisYear('2027-01-01T00:00:00Z', now)).toBe('Jan 1, 2027');
  });
});
