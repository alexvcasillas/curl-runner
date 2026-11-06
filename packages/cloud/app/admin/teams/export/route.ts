import { redirect } from 'next/navigation';
import { parseTeamsQuery, teamsHref } from '@/lib/admin-teams';
import { env } from '@/lib/env';
import { getSession } from '@/lib/session';

function text(status: number, body: string): Response {
  return new Response(body, {
    status,
    headers: { 'content-type': 'text/plain; charset=utf-8', 'cache-control': 'no-store' },
  });
}

/**
 * The teams CSV, streamed from the API with the session as the bearer. The
 * browser never sees the API's address. The API decides who may read it: a
 * non-admin gets a plain 404, like every admin path.
 */
export async function GET(request: Request): Promise<Response> {
  const params = new URL(request.url).searchParams;
  const { q, filter } = parseTeamsQuery({
    q: params.get('q') ?? undefined,
    filter: params.get('filter') ?? undefined,
  });
  const back = teamsHref({ q, filter, trail: [] });
  const token = await getSession();
  if (!token) {
    redirect(`/sign-in?return_to=${encodeURIComponent(back)}`);
  }

  const query = new URLSearchParams();
  if (q) {
    query.set('q', q);
  }
  if (filter !== 'all') {
    query.set('filter', filter);
  }
  const qs = query.toString();

  let upstream: Response;
  try {
    upstream = await fetch(`${env().API_INTERNAL_URL}/v1/admin/teams.csv${qs ? `?${qs}` : ''}`, {
      headers: { authorization: `Bearer ${token}`, accept: 'text/csv' },
      cache: 'no-store',
      redirect: 'manual',
    });
  } catch {
    return text(503, 'Cloud isn’t answering right now. Try again in a minute.');
  }

  if (upstream.status === 401) {
    // A dead session: the expired route clears the cookie and asks them to sign in again.
    redirect(`/sign-in/expired?return_to=${encodeURIComponent(back)}`);
  }
  if (upstream.status === 404) {
    return text(404, 'Not found.');
  }
  if (upstream.status === 403) {
    return text(403, 'You can’t download this.');
  }
  if (!upstream.ok || !upstream.body) {
    return text(503, 'Cloud isn’t answering right now. Try again in a minute.');
  }
  const headers = new Headers({
    'content-type': 'text/csv; charset=utf-8',
    'cache-control': 'no-store',
    'x-content-type-options': 'nosniff',
  });
  const disposition = upstream.headers.get('content-disposition');
  headers.set('content-disposition', disposition ?? 'attachment; filename="teams.csv"');
  return new Response(upstream.body, { status: 200, headers });
}
