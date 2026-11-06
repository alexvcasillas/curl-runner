// What the admin emails pages share: wire shapes, audiences, the log's URL state and wording.
// Safe to import from client components.

import type { ActionResult } from './action-result';
import { FIRST_PAGE, MAX_QUERY } from './admin-teams';
import { dateThisYear, relativeTime } from './time';

export const MAX_SUBJECT = 200;
export const MAX_BODY = 20_000;
/** The mail worker's give-up point, and the wait that doubles from 30 seconds to an hour. */
export const MAX_ATTEMPTS = 8;

// ---- Broadcasts ----

/** The only audiences the server knows. */
export const AUDIENCES = [
  {
    value: 'everyone',
    label: 'Everyone',
    noun: 'everyone',
    hint: 'everyone with an account',
  },
  {
    value: 'team_owners',
    label: 'Team owners',
    noun: 'team owners',
    hint: 'owners of every team',
  },
  {
    value: 'paying_teams',
    label: 'Paying teams',
    noun: 'owners of paying teams',
    hint: 'owners of teams that pay through Stripe',
  },
  {
    value: 'free_teams',
    label: 'Free teams',
    noun: 'owners of free teams',
    hint: 'owners of teams on the Free plan',
  },
  {
    value: 'granted_teams',
    label: 'Granted teams',
    noun: 'owners of granted teams',
    hint: 'owners of teams with a plan granted by an admin',
  },
] as const;

export type Audience = (typeof AUDIENCES)[number]['value'];

export function isAudience(value: unknown): value is Audience {
  return AUDIENCES.some((a) => a.value === value);
}

export function audienceOf(value: string) {
  return AUDIENCES.find((a) => a.value === value) ?? AUDIENCES[0];
}

export type BroadcastStatus = 'draft' | 'scheduled' | 'sending' | 'sent';

/** `GET /v1/admin/broadcasts[/{id}]`. */
export type Broadcast = {
  id: string;
  subject: string;
  body_md: string;
  audience: Audience;
  status: BroadcastStatus;
  scheduled_at: string | null;
  sent_at: string | null;
  /** Outbox rows queued for it so far. */
  sent_count: number;
  created_by: { user_id: string; email: string } | null;
  created_at: string;
  updated_at: string;
  delivered: number;
  failed: number;
};

export type DraftInput = { subject: string; body_md: string; audience: Audience };

/** `GET …/{id}/preview?as_user=`. */
export type Preview = {
  subject: string;
  html: string;
  text: string;
  as_user: { id: string; email: string; name: string | null };
  in_audience: boolean;
  opted_out: boolean;
};

export type AudienceCount = { audience: Audience; count: number };

/** What a broadcast action gives back to the editor. */
export type BroadcastResult =
  | { ok: true; broadcast: Broadcast }
  | Extract<ActionResult, { ok: false }>;

export function plural(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** "1,148 delivered · 3 failed", the part of a list row after the date. */
export function broadcastMeta(b: Broadcast, now: Date): string {
  const aud = audienceOf(b.audience).label;
  switch (b.status) {
    case 'draft':
      return `Draft · ${aud}`;
    case 'scheduled':
      return b.scheduled_at
        ? `Scheduled ${scheduledLabel(b.scheduled_at)} · ${aud}`
        : `Scheduled · ${aud}`;
    case 'sending':
      return `Sending · ${plural(b.sent_count, 'email', 'emails')} queued`;
    case 'sent': {
      const when = b.sent_at ? dateThisYear(b.sent_at, now) : '';
      const parts = [`Sent ${when}`.trim(), `${b.delivered.toLocaleString('en-US')} delivered`];
      if (b.failed > 0) {
        parts.push(`${b.failed.toLocaleString('en-US')} failed`);
      }
      return parts.join(' · ');
    }
  }
}

/** "Nov 9, 10:00 UTC". */
export function scheduledLabel(iso: string): string {
  const d = new Date(iso);
  const month = d.toLocaleString('en-US', { month: 'short', timeZone: 'UTC' });
  const hh = String(d.getUTCHours()).padStart(2, '0');
  const mm = String(d.getUTCMinutes()).padStart(2, '0');
  return `${month} ${d.getUTCDate()}, ${hh}:${mm} UTC`;
}

/** "Send to 214 people". */
export function sendLabel(count: number | null): string {
  return count === null ? 'Send' : `Send to ${peopleLabel(count)}`;
}

/** "214 people", "1 person". */
export function peopleLabel(count: number): string {
  return count === 1 ? '1 person' : `${count.toLocaleString('en-US')} people`;
}

/**
 * A schedule time from a `datetime-local` value (the browser's own zone), as the
 * UTC instant the server stores. `null` unless it is a real time after `now`.
 */
export function scheduleInstant(
  local: string,
  now: Date,
): { ok: true; iso: string } | { ok: false; message: string } {
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/.test(local)) {
    return { ok: false, message: 'Pick a date and a time.' };
  }
  const at = new Date(local);
  if (Number.isNaN(at.getTime())) {
    return { ok: false, message: 'Pick a date and a time.' };
  }
  if (at.getTime() <= now.getTime()) {
    return { ok: false, message: 'Pick a time in the future.' };
  }
  return { ok: true, iso: at.toISOString() };
}

// ---- The delivery log ----

export type LogStatus = 'queued' | 'sent' | 'failed';

/** One row of `GET /v1/admin/emails/log`. */
export type LogRow = {
  id: string;
  to: string;
  subject: string;
  kind: string;
  status: LogStatus;
  broadcast_id: string | null;
  created_at: string | null;
  sent_at: string | null;
  failed_at: string | null;
  next_attempt_at: string | null;
  attempts: number;
  last_error: string | null;
};

export type LogPage = { items: LogRow[]; next_cursor: string | null };

export const KIND_LABELS: Record<string, string> = {
  sign_in_code: 'Sign-in code',
  member_invite: 'Member invite',
  customer_invite: 'Customer invite',
  payment_failed: 'Payment failed',
  plan_changed: 'Plan changed',
  test_email: 'Test email',
  account_suspended: 'Account suspended',
  plan_granted: 'Plan granted',
  override_ending: 'Plan ending',
  team_suspended: 'Team suspended',
  link_disabled: 'Link disabled',
  admin: 'Admin message',
  broadcast: 'Broadcast',
  broadcast_test: 'Broadcast test',
  transactional: 'Other',
};

export const KINDS = Object.keys(KIND_LABELS);

export function kindLabel(kind: string): string {
  return KIND_LABELS[kind] ?? kind;
}

export const LOG_STATUSES = [
  { value: 'all', label: 'All' },
  { value: 'queued', label: 'Waiting' },
  { value: 'sent', label: 'Sent' },
  { value: 'failed', label: 'Failed' },
] as const;

export type LogStatusFilter = (typeof LOG_STATUSES)[number]['value'];

const CURSOR = /^[A-Za-z0-9_-]{1,200}$/;

export type LogQuery = {
  q: string;
  status: LogStatusFilter;
  /** A key of `KIND_LABELS`, or `all`. */
  kind: string;
  cursor: string | undefined;
  trail: string[];
};

type Raw = Record<string, string | string[] | undefined>;
const one = (v: string | string[] | undefined) => (typeof v === 'string' ? v : undefined);

export function parseLogQuery(sp: Raw): LogQuery {
  const status = one(sp.status);
  const kind = one(sp.kind);
  const cursor = one(sp.cursor);
  return {
    q: [...(one(sp.q) ?? '').trim()].slice(0, MAX_QUERY).join(''),
    status: LOG_STATUSES.some((s) => s.value === status) ? (status as LogStatusFilter) : 'all',
    kind: kind && kind in KIND_LABELS ? kind : 'all',
    cursor: cursor && CURSOR.test(cursor) ? cursor : undefined,
    trail: (one(sp.trail) ?? '').split('.').filter((c) => CURSOR.test(c)),
  };
}

export type EmailsTab = 'broadcasts' | 'log';

/** The emails page's URL: the log tab, its filters and its cursor trail, or a broadcast. */
export function logHref(
  query: Pick<LogQuery, 'q' | 'status' | 'kind' | 'trail'> & { cursor?: string },
) {
  const p = new URLSearchParams({ tab: 'log' });
  if (query.q) {
    p.set('q', query.q);
  }
  if (query.status !== 'all') {
    p.set('status', query.status);
  }
  if (query.kind !== 'all') {
    p.set('kind', query.kind);
  }
  if (query.cursor) {
    p.set('cursor', query.cursor);
  }
  if (query.trail.length > 0) {
    p.set('trail', query.trail.join('.'));
  }
  return `/admin/emails?${p}`;
}

export function broadcastHref(id?: string): string {
  return id ? `/admin/emails?b=${encodeURIComponent(id)}` : '/admin/emails';
}

export function previousLogCursor(trail: string[]): string | undefined {
  const last = trail[trail.length - 1];
  return last === FIRST_PAGE ? undefined : last;
}

/** The Detail column: what the server said, or where the retries stand. */
export function logDetail(row: LogRow, now: Date): string {
  if (row.status === 'sent') {
    return row.attempts > 1 ? `Sent on attempt ${row.attempts}` : 'Sent';
  }
  if (row.status === 'failed') {
    return row.last_error || `Gave up after ${MAX_ATTEMPTS} attempts`;
  }
  if (row.attempts === 0) {
    return 'Waiting to send';
  }
  const next = row.next_attempt_at ? ` · next try ${relativeNext(row.next_attempt_at, now)}` : '';
  return `${row.last_error ? `${row.last_error} · ` : ''}Attempt ${row.attempts} of ${MAX_ATTEMPTS} failed${next}`;
}

function relativeNext(iso: string, now: Date): string {
  const secs = Math.round((new Date(iso).getTime() - now.getTime()) / 1000);
  if (secs <= 0) {
    return 'soon';
  }
  if (secs < 90) {
    return `in ${secs} s`;
  }
  const mins = Math.round(secs / 60);
  return mins < 90 ? `in ${mins} min` : relativeTime(iso, now);
}

/** "13:04:12" for today, else "Nov 5, 13:04": UTC, like every admin time. */
export function logWhen(iso: string | null, now: Date): string {
  if (!iso) {
    return '–';
  }
  const d = new Date(iso);
  const hh = String(d.getUTCHours()).padStart(2, '0');
  const mm = String(d.getUTCMinutes()).padStart(2, '0');
  const ss = String(d.getUTCSeconds()).padStart(2, '0');
  const sameDay = d.toISOString().slice(0, 10) === now.toISOString().slice(0, 10);
  return sameDay ? `${hh}:${mm}:${ss}` : `${dateThisYear(iso, now)}, ${hh}:${mm}`;
}
