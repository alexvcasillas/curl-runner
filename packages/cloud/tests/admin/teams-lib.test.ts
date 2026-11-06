import { describe, expect, it } from 'vitest';
import {
  dayOf,
  endOfDay,
  headerSource,
  historyLine,
  limitsLabel,
  parseTeamsQuery,
  sourceOf,
  stripeUrl,
  teamsExportHref,
  teamsHref,
} from '@/lib/admin-teams';
import { detail, free, granted, NOW, rows } from './teams-fixtures';

const now = new Date(NOW);

describe('parseTeamsQuery', () => {
  it('reads q, filter, cursor and trail', () => {
    expect(
      parseTeamsQuery({ q: ' acme ', filter: 'granted', cursor: 'C2', trail: '-.C1' }),
    ).toEqual({ q: 'acme', filter: 'granted', cursor: 'C2', trail: ['-', 'C1'] });
  });

  it('falls back for anything it does not know', () => {
    expect(parseTeamsQuery({ filter: 'bogus', cursor: 'a b!', trail: '..;' })).toEqual({
      q: '',
      filter: 'all',
      cursor: undefined,
      trail: [],
    });
  });

  it('caps q at 200 characters', () => {
    expect(parseTeamsQuery({ q: 'x'.repeat(300) }).q).toHaveLength(200);
  });
});

describe('hrefs', () => {
  it('keeps the list and the export in step', () => {
    expect(teamsHref({ q: '', filter: 'all', trail: [] })).toBe('/admin/teams');
    expect(teamsHref({ q: 'a b', filter: 'past_due', cursor: 'C', trail: ['-'] })).toBe(
      '/admin/teams?q=a+b&filter=past_due&cursor=C&trail=-',
    );
    expect(teamsExportHref({ q: '', filter: 'all' })).toBe('/admin/teams/export');
    expect(teamsExportHref({ q: 'x', filter: 'free' })).toBe('/admin/teams/export?q=x&filter=free');
  });
});

describe('sourceOf', () => {
  const label = (i: number) => sourceOf(rows[i], now).label;
  it('says it the way the board does', () => {
    expect(label(0)).toBe('Stripe · annual');
    expect(label(1)).toBe('Stripe · past due, 9 days left');
    expect(label(2)).toBe('Granted until Mar 31, 2027');
    expect(label(3)).toBe('Granted, no end');
    expect(label(4)).toBe('None');
    expect(label(5)).toBe('Suspended');
  });

  it('names a scheduled cancel and never goes below zero days', () => {
    expect(sourceOf({ ...rows[0], cancel_at: '2026-12-03T00:00:00Z' }, now).label).toBe(
      'Stripe · cancels Dec 3',
    );
    expect(sourceOf({ ...rows[1], past_due_since: '2026-09-01T00:00:00Z' }, now).label).toBe(
      'Stripe · past due, 0 days left',
    );
  });
});

describe('headerSource', () => {
  it('follows the effective source', () => {
    expect(headerSource(detail, now).label).toBe('Stripe · annual');
    expect(headerSource(granted, now).label).toBe('Granted until Mar 31, 2027');
    expect(headerSource(free, now).label).toBe('No subscription');
  });
});

describe('limitsLabel', () => {
  it('matches the board', () => {
    expect(limitsLabel({ members: 3, restricted: false })).toBe(
      '3 members, no restricted collections',
    );
    expect(limitsLabel({ members: null, restricted: true })).toBe('none');
  });
});

describe('stripeUrl', () => {
  it('builds dashboard links for Stripe-shaped ids only', () => {
    expect(stripeUrl('subscriptions', 'sub_1QxN4kLm')).toBe(
      'https://dashboard.stripe.com/subscriptions/sub_1QxN4kLm',
    );
    expect(stripeUrl('customers', 'cus_R2pQ7')).toBe(
      'https://dashboard.stripe.com/customers/cus_R2pQ7',
    );
    expect(stripeUrl('subscriptions', 'sub_x/../evil')).toBeNull();
    expect(stripeUrl('subscriptions', 'javascript:1')).toBeNull();
  });
});

describe('historyLine', () => {
  const h = (source: string, over = {}) => ({
    at: NOW,
    source,
    plan: 'team',
    admin: 'alex@curl-runner.com',
    reason: null,
    detail: {},
    ...over,
  });
  it('reads each source as a sentence', () => {
    expect(historyLine(h('default', { plan: 'free' }))).toBe('Created on Free');
    expect(historyLine(h('stripe'))).toBe('Stripe set the plan to Team');
    expect(historyLine(h('past_due', { plan: 'free' }))).toBe('Payment failed: back to Free');
    expect(historyLine(h('override', { detail: { until: '2027-03-31T23:59:59Z' } }))).toBe(
      'alex@curl-runner.com granted Team until Mar 31, 2027',
    );
    expect(
      historyLine(h('override', { detail: { until: null, action: 'team.override.update' } })),
    ).toBe('alex@curl-runner.com changed Team with no end date');
    expect(historyLine(h('override_removed', { plan: 'free' }))).toBe(
      'alex@curl-runner.com removed the override: back to Free',
    );
    expect(historyLine(h('expired', { plan: 'free', admin: null }))).toBe(
      'The override ended: back to Free',
    );
  });
});

describe('endOfDay and dayOf', () => {
  it('turns a day into the end of that day, UTC', () => {
    expect(endOfDay('2027-03-31')).toBe('2027-03-31T23:59:59.000Z');
    expect(dayOf('2027-03-31T23:59:59.000Z')).toBe('2027-03-31');
  });

  it('refuses anything else', () => {
    expect(endOfDay('')).toBeNull();
    expect(endOfDay('2027-02-31')).toBeNull();
    expect(endOfDay('31/03/2027')).toBeNull();
  });
});
