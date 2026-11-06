import type { AdminOverview } from '@/lib/admin';

const days = (n: number, f: (i: number) => number) =>
  Array.from({ length: n }, (_, i) => ({
    day: new Date(Date.UTC(2026, 10, 6 - (n - 1 - i))).toISOString().slice(0, 10),
    n: f(i),
  }));

export const NOW = new Date('2026-11-06T12:00:00Z');

export const overview: AdminOverview = {
  billing: true,
  counts: {
    users: 1248,
    teams: 312,
    collections: 1904,
    collections_public: 611,
    collections_restricted: 284,
    access_list_people: 5730,
    plan_overrides: 6,
    plan_overrides_ending: 2,
    granted_teams: 3,
    granted_seats: 9,
    paying_teams: 41,
    paying_annual: 29,
    paying_monthly: 12,
    mrr_cents: 241600,
  },
  past_due: {
    teams: 1,
    next: { team: 'initech', name: 'Initech', deadline: '2026-11-12T12:00:00Z' },
  },
  signups_30d: days(30, (i) => (i === 29 ? 11 : 7)),
  teams_30d: days(30, (i) => (i % 4 === 0 ? 2 : 1)),
  publishes_14d: days(14, (i) => 2000 + i * 100),
  recent_events: [
    {
      id: 'e1',
      admin_email: 'ada@example.com',
      action: 'team.override.grant',
      target_kind: 'team',
      target_id: 't1',
      target_label: 'umbrella',
      at: '2026-11-06T11:52:00Z',
    },
    {
      id: 'e2',
      admin_email: 'rosa.mendez@example.com',
      action: 'collection.link.disable',
      target_kind: 'collection',
      target_id: 'c1',
      target_label: 'spam-tests',
      at: '2026-11-05T09:00:00Z',
    },
  ],
  p95_ms: 84,
  outbox: { waiting: 3, failed_today: 0 },
  last_webhook_at: '2026-11-06T11:58:00Z',
  signup_mode: 'closed',
};
