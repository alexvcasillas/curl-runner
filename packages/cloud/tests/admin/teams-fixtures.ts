import type { TeamDetail, TeamRow, TeamsPage } from '@/lib/admin-teams';

export const NOW = '2026-11-06T12:00:00Z';

export const ADA = 'a0000000-0000-4000-8000-000000000001';
export const GRACE = 'a0000000-0000-4000-8000-000000000002';
export const LINUS = 'a0000000-0000-4000-8000-000000000003';

const base: TeamRow = {
  slug: 'acme',
  name: 'Acme',
  created_at: '2026-03-02T09:00:00Z',
  owner: { email: 'ada@acme.example', name: 'Ada Lovelace' },
  members: 3,
  collections: 4,
  plan: 'team',
  plan_source: 'stripe',
  billing_status: 'active',
  billing_interval: 'annual',
  past_due_since: null,
  cancel_at: null,
  override_until: null,
  suspended: false,
  mrr_cents: 2400,
};

export const rows: TeamRow[] = [
  base,
  {
    ...base,
    slug: 'initech',
    name: 'Initech',
    owner: { email: 'peter@initech.example', name: null },
    past_due_since: '2026-11-01T12:00:00Z',
    mrr_cents: 7000,
  },
  {
    ...base,
    slug: 'umbrella',
    name: 'Umbrella',
    plan_source: 'override',
    override_until: '2027-03-31T23:59:59Z',
    mrr_cents: 0,
  },
  {
    ...base,
    slug: 'stark-labs',
    name: 'Stark Labs',
    plan_source: 'override',
    override_until: null,
    mrr_cents: 0,
  },
  {
    ...base,
    slug: 'globex',
    name: 'Globex Partners',
    plan: 'free',
    plan_source: 'none',
    billing_status: null,
    billing_interval: null,
    mrr_cents: 0,
    members: 2,
    collections: 1,
  },
  {
    ...base,
    slug: 'spam-tests',
    name: 'spam-tests',
    plan: 'free',
    plan_source: 'none',
    suspended: true,
    owner: null,
    mrr_cents: null,
  },
];

export const page: TeamsPage = { items: rows, next_cursor: 'CURSOR_2', total: 312 };

export const detail: TeamDetail = {
  team: {
    id: 't0000000-0000-4000-8000-000000000001',
    slug: 'acme',
    name: 'Acme',
    created_at: '2026-03-02T09:00:00Z',
    plan: 'team',
    plan_source: 'stripe',
    limits: { members: null, restricted: true },
  },
  stripe: {
    status: 'active',
    interval: 'annual',
    seats: 3,
    amount_cents: 28800,
    currency: 'usd',
    renews: '2027-11-05T00:00:00Z',
    cancel_at: null,
    past_due_since: null,
    subscription_id: 'sub_1QxN4kLm',
    customer_id: 'cus_R2pQ7',
  },
  override: null,
  plan_history: [
    {
      at: '2026-11-01T10:00:00Z',
      source: 'stripe',
      plan: 'team',
      admin: null,
      reason: null,
      detail: {},
    },
    {
      at: '2026-03-02T09:00:00Z',
      source: 'default',
      plan: 'free',
      admin: null,
      reason: null,
      detail: {},
    },
  ],
  members: [
    {
      user_id: ADA,
      email: 'ada@acme.example',
      name: 'Ada Lovelace',
      role: 'owner',
      joined_at: '2026-03-02T09:00:00Z',
    },
    {
      user_id: GRACE,
      email: 'grace@acme.example',
      name: 'Grace Hopper',
      role: 'owner',
      joined_at: '2026-03-03T09:00:00Z',
    },
    {
      user_id: LINUS,
      email: 'linus@acme.example',
      name: 'Linus Torvalds',
      role: 'member',
      joined_at: '2026-03-04T09:00:00Z',
    },
  ],
  pending: [
    {
      id: 'p0000000-0000-4000-8000-000000000001',
      email: 'dana@acme.example',
      role: 'member',
      expires_at: '2026-11-10T09:00:00Z',
    },
  ],
  collections: [
    {
      slug: 'payments-api',
      name: 'Payments API',
      source: 'cli',
      access: 'public',
      current_version: 14,
      updated_at: '2026-11-04T09:00:00Z',
      link_blocked: false,
    },
    {
      slug: 'internal-admin',
      name: 'Internal admin',
      source: 'cli',
      access: 'off',
      current_version: 1,
      updated_at: '2026-11-04T09:00:00Z',
      link_blocked: false,
    },
  ],
  usage: { tokens_used_this_month: 14, publishes_this_month: 1206 },
  suspension: null,
};

export const granted: TeamDetail = {
  ...detail,
  team: { ...detail.team, plan_source: 'override' },
  override: {
    plan: 'team',
    until: '2027-03-31T23:59:59Z',
    reason: 'Design partner, pilot until the end of Q1',
    by: 'alex@example.com',
    at: '2026-11-05T10:00:00Z',
    reminded_at: null,
    active: true,
  },
};

export const free: TeamDetail = {
  ...detail,
  team: {
    ...detail.team,
    slug: 'globex',
    name: 'Globex Partners',
    plan: 'free',
    plan_source: 'none',
    limits: { members: 3, restricted: false },
  },
  stripe: null,
};

export const suspended: TeamDetail = {
  ...free,
  team: { ...free.team, slug: 'spam-tests', name: 'spam-tests' },
  suspension: {
    at: '2026-10-29T10:00:00Z',
    reason: 'Phishing pages in shared collections',
    by: 'rosa@example.com',
  },
};
