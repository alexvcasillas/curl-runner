import { describe, expect, it } from 'vitest';
import { seatCostNote, seatNote } from '@/lib/members';
import {
  isExpiry,
  isRole,
  isUuid,
  normalizeEmail,
  parseTeamName,
  parseTokenName,
} from '@/lib/team-input';
import { expiresIn, formatDate, tokenExpiry } from '@/lib/time';

describe('team inputs', () => {
  it('normalizes an invite email and refuses domains and junk', () => {
    expect(normalizeEmail(' Ada@Example.COM ')).toBe('ada@example.com');
    for (const bad of [
      '*@example.com',
      'ada',
      'a@b@c.com',
      'ada @x.com',
      '',
      `${'a'.repeat(250)}@x.com`,
    ]) {
      expect(normalizeEmail(bad), bad).toBeNull();
    }
  });

  it('accepts only the roles, expiries and uuids the page can send', () => {
    expect(isRole('owner')).toBe(true);
    expect(isRole('member')).toBe(true);
    expect(isRole('admin')).toBe(false);
    expect(isRole(undefined)).toBe(false);
    for (const ok of [null, 30, 90, 365]) {
      expect(isExpiry(ok)).toBe(true);
    }
    for (const bad of [0, 7, 3650, '30', undefined, -1]) {
      expect(isExpiry(bad)).toBe(false);
    }
    expect(isUuid('3f2b8a52-8f2d-4a3e-9f53-0e2d2f5a7c11')).toBe(true);
    expect(isUuid('../etc')).toBe(false);
  });

  it('trims names and caps them at 100 characters', () => {
    expect(parseTokenName('  github-actions ')).toBe('github-actions');
    expect(parseTokenName('   ')).toBeNull();
    expect(parseTokenName('x'.repeat(101))).toBeNull();
    expect(parseTokenName('x'.repeat(100))).toBe('x'.repeat(100));
    expect(parseTokenName(3)).toBeNull();
    expect(parseTeamName(' Acme ')).toBe('Acme');
    expect(parseTeamName('')).toBeNull();
    expect(parseTeamName('y'.repeat(101))).toBeNull();
  });
});

describe('dates', () => {
  const now = new Date('2026-11-05T10:00:00Z');
  it('formats a day', () => {
    expect(formatDate('2027-02-01T00:00:00Z')).toBe('Feb 1, 2027');
  });
  it('names a token expiry', () => {
    expect(tokenExpiry(null, now)).toEqual({ label: 'Never', soon: false });
    expect(tokenExpiry('2026-11-14T10:00:00Z', now)).toEqual({ label: 'In 9 days', soon: true });
    expect(tokenExpiry('2026-11-06T09:00:00Z', now)).toEqual({ label: 'In 1 day', soon: true });
    expect(tokenExpiry('2027-02-01T00:00:00Z', now)).toEqual({ label: 'Feb 1, 2027', soon: false });
  });
  it('names an invite expiry', () => {
    expect(expiresIn('2026-11-10T10:00:00Z', now)).toBe('Expires in 5 days');
    expect(expiresIn('2026-11-06T10:00:00Z', now)).toBe('Expires in 1 day');
    expect(expiresIn('2026-11-05T15:00:00Z', now)).toBe('Expires in 5 hours');
    expect(expiresIn('2026-11-05T10:10:00Z', now)).toBe('Expires soon');
  });
});

describe('seat notes', () => {
  it('counts the Free plan seats, pending invites included', () => {
    expect(seatNote('free', true, 2, 1)).toBe(
      '2 members and 1 pending invite: 3 of 3 on the Free plan.',
    );
    expect(seatNote('free', false, 1, 0)).toBe('1 member: 1 of 3 on the Free plan.');
    expect(seatNote('free', true, 1, 2)).toBe(
      '1 member and 2 pending invites: 3 of 3 on the Free plan.',
    );
  });
  it('says what Team seats mean', () => {
    expect(seatNote('team', true, 3, 0)).toBe(
      '3 members on the Team plan, billed per member. Customers are never seats.',
    );
    expect(seatNote('team', false, 3, 0)).toBe('3 members. Customers are never seats.');
  });

  const prices = {
    monthly: { unit_amount: 1000, currency: 'usd' },
    annual: { unit_amount: 9600, currency: 'usd' },
  };
  const summary = (interval: string | null) => ({
    plan: 'team' as const,
    interval,
    status: 'active',
    seats: 3,
    past_due_since: null,
    can_manage: true,
    cancel_at: null,
    current_period_end: null,
    plan_source: 'stripe',
  });
  it('prices a seat on the annual plan as a month', () => {
    expect(seatCostNote(true, 'team', prices, summary('annual'))).toBe(
      'On the Team plan, each member who accepts adds $8 a month on the annual plan, prorated on the next invoice. Pending invites aren’t billed.',
    );
  });
  it('prices a seat on the monthly plan', () => {
    expect(seatCostNote(true, 'team', prices, summary('monthly'))).toBe(
      'On the Team plan, each member who accepts adds $10 a month on the monthly plan, prorated on the next invoice. Pending invites aren’t billed.',
    );
  });
  it('keeps cents when the month is not a round number', () => {
    const p = { ...prices, annual: { unit_amount: 10000, currency: 'usd' } };
    expect(seatCostNote(true, 'team', p, summary('annual'))).toContain('$8.33 a month');
  });
  it('says nothing without billing, a Team plan, a subscription interval or prices', () => {
    expect(seatCostNote(false, 'team', prices, summary('annual'))).toBeNull();
    expect(seatCostNote(true, 'free', prices, summary('annual'))).toBeNull();
    expect(seatCostNote(true, 'team', prices, summary(null))).toBeNull();
    expect(seatCostNote(true, 'team', null, summary('annual'))).toBeNull();
    expect(seatCostNote(true, 'team', prices, null)).toBeNull();
  });
});
