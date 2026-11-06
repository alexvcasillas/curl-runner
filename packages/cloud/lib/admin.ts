// What the admin panel shares: permissions, the nav, wire shapes and wording.
// Safe to import from client components.

export type Perm = 'users' | 'teams' | 'billing' | 'emails' | 'settings' | 'admins';
export type AdminRole = 'owner' | 'admin';

/** `GET /v1/admin/me`. */
export type AdminMe = { role: AdminRole; permissions: Perm[] };

/**
 * The one place that decides whether a write control is live. Every admin can
 * read everything, so a missing permission never hides a page, only its writes.
 */
export function can(permissions: readonly Perm[], perm: Perm): boolean {
  return permissions.includes(perm);
}

export const adminRoutes = {
  overview: '/admin',
  users: '/admin/users',
  user: (id: string) => `/admin/users/${encodeURIComponent(id)}`,
  teams: '/admin/teams',
  team: (slug: string) => `/admin/teams/${encodeURIComponent(slug)}`,
  collections: '/admin/collections',
  collectionsSearch: (q: string) => `/admin/collections?q=${encodeURIComponent(q)}`,
  emails: '/admin/emails',
  stripe: '/admin/stripe',
  settings: '/admin/settings',
  health: '/admin/health',
  admins: '/admin/admins',
  activity: '/admin/activity',
} as const;

export type NavItem = {
  label: string;
  href: string;
  /** A 16px stroke path, from the board. */
  d: string;
  /** Overview only matches its own path. */
  exact?: boolean;
};

export const NAV_GROUPS: { label: string; items: NavItem[] }[] = [
  {
    label: 'General',
    items: [
      {
        label: 'Overview',
        href: adminRoutes.overview,
        exact: true,
        d: 'M2.5 2.5h4.5v4.5H2.5zM9 2.5h4.5v4.5H9zM2.5 9h4.5v4.5H2.5zM9 9h4.5v4.5H9z',
      },
    ],
  },
  {
    label: 'Customers',
    items: [
      {
        label: 'Users',
        href: adminRoutes.users,
        d: 'M6 7.5a2.5 2.5 0 100-5 2.5 2.5 0 000 5zM1.5 13.5a4.5 4.5 0 019 0M11 3a2.5 2.5 0 010 4.5M12.5 9.5a4 4 0 012 4',
      },
      { label: 'Teams', href: adminRoutes.teams, d: 'M2.5 4.5h11v9h-11zM5.5 4.5v-2h5v2' },
      { label: 'Collections', href: adminRoutes.collections, d: 'M2 4.5h4l1.5 1.5H14v6.5H2z' },
    ],
  },
  {
    label: 'Operations',
    items: [{ label: 'Emails', href: adminRoutes.emails, d: 'M2 3.5h12v9H2zM2 4l6 5 6-5' }],
  },
  {
    label: 'Billing',
    items: [{ label: 'Stripe', href: adminRoutes.stripe, d: 'M2 4.5h12v8H2zM2 7h12M4.5 10.5h2' }],
  },
  {
    label: 'Server',
    items: [
      {
        label: 'Settings',
        href: adminRoutes.settings,
        d: 'M8 5.5a2.5 2.5 0 100 5 2.5 2.5 0 000-5zM8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4',
      },
      { label: 'Health', href: adminRoutes.health, d: 'M1.5 8h3l2-5 3 10 2-5h3' },
    ],
  },
  {
    label: 'Access',
    items: [
      {
        label: 'Admins',
        href: adminRoutes.admins,
        d: 'M8 1.5l5 2v4c0 3.2-2.2 5.6-5 7-2.8-1.4-5-3.8-5-7v-4z',
      },
      { label: 'Activity log', href: adminRoutes.activity, d: 'M3 3.5h10M3 8h10M3 12.5h6' },
    ],
  },
];

const NAV_PERM: Record<string, Perm> = {
  [adminRoutes.users]: 'users',
  [adminRoutes.teams]: 'teams',
  [adminRoutes.emails]: 'emails',
  [adminRoutes.stripe]: 'billing',
  [adminRoutes.settings]: 'settings',
  [adminRoutes.admins]: 'admins',
};

/** The permission that edits what a nav item opens, or null for read-only pages. */
export function navPerm(item: NavItem): Perm | null {
  return NAV_PERM[item.href] ?? null;
}

/** Whether `path` is inside the page a nav item opens. */
export function navActive(item: NavItem, path: string): boolean {
  return item.exact ? path === item.href : path === item.href || path.startsWith(`${item.href}/`);
}

// ---- GET /v1/admin/overview ----

export type Day = { day: string; n: number };

export type AdminEvent = {
  id: string;
  admin_email: string;
  action: string;
  target_kind: string | null;
  target_id: string | null;
  target_label: string | null;
  at: string;
};

export type AdminOverview = {
  billing: boolean;
  counts: {
    users: number;
    teams: number;
    collections: number;
    collections_public: number;
    collections_restricted: number;
    access_list_people: number;
    plan_overrides: number;
    plan_overrides_ending: number;
    granted_teams: number;
    granted_seats: number;
    paying_teams: number | null;
    paying_annual: number | null;
    paying_monthly: number | null;
    mrr_cents: number | null;
  };
  past_due: { teams: number; next: { team: string; name: string; deadline: string } | null };
  signups_30d: Day[];
  teams_30d: Day[];
  publishes_14d: Day[];
  recent_events: AdminEvent[];
  p95_ms: number | null;
  outbox: { waiting: number; failed_today: number };
  last_webhook_at: string | null;
  signup_mode: 'open' | 'domains' | 'closed';
};

// ---- Search ----

export type SearchResults = {
  users: { id: string; email: string; name: string | null }[];
  teams: { slug: string; name: string; plan: string }[];
  collections: { team: string; slug: string; name: string }[];
};

// ---- Wording ----

const ACTIONS: Record<string, string> = {
  'team.override.grant': 'granted a plan to',
  'team.override.update': 'changed the plan of',
  'team.override.remove': 'removed the plan override of',
  'team.plan.set': 'set the plan of',
  'team.suspend': 'suspended',
  'team.unsuspend': 'unsuspended',
  'team.delete': 'deleted',
  'team.member.role': 'changed a member role in',
  'team.email_owners': 'emailed the owners of',
  'team.invoice.resend': 'resent an invoice to',
  'team.stripe_sync': 'synced Stripe for',
  'collection.link.disable': 'disabled the share link of',
  'collection.link.enable': 'enabled the share link of',
  'user.suspend': 'suspended',
  'user.unsuspend': 'unsuspended',
  'user.delete': 'deleted',
  'user.email': 'emailed',
  'user.sign_out_everywhere': 'signed out every device of',
  'user.session.revoke': 'signed out a device of',
  'user.identity.unlink': 'unlinked a sign-in method of',
  'users.bulk.suspend': 'suspended several users:',
  'users.bulk.sign_out': 'signed out several users:',
  'broadcast.create': 'drafted email',
  'broadcast.update': 'edited email',
  'broadcast.schedule': 'scheduled email',
  'broadcast.send': 'sent email',
  'broadcast.test': 'sent a test of email',
  'admin.add': 'added admin',
  'admin.update': 'changed the permissions of',
  'admin.remove': 'removed admin',
  'settings.update': 'changed settings:',
  'settings.test_email': 'sent a test email',
};

/** An audit action as a verb phrase: "granted a plan to". */
export function describeEvent(action: string): string {
  return ACTIONS[action] ?? action.replace(/[._]/g, ' ');
}

const usd = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD',
  maximumFractionDigits: 0,
});

/** Whole dollars from cents. */
export function formatMoney(cents: number): string {
  return usd.format(cents / 100);
}

const int = new Intl.NumberFormat('en-US');
export function formatInt(n: number): string {
  return int.format(n);
}

// ---- Permission labels: the one place they are named ----

export const PERMS: readonly Perm[] = ['users', 'teams', 'billing', 'emails', 'settings', 'admins'];

export function isPerm(value: unknown): value is Perm {
  return PERMS.includes(value as Perm);
}

/**
 * The words for each permission, as the Admins board has them. The board's six
 * rows don't match the server's one for one: "Manage users and teams" and
 * "Moderate collections" are the server's `users` and `teams`, and "Manage
 * plans" is `billing`.
 */
export const PERM_LABELS: Record<Perm, { title: string; short: string; desc: string }> = {
  users: {
    title: 'Manage users',
    short: 'Users',
    desc: 'Sign people out, suspend and delete accounts.',
  },
  teams: {
    title: 'Manage teams and collections',
    short: 'Teams',
    desc: 'Suspend teams, change member roles, disable share links that are abused.',
  },
  billing: {
    title: 'Manage plans',
    short: 'Plans',
    desc: 'Grant and remove plan overrides. Billing itself stays in Stripe.',
  },
  emails: {
    title: 'Send emails',
    short: 'Emails',
    desc: 'Broadcasts to users and team owners.',
  },
  settings: {
    title: 'Change server settings',
    short: 'Settings',
    desc: 'Sign-up, plan limits, the banner, pausing publishing.',
  },
  admins: {
    title: 'Manage admins',
    short: 'Admins',
    desc: 'Add admins and change their permissions.',
  },
};

/** "Users, emails", "Every permission" or "Read only". */
export function permissionsLabel(permissions: readonly Perm[]): string {
  if (PERMS.every((p) => permissions.includes(p))) {
    return 'Every permission';
  }
  const names = PERMS.filter((p) => permissions.includes(p)).map((p) => PERM_LABELS[p].short);
  if (names.length === 0) {
    return 'Read only';
  }
  return names.map((n, i) => (i === 0 ? n : n.toLowerCase())).join(', ');
}
