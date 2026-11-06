// What the admin collections page shares: wire shapes, the list's URL state and wording.
// Safe to import from client components.

import {
  one,
  pagingParams,
  parseCursor,
  parseSearch,
  parseTrail,
  type RawParams,
} from './admin-paging';

export type CollectionAccess = 'off' | 'public' | 'restricted';

/** One row of `GET /v1/admin/collections`. */
export type CollectionRow = {
  team: string;
  slug: string;
  name: string;
  source: 'git' | 'cloud';
  access: CollectionAccess;
  versions: number;
  /** The current version's files, in bytes. */
  size_bytes: number;
  /** Signed-in opens in the last 30 days, at most one per person a day. */
  opens_30d: number;
  last_publish: string | null;
  link_disabled: boolean;
};

export type CollectionsPage = { items: CollectionRow[]; next_cursor: string | null; total: number };

/** `GET /v1/admin/collections/stats`: totals over every collection, whatever the filters. */
export type CollectionStats = { count: number; bytes_raw: number; bytes_stored: number };

export const COLLECTION_FILTERS = [
  { value: 'all', label: 'All' },
  { value: 'public', label: 'Public' },
  { value: 'restricted', label: 'Restricted' },
  { value: 'disabled', label: 'Disabled by admin' },
] as const;

export type CollectionFilter = (typeof COLLECTION_FILTERS)[number]['value'];

export function isCollectionFilter(value: unknown): value is CollectionFilter {
  return COLLECTION_FILTERS.some((f) => f.value === value);
}

export type CollectionsQuery = {
  q: string;
  filter: CollectionFilter;
  cursor: string | undefined;
  /** The cursors of the pages before this one, oldest first, so Previous can go back. */
  trail: string[];
};

/** The list's search params, checked: anything unknown falls back to the first page of every collection. */
export function parseCollectionsQuery(sp: RawParams): CollectionsQuery {
  const filter = one(sp.filter);
  return {
    q: parseSearch(sp.q),
    filter: isCollectionFilter(filter) ? filter : 'all',
    cursor: parseCursor(sp.cursor),
    trail: parseTrail(sp.trail),
  };
}

function params(query: Pick<CollectionsQuery, 'q' | 'filter'>) {
  const p = new URLSearchParams();
  if (query.q) {
    p.set('q', query.q);
  }
  if (query.filter !== 'all') {
    p.set('filter', query.filter);
  }
  return p;
}

export function collectionsHref(
  query: Pick<CollectionsQuery, 'q' | 'filter' | 'trail'> & { cursor?: string },
) {
  const p = params(query);
  pagingParams(p, query);
  const qs = p.toString();
  return qs ? `/admin/collections?${qs}` : '/admin/collections';
}

export function collectionsExportHref(query: Pick<CollectionsQuery, 'q' | 'filter'>) {
  const qs = params(query).toString();
  return qs ? `/admin/collections/export?${qs}` : '/admin/collections/export';
}

// ---- Wording ----

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];

/** "84 KB", "9 KB", "1.2 MB", "2.1 GB". 1024 to the step. */
export function formatBytes(bytes: number): string {
  let value = Math.max(0, bytes);
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const decimals = unit >= 2 || value < 10 ? 1 : 0;
  const text = value.toFixed(decimals).replace(/\.0$/, '');
  return `${text} ${UNITS[unit]}`;
}

export function accessLabel(access: CollectionAccess): string {
  return access === 'public' ? 'Public' : access === 'restricted' ? 'Restricted' : 'Off';
}
