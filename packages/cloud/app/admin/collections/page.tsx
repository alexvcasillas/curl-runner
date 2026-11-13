import type { Metadata } from 'next';
import { CollectionsView } from '@/components/admin/CollectionsView';
import { parseCollectionsQuery } from '@/lib/admin-collections';
import {
  COLLECTIONS_PAGE_SIZE,
  getAdminMe,
  getCollectionStats,
  getCollections,
} from '@/lib/admin-data';
import { disableLink, enableLink, unlistCollection } from './actions';

export const metadata: Metadata = { title: 'Collections' };

export default async function AdminCollectionsPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const query = parseCollectionsQuery(await searchParams);
  // The guard first: a non-admin gets a 404 before any list request goes out.
  const admin = await getAdminMe();
  const [page, stats] = await Promise.all([getCollections(query), getCollectionStats()]);
  return (
    <CollectionsView
      page={page}
      stats={stats}
      q={query.q}
      filter={query.filter}
      cursor={query.cursor}
      trail={query.trail}
      pageSize={COLLECTIONS_PAGE_SIZE}
      now={new Date().toISOString()}
      permissions={admin.permissions}
      actions={{ disable: disableLink, enable: enableLink, unlist: unlistCollection }}
    />
  );
}
