// What the admin users pages share: wire shapes, the list's URL state and wording.
// Safe to import from client components.

import { describeEvent } from './admin';

export type UserRole = 'owner' | 'member';
export type UserTeamRef = { slug: string; name: string; role: UserRole };

/** One row of `GET /v1/admin/users`. */
export type UserRow = {
  id: string;
  email: string;
  name: string | null;
  /** `email` first, then each linked provider. */
  methods: string[];
  teams: UserTeamRef[];
  shared_in: number;
  last_active: string | null;
  status: 'active' | 'suspended';
  created_at: string;
};

export type UsersPage = { items: UserRow[]; next_cursor: string | null; total: number };

export type UserEvent = {
  id: string;
  at: string;
  admin: string;
  action: string;
  detail: unknown;
  ip: string | null;
};

/** `GET /v1/admin/users/{id}`. */
export type UserDetail = {
  user: {
    id: string;
    email: string;
    name: string | null;
    status: 'active' | 'suspended';
    created_at: string;
    last_active: string | null;
    product_emails: boolean;
    suspension: { at: string; reason: string | null; by: string | null } | null;
  };
  teams: {
    slug: string;
    name: string;
    role: UserRole;
    joined_at: string;
    plan: 'free' | 'team';
    plan_source: 'override' | 'stripe' | 'none';
  }[];
  identities: {
    provider: string;
    subject: string | null;
    email: string | null;
    created_at: string;
  }[];
  sessions: {
    id: string;
    kind: string;
    label: string | null;
    created_at: string;
    last_used_at: string;
    expires_at: string;
  }[];
  shared: {
    team: string;
    team_slug: string;
    collection: string;
    name: string;
    version: number;
    updated_at: string;
    access: 'off' | 'public' | 'restricted';
  }[];
  events: UserEvent[];
};

// ---- The list's URL ----

export const USER_FILTERS = [
  { value: 'all', label: 'All' },
  { value: 'members', label: 'Team members' },
  { value: 'customers', label: 'Customers only' },
  { value: 'admins', label: 'Admins' },
  { value: 'suspended', label: 'Suspended' },
] as const;

export type UserFilter = (typeof USER_FILTERS)[number]['value'];

export function isUserFilter(value: unknown): value is UserFilter {
  return USER_FILTERS.some((f) => f.value === value);
}

/** The server's longest `q`. */
export const MAX_QUERY = 200;
/** The first page in a trail: it has no cursor. */
export const FIRST_PAGE = '-';
const CURSOR = /^[A-Za-z0-9_-]{1,200}$/;

export type UsersQuery = {
  q: string;
  filter: UserFilter;
  cursor: string | undefined;
  /** The cursors of the pages before this one, oldest first, so Previous can go back. */
  trail: string[];
};

type Raw = Record<string, string | string[] | undefined>;

const one = (v: string | string[] | undefined) => (typeof v === 'string' ? v : undefined);

/** The list's search params, checked: anything unknown falls back to the first page of everyone. */
export function parseUsersQuery(sp: Raw): UsersQuery {
  const filter = one(sp.filter);
  const cursor = one(sp.cursor);
  return {
    q: [...(one(sp.q) ?? '').trim()].slice(0, MAX_QUERY).join(''),
    filter: isUserFilter(filter) ? filter : 'all',
    cursor: cursor && CURSOR.test(cursor) ? cursor : undefined,
    trail: (one(sp.trail) ?? '').split('.').filter((c) => CURSOR.test(c)),
  };
}

export function usersHref(query: Pick<UsersQuery, 'q' | 'filter' | 'trail'> & { cursor?: string }) {
  const p = new URLSearchParams();
  if (query.q) {
    p.set('q', query.q);
  }
  if (query.filter !== 'all') {
    p.set('filter', query.filter);
  }
  if (query.cursor) {
    p.set('cursor', query.cursor);
  }
  if (query.trail.length > 0) {
    p.set('trail', query.trail.join('.'));
  }
  const qs = p.toString();
  return qs ? `/admin/users?${qs}` : '/admin/users';
}

export function usersExportHref(query: Pick<UsersQuery, 'q' | 'filter'>) {
  const p = new URLSearchParams();
  if (query.q) {
    p.set('q', query.q);
  }
  if (query.filter !== 'all') {
    p.set('filter', query.filter);
  }
  const qs = p.toString();
  return qs ? `/admin/users/export?${qs}` : '/admin/users/export';
}

// ---- Wording ----

const PROVIDERS: Record<string, string> = { email: 'Email', github: 'GitHub', google: 'Google' };

export function providerName(provider: string): string {
  return PROVIDERS[provider] ?? provider.charAt(0).toUpperCase() + provider.slice(1);
}

/** "Email · GitHub". */
export function methodsLabel(methods: string[]): string {
  return methods.map(providerName).join(' · ');
}

/** "Acme (owner), Globex Partners", or "None". */
export function teamsLabel(teams: UserTeamRef[]): string {
  if (teams.length === 0) {
    return 'None';
  }
  return teams.map((t) => (t.role === 'owner' ? `${t.name} (owner)` : t.name)).join(', ');
}

export function displayName(user: { name: string | null; email: string }): string {
  return user.name || user.email;
}

/**
 * The first word of a name, else the part of the email before the `@`, as the
 * board's buttons say it: "Email Ada".
 */
export function firstName(user: { name: string | null; email: string }): string {
  return user.name?.trim().split(/\s+/)[0] || user.email.split('@')[0] || user.email;
}

function str(detail: unknown, key: string): string | null {
  if (detail && typeof detail === 'object' && key in detail) {
    const v = (detail as Record<string, unknown>)[key];
    return typeof v === 'string' && v ? v : null;
  }
  return null;
}

/** One admin event as a sentence about `who`. API text stays text. */
export function eventLine(e: UserEvent, who: string): string {
  const verb = describeEvent(e.action);
  if (e.action.startsWith('users.bulk.')) {
    return `${e.admin} ${verb} including ${who}`;
  }
  let tail = '';
  const reason = str(e.detail, 'reason');
  const subject = str(e.detail, 'subject');
  const provider = str(e.detail, 'provider');
  const label = str(e.detail, 'label');
  if (e.action === 'user.suspend' && reason) {
    tail = `: “${reason}”`;
  } else if (e.action === 'user.email' && subject) {
    tail = `: “${subject}”`;
  } else if (e.action === 'user.identity.unlink' && provider) {
    tail = `: ${providerName(provider)}`;
  } else if (e.action === 'user.session.revoke' && label) {
    tail = `: ${label}`;
  }
  return `${e.admin} ${verb} ${who}${tail}`;
}
