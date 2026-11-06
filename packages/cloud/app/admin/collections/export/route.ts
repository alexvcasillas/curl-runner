import { collectionsHref, parseCollectionsQuery } from '@/lib/admin-collections';
import { proxyCsv } from '@/lib/admin-csv';

/** The collections CSV: the page's `q` and filter, every matching row. */
export async function GET(request: Request): Promise<Response> {
  const params = new URL(request.url).searchParams;
  const { q, filter } = parseCollectionsQuery({
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
  return proxyCsv(
    '/v1/admin/collections.csv',
    query,
    collectionsHref({ q, filter, trail: [] }),
    'collections.csv',
  );
}
