import type { UserDetail, UserRow, UsersPage } from '@/lib/admin-users';

export const NOW = '2026-11-06T12:00:00Z';

export const ADA = 'a0000000-0000-4000-8000-000000000001';
export const LINUS = 'a0000000-0000-4000-8000-000000000003';
export const JANE = 'a0000000-0000-4000-8000-000000000004';
export const SESSION_A = 'b0000000-0000-4000-8000-00000000000a';
export const SESSION_B = 'b0000000-0000-4000-8000-00000000000b';

export const rows: UserRow[] = [
  {
    id: ADA,
    email: 'ada@acme.example',
    name: 'Ada Lovelace',
    methods: ['email', 'github'],
    teams: [
      { slug: 'acme', name: 'Acme', role: 'owner' },
      { slug: 'globex', name: 'Globex Partners', role: 'member' },
    ],
    shared_in: 0,
    last_active: '2026-11-06T11:54:00Z',
    status: 'active',
    created_at: '2026-03-02T09:00:00Z',
  },
  {
    id: LINUS,
    email: 'linus@acme.example',
    name: 'Linus Torvalds',
    methods: ['email', 'github'],
    teams: [{ slug: 'acme', name: 'Acme', role: 'member' }],
    shared_in: 0,
    last_active: '2026-11-05T10:00:00Z',
    status: 'active',
    created_at: '2026-03-03T09:00:00Z',
  },
  {
    id: JANE,
    email: 'jane@globex.example',
    name: null,
    methods: ['email'],
    teams: [],
    shared_in: 3,
    last_active: null,
    status: 'suspended',
    created_at: '2026-04-01T09:00:00Z',
  },
];

export const page: UsersPage = { items: rows, next_cursor: 'CURSOR_2', total: 1248 };

export const detail: UserDetail = {
  user: {
    id: ADA,
    email: 'ada@acme.example',
    name: 'Ada Lovelace',
    status: 'active',
    created_at: '2026-03-02T09:00:00Z',
    last_active: '2026-11-06T11:54:00Z',
    product_emails: true,
    suspension: null,
  },
  teams: [
    {
      slug: 'acme',
      name: 'Acme',
      role: 'owner',
      joined_at: '2026-03-02T09:00:00Z',
      plan: 'team',
      plan_source: 'stripe',
    },
    {
      slug: 'globex',
      name: 'Globex Partners',
      role: 'member',
      joined_at: '2026-07-09T09:00:00Z',
      plan: 'free',
      plan_source: 'none',
    },
  ],
  identities: [
    {
      provider: 'email',
      subject: null,
      email: 'ada@acme.example',
      created_at: '2026-03-02T09:00:00Z',
    },
    {
      provider: 'github',
      subject: '4021733',
      email: 'ada@acme.example',
      created_at: '2026-03-05T09:00:00Z',
    },
  ],
  sessions: [
    {
      id: SESSION_A,
      kind: 'web',
      label: 'Chrome on macOS',
      created_at: '2026-11-01T09:00:00Z',
      last_used_at: '2026-11-06T11:54:00Z',
      expires_at: '2027-02-01T09:00:00Z',
    },
    {
      id: SESSION_B,
      kind: 'device',
      label: null,
      created_at: '2026-11-01T09:00:00Z',
      last_used_at: '2026-11-05T09:00:00Z',
      expires_at: '2027-02-01T09:00:00Z',
    },
  ],
  shared: [
    {
      team: 'Initech',
      team_slug: 'initech',
      collection: 'billing-sandbox',
      name: 'Billing sandbox',
      version: 3,
      updated_at: '2026-10-30T09:00:00Z',
      access: 'public',
    },
  ],
  events: [
    {
      id: 'e1',
      at: '2026-11-05T13:02:00Z',
      admin: 'alex@example.com',
      action: 'user.suspend',
      detail: { reason: 'Chargeback dispute', notify: false },
      ip: '83.44.12.9',
    },
    {
      id: 'e2',
      at: '2026-11-04T09:03:00Z',
      admin: 'alex@example.com',
      action: 'users.bulk.sign_out',
      detail: { ids: [ADA] },
      ip: null,
    },
  ],
};

export const suspended: UserDetail = {
  ...detail,
  user: {
    ...detail.user,
    status: 'suspended',
    suspension: {
      at: '2026-11-05T13:02:00Z',
      reason: 'Chargeback dispute, waiting on bank',
      by: 'alex@example.com',
    },
  },
  sessions: [],
};
