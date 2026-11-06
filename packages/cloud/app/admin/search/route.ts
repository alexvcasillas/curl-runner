import { NextResponse } from 'next/server';
import type { SearchResults } from '@/lib/admin';
import { api } from '@/lib/api';
import { ApiError } from '@/lib/errors';

type Page<T> = { items: T[] };

const EMPTY: SearchResults = { users: [], teams: [], collections: [] };

/**
 * The ⌘K palette's data: five users, teams and collections. The browser can't
 * call the API itself (the session token stays on the server), so it asks here.
 * A non-admin gets the same 404 as every other admin path.
 */
export async function GET(request: Request) {
  const q = new URL(request.url).searchParams.get('q')?.trim() ?? '';
  if (!q) {
    return NextResponse.json(EMPTY);
  }
  const query = `?q=${encodeURIComponent(q)}&limit=5`;
  try {
    const [users, teams, collections] = await Promise.all([
      api<Page<SearchResults['users'][number]>>(`/v1/admin/users${query}`),
      api<Page<SearchResults['teams'][number]>>(`/v1/admin/teams${query}`),
      api<Page<SearchResults['collections'][number]>>(`/v1/admin/collections${query}`),
    ]);
    const body: SearchResults = {
      users: users.items.map(({ id, email, name }) => ({ id, email, name })),
      teams: teams.items.map(({ slug, name, plan }) => ({ slug, name, plan })),
      collections: collections.items.map(({ team, slug, name }) => ({ team, slug, name })),
    };
    return NextResponse.json(body, { headers: { 'cache-control': 'no-store' } });
  } catch (err) {
    if (err instanceof ApiError) {
      return NextResponse.json({ error: err.code }, { status: err.status === 404 ? 404 : 503 });
    }
    throw err;
  }
}
