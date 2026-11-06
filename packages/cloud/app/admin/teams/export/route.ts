import { proxyCsv } from '@/lib/admin-csv';
import { parseTeamsQuery, teamsHref } from '@/lib/admin-teams';

/** The teams CSV: the page's `q` and filter, every matching row. */
export async function GET(request: Request): Promise<Response> {
  const params = new URL(request.url).searchParams;
  const { q, filter } = parseTeamsQuery({
    q: params.get('q') ?? undefined,
    filter: params.get('filter') ?? undefined,
  });
  const query = new URLSearchParams();
  if (q) {
    query.set('q', q);
  }
  if (filter !== 'all') {
    query.set('filter', filter);
  }
  return proxyCsv('/v1/admin/teams.csv', query, teamsHref({ q, filter, trail: [] }), 'teams.csv');
}
