import type { CollectionRow, CollectionStats, CollectionsPage } from '@/lib/admin-collections';

export const NOW = '2026-11-06T12:00:00Z';

const base: CollectionRow = {
  team: 'acme',
  slug: 'payments-api',
  name: 'Payments API',
  source: 'git',
  access: 'public',
  versions: 14,
  size_bytes: 86_016,
  opens_30d: 612,
  last_publish: '2026-11-06T10:00:00Z',
  link_disabled: false,
};

export const rows: CollectionRow[] = [
  base,
  { ...base, slug: 'webhooks', name: 'Webhooks', access: 'restricted', versions: 3, opens_30d: 48 },
  {
    ...base,
    team: 'spam-tests',
    slug: 'free-gift-cards',
    name: 'free-gift-cards',
    versions: 2,
    size_bytes: 4096,
    opens_30d: 412,
    last_publish: '2026-10-29T10:00:00Z',
  },
  {
    ...base,
    slug: 'internal-admin',
    name: 'Internal admin',
    access: 'off',
    versions: 1,
    opens_30d: 0,
    last_publish: null,
  },
  {
    ...base,
    team: 'initech',
    slug: 'old-link',
    name: 'Old link',
    link_disabled: true,
  },
];

export const page: CollectionsPage = { items: rows, next_cursor: null, total: rows.length };

export const stats: CollectionStats = {
  count: 1904,
  bytes_raw: 2_254_857_830,
  bytes_stored: 671_088_640,
};
