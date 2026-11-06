// What the activity log page shares: wire shapes, the URL's filters and wording.
// Safe to import from client components.

import { one, pagingParams, parseCursor, parseTrail, type RawParams } from './admin-paging';

export type ActivityItem = {
  id: string;
  at: string;
  /** `null` once the account is deleted. */
  admin: { user_id: string; email: string; name: string | null } | null;
  action: string;
  target_kind: string | null;
  target_id: string | null;
  target_label: string | null;
  detail: unknown;
  ip: string | null;
};

/** `GET /v1/admin/activity`. */
export type ActivityPage = { items: ActivityItem[]; next_cursor: string | null; total: number };

export const ACTION_GROUPS = [
  { value: 'all', label: 'All actions' },
  { value: 'override', label: 'Plan overrides' },
  { value: 'suspension', label: 'Suspensions' },
  { value: 'email', label: 'Emails' },
  { value: 'settings', label: 'Settings' },
  { value: 'admins', label: 'Admins' },
] as const;
export type ActionGroup = (typeof ACTION_GROUPS)[number]['value'];

export const SINCE_OPTIONS = [
  { value: '7d', label: 'Last 7 days' },
  { value: '30d', label: 'Last 30 days' },
  { value: 'all', label: 'All time' },
] as const;
export type Since = (typeof SINCE_OPTIONS)[number]['value'];

export type ActivityFilters = { admin: string | undefined; group: ActionGroup; since: Since };
export type ActivityQuery = ActivityFilters & { cursor: string | undefined; trail: string[] };

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** The page's search params, checked: anything unknown falls back to the defaults. */
export function parseActivityQuery(sp: RawParams): ActivityQuery {
  const admin = one(sp.admin);
  const group = one(sp.group);
  const since = one(sp.since);
  return {
    admin: admin && UUID.test(admin) ? admin : undefined,
    group: ACTION_GROUPS.some((g) => g.value === group) ? (group as ActionGroup) : 'all',
    since: SINCE_OPTIONS.some((s) => s.value === since) ? (since as Since) : '7d',
    cursor: parseCursor(sp.cursor),
    trail: parseTrail(sp.trail),
  };
}

function filterParams(f: ActivityFilters): URLSearchParams {
  const p = new URLSearchParams();
  if (f.admin) {
    p.set('admin', f.admin);
  }
  if (f.group !== 'all') {
    p.set('group', f.group);
  }
  if (f.since !== '7d') {
    p.set('since', f.since);
  }
  return p;
}

export function activityHref(query: ActivityFilters & { cursor?: string; trail: string[] }) {
  const p = filterParams(query);
  pagingParams(p, query);
  const qs = p.toString();
  return qs ? `/admin/activity?${qs}` : '/admin/activity';
}

export function activityExportHref(filters: ActivityFilters) {
  const qs = filterParams(filters).toString();
  return qs ? `/admin/activity/export?${qs}` : '/admin/activity/export';
}

const DAY_MS = 86_400_000;

/** The `since` the API wants for a period: an RFC 3339 time, or nothing for all time. */
export function sinceIso(since: Since, now: Date = new Date()): string | undefined {
  if (since === 'all') {
    return undefined;
  }
  return new Date(now.getTime() - (since === '7d' ? 7 : 30) * DAY_MS).toISOString();
}

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** "Nov 5 14:08", in UTC, as the board's Time column. */
export function utcStamp(iso: string): string {
  const d = new Date(iso);
  const hh = String(d.getUTCHours()).padStart(2, '0');
  const mm = String(d.getUTCMinutes()).padStart(2, '0');
  return `${MONTHS[d.getUTCMonth()]} ${d.getUTCDate()} ${hh}:${mm}`;
}

// ---- The detail, as text ----

function label(key: string): string {
  const words = key.replace(/[._]+/g, ' ').trim();
  return words.charAt(0).toUpperCase() + words.slice(1);
}

function scalar(value: unknown): string {
  if (value === null || value === undefined) {
    return 'None';
  }
  if (typeof value === 'boolean') {
    return value ? 'Yes' : 'No';
  }
  if (Array.isArray(value)) {
    return value.length === 0 ? 'None' : value.map(scalar).join(', ');
  }
  if (typeof value === 'object') {
    return JSON.stringify(value);
  }
  return String(value);
}

const isObject = (v: unknown): v is Record<string, unknown> =>
  typeof v === 'object' && v !== null && !Array.isArray(v);

/**
 * An event's `detail` as label and value pairs. A `from` and `to` pair reads as
 * one change per field ("closed → domains"); other objects flatten with their
 * key in front. Everything is text: nothing here is ever markup.
 */
export function detailPairs(detail: unknown, prefix = ''): [string, string][] {
  if (detail === null || detail === undefined) {
    return [];
  }
  if (!isObject(detail)) {
    return [['Detail', scalar(detail)]];
  }
  const out: [string, string][] = [];
  const { from, to } = detail;
  const change = isObject(from) && isObject(to);
  for (const [key, value] of Object.entries(detail)) {
    if (change && (key === 'from' || key === 'to')) {
      continue;
    }
    const name = prefix ? `${prefix} ${key}` : key;
    if (isObject(value)) {
      out.push(...detailPairs(value, name));
    } else {
      out.push([label(name), scalar(value)]);
    }
  }
  if (change) {
    for (const key of new Set([...Object.keys(to), ...Object.keys(from)])) {
      out.push([
        label(prefix ? `${prefix} ${key}` : key),
        `${scalar(from[key])} → ${scalar(to[key])}`,
      ]);
    }
  }
  return out;
}
