import type { Metadata } from 'next';
import { TeamsView } from '@/components/admin/TeamsView';
import { getAdminMe, getTeams, TEAMS_PAGE_SIZE } from '@/lib/admin-data';
import { parseTeamsQuery } from '@/lib/admin-teams';

export const metadata: Metadata = { title: 'Teams' };

export default async function AdminTeamsPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const query = parseTeamsQuery(await searchParams);
  // The guard first: a non-admin gets a 404 before any list request goes out.
  await getAdminMe();
  const page = await getTeams(query);
  return (
    <TeamsView
      page={page}
      q={query.q}
      filter={query.filter}
      cursor={query.cursor}
      trail={query.trail}
      pageSize={TEAMS_PAGE_SIZE}
      now={new Date().toISOString()}
    />
  );
}
