// What the Stripe and Health pages share: wire shapes and wording.
// Safe to import from client components.

import type { PillTone } from '@/components/Pill';

// ---- GET /v1/admin/stripe ----

export type StripeEvent = {
  id: string;
  type: string;
  team: { slug: string; name: string } | null;
  summary: string | null;
  mrr_delta_cents: number | null;
  at: string;
};

export type StripeSource = {
  source: 'stripe_annual' | 'stripe_monthly' | 'granted';
  teams: number;
  seats: number;
};

export type StripeStats = {
  currency: string;
  mrr_cents: number;
  arr_cents: number;
  paying_teams: number;
  seats: number;
  arpa: number;
  past_due: { teams: number; at_risk_cents: number };
  mrr_90d: { day: string; mrr_cents: number }[];
  net_new: {
    since: string;
    total_cents: number;
    new_cents: number;
    expansion_cents: number;
    churned_cents: number;
    cancelled_teams: number;
  };
  by_source: StripeSource[];
  webhooks: { events_24h: number; bad_signatures_24h: number; duplicates_24h: number };
  events: StripeEvent[];
};

const usd = new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' });

/** "+$16", "−$10", "$0", or "—" when the server couldn't price the change. */
export function formatDelta(cents: number | null): string {
  if (cents === null) {
    return '—';
  }
  if (cents === 0) {
    return '$0';
  }
  const abs = Math.abs(cents) / 100;
  const text = usd.format(abs).replace(/\.00$/, '');
  return `${cents > 0 ? '+' : '−'}${text}`;
}

/** The board's pill for a Stripe event: what happened, in a word. */
export function eventKind(type: string, summary: string | null): { label: string; tone: PillTone } {
  const s = summary ?? '';
  switch (type) {
    case 'customer.subscription.created':
    case 'checkout.session.completed':
      return { label: 'New', tone: 'ok' };
    case 'customer.subscription.deleted':
      return { label: 'Ended', tone: 'neutral' };
    case 'invoice.payment_failed':
      return { label: 'Payment failed', tone: 'err' };
    case 'customer.subscription.updated':
      if (s.startsWith('Payment failed')) {
        return { label: 'Payment failed', tone: 'err' };
      }
      if (s.startsWith('Seats')) {
        return { label: 'Seats', tone: 'neutral' };
      }
      if (s.startsWith('Switched')) {
        return { label: 'Interval', tone: 'neutral' };
      }
      if (s.startsWith('Cancel')) {
        return { label: 'Cancels', tone: 'warn' };
      }
      return { label: 'Updated', tone: 'neutral' };
    default:
      return { label: type, tone: 'neutral' };
  }
}

// ---- GET /v1/admin/health ----

export type HealthJob = {
  name: string;
  last_run: string | null;
  status: 'ok' | 'failed' | 'stale' | 'never_run';
};

export type Health = {
  stats: {
    version: string;
    uptime_secs: number;
    p95_ms: number | null;
    requests_5m: number;
    errors_5xx_5m: number;
    db: {
      size_bytes: number;
      connections: number;
      max_connections: number;
      migration: number | null;
    };
    email: { waiting: number; oldest_waiting_secs: number | null; failed_today: number };
  };
  jobs: HealthJob[];
  webhooks_24h: { type: string; n: number }[];
  bad_signatures_24h: number;
  duplicates_24h: number;
  last_webhook_at: string | null;
  rate_limits: { key: string; what: string; n: number }[];
};

const JOB_NAMES: Record<string, string> = {
  mail_worker: 'Email outbox',
  billing_sync: 'Seat sync with Stripe',
  override_expiry: 'Plan overrides ending',
  broadcast_fanout: 'Broadcast sending',
  mrr_snapshot: 'Daily MRR snapshot',
  audit_retention: 'Activity log clean-up',
};

export function jobLabel(name: string): string {
  return JOB_NAMES[name] ?? name.replace(/_/g, ' ');
}

const plural = (n: number, one: string) => `${n} ${n === 1 ? one : `${one}s`}`;

/** "4 s ago", "2 min ago", "3 h ago", "3 days ago". */
export function agoShort(iso: string, now: Date): string {
  const secs = Math.max(0, Math.floor((now.getTime() - new Date(iso).getTime()) / 1000));
  if (secs < 60) {
    return `${secs} s ago`;
  }
  if (secs < 3600) {
    return `${Math.floor(secs / 60)} min ago`;
  }
  if (secs < 86_400) {
    return `${Math.floor(secs / 3600)} h ago`;
  }
  return `${plural(Math.floor(secs / 86_400), 'day')} ago`;
}

/** "45 s", "5 h 2 min", "4 days". */
export function formatUptime(secs: number): string {
  if (secs < 60) {
    return `${secs} s`;
  }
  if (secs < 3600) {
    return `${Math.floor(secs / 60)} min`;
  }
  if (secs < 86_400) {
    return `${Math.floor(secs / 3600)} h ${Math.floor((secs % 3600) / 60)} min`;
  }
  return plural(Math.floor(secs / 86_400), 'day');
}

export function formatBytes(bytes: number): string {
  const mb = bytes / 1024 ** 2;
  if (mb < 1024) {
    return `${Math.round(mb)} MB`;
  }
  return `${(mb / 1024).toFixed(1)} GB`;
}
