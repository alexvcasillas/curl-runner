import { activityHref, parseActivityQuery, sinceIso } from '@/lib/admin-activity';
import { proxyCsv } from '@/lib/admin-csv';

/** The activity CSV: the same filters as the page, every matching row. */
export async function GET(request: Request): Promise<Response> {
  const params = new URL(request.url).searchParams;
  const { admin, group, since } = parseActivityQuery({
    admin: params.get('admin') ?? undefined,
    group: params.get('group') ?? undefined,
    since: params.get('since') ?? undefined,
  });
  const query = new URLSearchParams();
  if (admin) {
    query.set('admin', admin);
  }
  if (group !== 'all') {
    query.set('action_group', group);
  }
  const from = sinceIso(since);
  if (from) {
    query.set('since', from);
  }
  return proxyCsv(
    '/v1/admin/activity.csv',
    query,
    activityHref({ admin, group, since, trail: [] }),
    'activity.csv',
  );
}
