import type { ActivityPage } from '@/lib/admin-activity';
import type { AdminRow, SettingsData } from '@/lib/admin-settings';
import type { Health, StripeStats } from '@/lib/admin-stats';

export const NOW = new Date('2026-11-06T12:00:05Z');
export const ROSA = 'a0000000-0000-4000-8000-000000000002';
export const MARCO = 'a0000000-0000-4000-8000-000000000003';
export const ADA = 'a0000000-0000-4000-8000-000000000001';

export const settings: SettingsData = {
  signup_mode: 'closed',
  signup_domains: [],
  default_plan: 'free',
  free_member_limit: 3,
  free_restricted: false,
  grace_days: 14,
  banner_message: null,
  banner_tone: 'info',
  banner_audience: 'signed_in',
  banner_link: null,
  banner_version: 1,
  publishing_paused: false,
  updated_at: null,
  updated_by_email: null,
  env: [
    { key: 'PUBLIC_APP_URL', value: 'https://cloud.curl-runner.com', secret: false },
    { key: 'PUBLIC_API_URL', value: 'https://api.curl-runner.com', secret: false },
    { key: 'MAIL_FROM', value: 'curl-runner <no-reply@curl-runner.com>', secret: false },
    { key: 'SMTP_HOST', value: 'smtp.example.com', secret: false },
    { key: 'SMTP_PORT', value: '587', secret: false },
    { key: 'STRIPE_PRICE_MONTHLY', value: null, secret: false },
    { key: 'SMTP_PASSWORD', value: null, secret: true },
    { key: 'GITHUB_CLIENT_ID', value: null, secret: true },
    { key: 'GOOGLE_CLIENT_ID', value: null, secret: true },
    { key: 'STRIPE_SECRET_KEY', value: null, secret: true },
    { key: 'OPERATOR_EMAILS', value: null, secret: true },
  ],
};

export const admins: AdminRow[] = [
  {
    user_id: ADA,
    email: 'ada@example.com',
    name: 'Ada',
    role: 'owner',
    permissions: ['users', 'teams', 'billing', 'emails', 'settings', 'admins'],
    last_active: '2026-11-06T12:00:00Z',
    added_by: null,
    added_at: null,
  },
  {
    user_id: ROSA,
    email: 'rosa@example.com',
    name: 'Rosa Méndez',
    role: 'admin',
    permissions: ['users', 'teams', 'emails'],
    last_active: '2026-11-06T09:00:00Z',
    added_by: { user_id: ADA, email: 'ada@example.com', name: 'Ada' },
    added_at: '2026-11-02T10:00:00Z',
  },
  {
    user_id: MARCO,
    email: 'marco@example.com',
    name: null,
    role: 'admin',
    permissions: ['teams'],
    last_active: null,
    added_by: { user_id: ADA, email: 'ada@example.com', name: null },
    added_at: '2026-11-04T10:00:00Z',
  },
];

export const activity: ActivityPage = {
  total: 412,
  next_cursor: 'cur2',
  items: [
    {
      id: 'e1',
      at: '2026-11-05T14:08:00Z',
      admin: { user_id: ADA, email: 'ada@example.com', name: 'Ada' },
      action: 'team.override.grant',
      target_kind: 'team',
      target_id: 't1',
      target_label: 'umbrella',
      detail: { plan: 'free → team', until: '2027-03-31', emailed_owners: true },
      ip: '83.44.12.9',
    },
    {
      id: 'e2',
      at: '2026-11-05T11:02:00Z',
      admin: { user_id: ROSA, email: 'rosa@example.com', name: 'Rosa Méndez' },
      action: 'collection.link.disable',
      target_kind: 'collection',
      target_id: 'c1',
      target_label: 'free-gift-cards',
      detail: { reason: '<img src=x onerror=alert(1)>' },
      ip: null,
    },
    {
      id: 'e3',
      at: '2026-11-04T09:12:00Z',
      admin: null,
      action: 'settings.update',
      target_kind: 'settings',
      target_id: 'signup',
      target_label: 'Server settings',
      detail: { section: 'signup', from: { mode: 'closed' }, to: { mode: 'domains' } },
      ip: '83.44.12.9',
    },
  ],
};

export const stripe: StripeStats = {
  currency: 'usd',
  mrr_cents: 241600,
  arr_cents: 2899200,
  paying_teams: 41,
  seats: 145,
  arpa: 5892,
  past_due: { teams: 1, at_risk_cents: 7000 },
  mrr_90d: Array.from({ length: 30 }, (_, i) => ({
    day: new Date(Date.UTC(2026, 9, 8 + i)).toISOString().slice(0, 10),
    mrr_cents: 138000 + i * 3600,
  })),
  net_new: {
    since: '2026-11-01',
    total_cents: 26400,
    new_cents: 21000,
    expansion_cents: 9600,
    churned_cents: -4200,
    cancelled_teams: 1,
  },
  by_source: [
    { source: 'stripe_annual', teams: 29, seats: 104 },
    { source: 'stripe_monthly', teams: 12, seats: 41 },
    { source: 'granted', teams: 6, seats: 38 },
  ],
  webhooks: { events_24h: 214, bad_signatures_24h: 0, duplicates_24h: 2 },
  events: [
    {
      id: 'evt_1QxN4',
      type: 'customer.subscription.created',
      team: { slug: 'acme', name: 'Acme' },
      summary: 'Team annual, 2 seats',
      mrr_delta_cents: 1600,
      at: '2026-11-02T10:00:00Z',
    },
    {
      id: 'evt_1QxK2',
      type: 'invoice.payment_failed',
      team: { slug: 'initech', name: 'Initech' },
      summary: 'Payment failed, 3 seats',
      mrr_delta_cents: 0,
      at: '2026-10-30T10:00:00Z',
    },
    {
      id: 'evt_1Qv61',
      type: 'customer.subscription.updated',
      team: null,
      summary: 'Switched to annual, 3 seats',
      mrr_delta_cents: -1000,
      at: '2026-10-22T10:00:00Z',
    },
  ],
};

export const health: Health = {
  stats: {
    version: '2.7.0',
    uptime_secs: 86400 * 4 + 3600,
    p95_ms: 84,
    requests_5m: 1200,
    errors_5xx_5m: 3,
    db: { size_bytes: 1_500_000_000, connections: 12, max_connections: 50, migration: 4 },
    email: { waiting: 3, oldest_waiting_secs: 40, failed_today: 1 },
  },
  jobs: [
    { name: 'mail_worker', last_run: '2026-11-06T12:00:01Z', status: 'ok' },
    { name: 'billing_sync', last_run: '2026-11-06T11:59:34Z', status: 'failed' },
    { name: 'override_expiry', last_run: null, status: 'never_run' },
    { name: 'mrr_snapshot', last_run: '2026-11-06T11:40:00Z', status: 'stale' },
  ],
  webhooks_24h: [
    { type: 'customer.subscription.updated', n: 96 },
    { type: 'invoice.paid', n: 61 },
  ],
  bad_signatures_24h: 0,
  duplicates_24h: 2,
  last_webhook_at: '2026-11-06T11:58:00Z',
  rate_limits: [
    { key: 'email-start:ip', what: 'Email sign-in starts, per IP', n: 41 },
    { key: 'shared:ip', what: 'Share link opens, per IP', n: 3 },
  ],
};
