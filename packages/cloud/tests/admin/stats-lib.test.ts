import { describe, expect, it } from 'vitest';
import {
  agoShort,
  eventKind,
  formatDelta,
  formatStorage,
  formatUptime,
  jobLabel,
} from '@/lib/admin-stats';

describe('formatDelta', () => {
  it('signs MRR changes, with a real minus', () => {
    expect(formatDelta(1600)).toBe('+$16');
    expect(formatDelta(-1000)).toBe('−$10');
    expect(formatDelta(0)).toBe('$0');
    expect(formatDelta(null)).toBe('—');
    expect(formatDelta(1650)).toBe('+$16.50');
  });
});

describe('eventKind', () => {
  it('names a Stripe event the way the board does', () => {
    expect(eventKind('customer.subscription.created', 'Team annual, 2 seats')).toEqual({
      label: 'New',
      tone: 'ok',
    });
    expect(eventKind('customer.subscription.updated', 'Seats 2 to 3')).toEqual({
      label: 'Seats',
      tone: 'neutral',
    });
    expect(eventKind('customer.subscription.updated', 'Switched to annual, 3 seats')).toEqual({
      label: 'Interval',
      tone: 'neutral',
    });
    expect(eventKind('customer.subscription.updated', 'Payment failed, 3 seats')).toEqual({
      label: 'Payment failed',
      tone: 'err',
    });
    expect(eventKind('customer.subscription.deleted', 'Cancelled')).toEqual({
      label: 'Ended',
      tone: 'neutral',
    });
    expect(eventKind('invoice.payment_failed', null)).toEqual({
      label: 'Payment failed',
      tone: 'err',
    });
    expect(eventKind('something.else', null)).toEqual({ label: 'something.else', tone: 'neutral' });
  });
});

describe('health wording', () => {
  const now = new Date('2026-11-06T12:00:00Z');
  it('says how long ago in the coarsest honest unit', () => {
    expect(agoShort('2026-11-06T11:59:56Z', now)).toBe('4 s ago');
    expect(agoShort('2026-11-06T11:58:00Z', now)).toBe('2 min ago');
    expect(agoShort('2026-11-06T09:00:00Z', now)).toBe('3 h ago');
    expect(agoShort('2026-11-03T12:00:00Z', now)).toBe('3 days ago');
  });
  it('formats uptime, sizes and job names', () => {
    expect(formatUptime(45)).toBe('45 s');
    expect(formatUptime(3600 * 5 + 120)).toBe('5 h 2 min');
    expect(formatUptime(86400 * 4 + 3600)).toBe('4 days');
    expect(formatStorage(1_500_000_000)).toBe('1.4 GB');
    expect(formatStorage(52_428_800)).toBe('50 MB');
    expect(jobLabel('mail_worker')).toBe('Email outbox');
    expect(jobLabel('mystery_job')).toBe('mystery job');
  });
});
