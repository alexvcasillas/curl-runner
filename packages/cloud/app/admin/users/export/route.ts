import { proxyCsv } from '@/lib/admin-csv';
import { parseUsersQuery, usersHref } from '@/lib/admin-users';

/** The users CSV: the page's `q` and filter, every matching row. */
export async function GET(request: Request): Promise<Response> {
  const params = new URL(request.url).searchParams;
  const { q, filter } = parseUsersQuery({
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
  return proxyCsv('/v1/admin/users.csv', query, usersHref({ q, filter, trail: [] }), 'users.csv');
}
