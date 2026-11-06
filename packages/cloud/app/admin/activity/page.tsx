import type { Metadata } from 'next';
import { ActivityView } from '@/components/admin/ActivityView';
import { parseActivityQuery, sinceIso } from '@/lib/admin-activity';
import { ACTIVITY_PAGE_SIZE, getActivity, getAdminMe, getAdmins } from '@/lib/admin-data';

export const metadata: Metadata = { title: 'Activity log' };

export default async function AdminActivityPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const query = parseActivityQuery(await searchParams);
  // The guard first: a non-admin gets a 404 before any list request goes out.
  await getAdminMe();
  const [page, admins] = await Promise.all([
    getActivity(query, sinceIso(query.since)),
    getAdmins(),
  ]);
  const people = admins.flatMap((a) =>
    a.user_id ? [{ id: a.user_id, label: a.name || a.email }] : [],
  );
  return <ActivityView page={page} people={people} query={query} pageSize={ACTIVITY_PAGE_SIZE} />;
}
