import type { GalleryCollection, GalleryDetail, GalleryList } from '@/lib/gallery';

const hoursAgo = (h: number) => new Date(Date.now() - h * 3_600_000).toISOString();

export const PAYMENTS: GalleryCollection = {
  team: { slug: 'acme', name: 'Acme' },
  slug: 'payments-api',
  name: 'Payments API',
  description: 'Charges, refunds and payouts for the Acme platform.',
  requests: 18,
  version: 14,
  updated_at: hoursAgo(2),
  listed_at: hoursAgo(3),
  link: 'https://cloud.curl-runner.com/s/crsl_abc',
};

export const USERS: GalleryCollection = {
  ...PAYMENTS,
  slug: 'users-api',
  name: 'Users API',
  description: 'Accounts and sessions.',
  requests: 6,
  version: 2,
};

export const GLOBEX: GalleryCollection = {
  ...PAYMENTS,
  team: { slug: 'globex', name: 'Globex' },
  slug: 'orders',
  name: 'Orders',
  link: 'https://cloud.curl-runner.com/s/crsl_def',
};

export const LIST: GalleryList = {
  collections: [PAYMENTS, USERS, GLOBEX],
  next_cursor: 'cur_2',
  total: 48,
  teams: 21,
};

export const DETAIL: GalleryDetail = {
  team: PAYMENTS.team,
  slug: PAYMENTS.slug,
  name: PAYMENTS.name,
  description: PAYMENTS.description,
  version: 14,
  updated_at: PAYMENTS.updated_at,
  link: PAYMENTS.link,
  inputs: [
    { name: 'BASE_URL', description: 'Where the API lives', secret: false },
    { name: 'API_KEY', description: 'Your secret key', secret: true },
  ],
  files: [
    { path: 'curl-runner.yaml', text: 'global:\n  timeout: 5000\n' },
    {
      path: 'health.yaml',
      text: 'request:\n  name: Health\n  url: https://api.example.com/health\n',
      requests: [{ method: 'GET', name: 'Health' }],
    },
    {
      path: 'charges/create-charge.yaml',
      text: 'request:\n  name: Create charge\n  method: POST\n  url: https://api.example.com/charges\n',
      requests: [{ method: 'POST', name: 'Create charge' }],
    },
    {
      path: 'charges/refunds.yaml',
      text: 'requests:\n  - name: Refund\n  - name: Void\n',
      requests: [
        { method: 'DELETE', name: 'Refund' },
        { method: 'PATCH', name: 'Void' },
      ],
    },
  ],
};

export function json(body: unknown, status = 200): Response {
  return Response.json(body, { status });
}
