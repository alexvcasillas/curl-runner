// What the admin teams pages share: wire shapes, the list's URL state and wording.
// Safe to import from client components.

import { describeEvent } from './admin';
import { formatDate, shortDate } from './time';

export type TeamPlan = 'free' | 'team';
export type PlanSource = 'override' | 'stripe' | 'none';
export type TeamRole = 'owner' | 'member';

/** One row of `GET /v1/admin/teams`. */
export type TeamRow = {
  slug: string;
  name: string;
  created_at: string;
  /** The longest-standing owner. */
  owner: { email: string; name: string | null } | null;
  members: number;
  collections: number;
  plan: TeamPlan;
  plan_source: PlanSource;
  billing_status: string | null;
  billing_interval: string | null;
  past_due_since: string | null;
  cancel_at: string | null;
  override_until: string | null;
  suspended: boolean;
  /** `null` when the price can't be read. */
  mrr_cents: number | null;
};

export type TeamsPage = { items: TeamRow[]; next_cursor: string | null; total: number };

export type StripeBlock = {
  status: string | null;
  interval: string | null;
  seats: number;
  amount_cents: number | null;
  currency: string | null;
  renews: string | null;
  cancel_at: string | null;
  past_due_since: string | null;
  subscription_id: string;
  customer_id: string | null;
};

export type OverrideBody = {
  plan: TeamPlan;
  until: string | null;
  reason: string | null;
  by: string | null;
  at: string | null;
  reminded_at: string | null;
  /** False once `until` has passed and before the worker clears it. */
  active: boolean;
};

export type Suspension = { at: string; reason: string | null; by: string | null };

export type HistoryRow = {
  at: string;
  source: string;
  plan: string;
  admin: string | null;
  reason: string | null;
  detail: unknown;
};

/** `GET /v1/admin/teams/{slug}`. */
export type TeamDetail = {
  team: {
    id: string;
    slug: string;
    name: string;
    created_at: string;
    plan: TeamPlan;
    plan_source: PlanSource;
    limits: { members: number | null; restricted: boolean };
  };
  stripe: StripeBlock | null;
  override: OverrideBody | null;
  plan_history: HistoryRow[];
  members: {
    user_id: string;
    email: string;
    name: string | null;
    role: TeamRole;
    joined_at: string;
  }[];
  pending: { id: string; email: string; role: TeamRole; expires_at: string }[];
  collections: {
    slug: string;
    name: string;
    source: string;
    access: 'off' | 'public' | 'restricted';
    current_version: number | null;
    updated_at: string | null;
    link_blocked: boolean;
  }[];
  usage: { tokens_used_this_month: number; publishes_this_month: number };
  suspension: Suspension | null;
};

/** The body of `PUT …/override`: `until` is an ISO instant, or null for no end date. */
export type OverrideInput = {
  plan: TeamPlan;
  until: string | null;
  reason: string;
  notify: boolean;
};

// ---- The list's URL ----

export const TEAM_FILTERS = [
  { value: 'all', label: 'All' },
  { value: 'stripe', label: 'Paying' },
  { value: 'granted', label: 'Granted' },
  { value: 'past_due', label: 'Past due' },
  { value: 'free', label: 'Free' },
  { value: 'suspended', label: 'Suspended' },
] as const;

export type TeamFilter = (typeof TEAM_FILTERS)[number]['value'];

export function isTeamFilter(value: unknown): value is TeamFilter {
  return TEAM_FILTERS.some((f) => f.value === value);
}

/** The server's longest `q`. */
export const MAX_QUERY = 200;
/** The first page in a trail: it has no cursor. */
export const FIRST_PAGE = '-';
const CURSOR = /^[A-Za-z0-9_-]{1,200}$/;

export type TeamsQuery = {
  q: string;
  filter: TeamFilter;
  cursor: string | undefined;
  /** The cursors of the pages before this one, oldest first, so Previous can go back. */
  trail: string[];
};

type Raw = Record<string, string | string[] | undefined>;

const one = (v: string | string[] | undefined) => (typeof v === 'string' ? v : undefined);

/** The list's search params, checked: anything unknown falls back to the first page of every team. */
export function parseTeamsQuery(sp: Raw): TeamsQuery {
  const filter = one(sp.filter);
  const cursor = one(sp.cursor);
  return {
    q: [...(one(sp.q) ?? '').trim()].slice(0, MAX_QUERY).join(''),
    filter: isTeamFilter(filter) ? filter : 'all',
    cursor: cursor && CURSOR.test(cursor) ? cursor : undefined,
    trail: (one(sp.trail) ?? '').split('.').filter((c) => CURSOR.test(c)),
  };
}

function params(query: Pick<TeamsQuery, 'q' | 'filter'>) {
  const p = new URLSearchParams();
  if (query.q) {
    p.set('q', query.q);
  }
  if (query.filter !== 'all') {
    p.set('filter', query.filter);
  }
  return p;
}

export function teamsHref(query: Pick<TeamsQuery, 'q' | 'filter' | 'trail'> & { cursor?: string }) {
  const p = params(query);
  if (query.cursor) {
    p.set('cursor', query.cursor);
  }
  if (query.trail.length > 0) {
    p.set('trail', query.trail.join('.'));
  }
  const qs = p.toString();
  return qs ? `/admin/teams?${qs}` : '/admin/teams';
}

export function teamsExportHref(query: Pick<TeamsQuery, 'q' | 'filter'>) {
  const qs = params(query).toString();
  return qs ? `/admin/teams/export?${qs}` : '/admin/teams/export';
}

// ---- Wording ----

const DAY = 86_400_000;
/** The grace period the Billing page also tells owners about. */
const GRACE_DAYS = 14;

export type SourceTone = 'ok' | 'warn' | 'err' | 'info' | 'neutral';
export type Source = { label: string; tone: SourceTone };

const interval = (i: string | null) => (i === 'monthly' ? 'monthly' : 'annual');

function daysLeft(pastDueSince: string, now: Date): number {
  const end = new Date(pastDueSince).getTime() + GRACE_DAYS * DAY;
  return Math.max(0, Math.ceil((end - now.getTime()) / DAY));
}

/** Where a team's plan comes from, as the list says it: "Stripe · annual", "Granted until Mar 31, 2027". */
export function sourceOf(
  t: Pick<
    TeamRow,
    | 'suspended'
    | 'plan_source'
    | 'override_until'
    | 'past_due_since'
    | 'cancel_at'
    | 'billing_interval'
  >,
  now: Date,
): Source {
  if (t.suspended) {
    return { label: 'Suspended', tone: 'err' };
  }
  if (t.plan_source === 'override') {
    return {
      label: t.override_until ? `Granted until ${formatDate(t.override_until)}` : 'Granted, no end',
      tone: 'info',
    };
  }
  if (t.plan_source === 'stripe') {
    if (t.past_due_since) {
      const left = daysLeft(t.past_due_since, now);
      return {
        label: `Stripe · past due, ${left} ${left === 1 ? 'day' : 'days'} left`,
        tone: 'err',
      };
    }
    if (t.cancel_at) {
      return { label: `Stripe · cancels ${shortDate(t.cancel_at)}`, tone: 'warn' };
    }
    return { label: `Stripe · ${interval(t.billing_interval)}`, tone: 'ok' };
  }
  return { label: 'None', tone: 'neutral' };
}

/** What the detail header says about the source, apart from a suspension: the banner says that. */
export function headerSource(d: TeamDetail, now: Date): Source {
  const { team, stripe, override } = d;
  if (team.plan_source === 'override' && override) {
    return {
      label: override.until ? `Granted until ${formatDate(override.until)}` : 'Granted, no end',
      tone: 'info',
    };
  }
  if (team.plan_source === 'stripe' && stripe) {
    return sourceOf(
      {
        suspended: false,
        plan_source: 'stripe',
        override_until: null,
        past_due_since: stripe.past_due_since,
        cancel_at: stripe.cancel_at,
        billing_interval: stripe.interval,
      },
      now,
    );
  }
  return { label: 'No subscription', tone: 'neutral' };
}

/** "3 members, no restricted collections", or "none" for a plan without limits. */
export function limitsLabel(limits: TeamDetail['team']['limits']): string {
  const parts: string[] = [];
  if (limits.members !== null) {
    parts.push(`${limits.members} members`);
  }
  if (!limits.restricted) {
    parts.push('no restricted collections');
  }
  return parts.length > 0 ? parts.join(', ') : 'none';
}

export const planName = (plan: string) => (plan === 'team' ? 'Team' : 'Free');

/** `https://dashboard.stripe.com/subscriptions/sub_…`, or null for an id that isn't Stripe's shape. */
const STRIPE_ID = /^(sub|cus)_[A-Za-z0-9]{1,64}$/;
export function stripeUrl(kind: 'subscriptions' | 'customers', id: string): string | null {
  return STRIPE_ID.test(id) ? `https://dashboard.stripe.com/${kind}/${id}` : null;
}

function str(detail: unknown, key: string): string | null {
  if (detail && typeof detail === 'object' && key in detail) {
    const v = (detail as Record<string, unknown>)[key];
    return typeof v === 'string' && v ? v : null;
  }
  return null;
}

/** One plan-history row as a sentence. API text (reasons, emails) stays text. */
export function historyLine(h: HistoryRow): string {
  const plan = planName(h.plan);
  const who = h.admin ?? 'An admin';
  const until = str(h.detail, 'until');
  switch (h.source) {
    case 'default':
      return `Created on ${plan}`;
    case 'admin':
      return `${who} set the plan to ${plan}`;
    case 'stripe':
      return `Stripe set the plan to ${plan}`;
    case 'past_due':
      return `Payment failed: back to ${plan}`;
    case 'override': {
      const verb = str(h.detail, 'action') === 'team.override.update' ? 'changed' : 'granted';
      return `${who} ${verb} ${plan} ${until ? `until ${formatDate(until)}` : 'with no end date'}`;
    }
    case 'override_removed':
      return `${who} removed the override: back to ${plan}`;
    case 'expired':
      return `The override ended: back to ${plan}`;
    default:
      return `${describeEvent(h.source)} ${plan}`;
  }
}

// ---- The grant dialog's date ----

/** A `YYYY-MM-DD` day as the end of that day, UTC, or null for anything else. */
export function endOfDay(day: string): string | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(day);
  if (!m) {
    return null;
  }
  const d = new Date(`${day}T23:59:59.000Z`);
  if (Number.isNaN(d.getTime()) || d.getUTCDate() !== Number(m[3])) {
    return null;
  }
  return d.toISOString();
}

/** The `YYYY-MM-DD` of an ISO instant, UTC, for the date input. */
export function dayOf(iso: string): string {
  return new Date(iso).toISOString().slice(0, 10);
}
