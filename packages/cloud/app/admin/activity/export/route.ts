import { redirect } from 'next/navigation';
import { activityHref, parseActivityQuery, sinceIso } from '@/lib/admin-activity';
import { env } from '@/lib/env';
import { getSession } from '@/lib/session';

function text(status: number, body: string): Response {
  return new Response(body, {
    status,
    headers: { 'content-type': 'text/plain; charset=utf-8', 'cache-control': 'no-store' },
  });
}

/**
 * The activity CSV, streamed from the API with the session as the bearer. The
 * same filters as the page, every matching row. The API decides who may read
 * it: a non-admin gets a plain 404, like every admin path.
 */
export async function GET(request: Request): Promise<Response> {
  const params = new URL(request.url).searchParams;
  const { admin, group, since } = parseActivityQuery({
    admin: params.get('admin') ?? undefined,
    group: params.get('group') ?? undefined,
    since: params.get('since') ?? undefined,
  });
  const back = activityHref({ admin, group, since, trail: [] });
  const token = await getSession();
  if (!token) {
    redirect(`/sign-in?return_to=${encodeURIComponent(back)}`);
  }

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
  const qs = query.toString();

  let upstream: Response;
  try {
    upstream = await fetch(`${env().API_INTERNAL_URL}/v1/admin/activity.csv${qs ? `?${qs}` : ''}`, {
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
  headers.set('content-disposition', disposition ?? 'attachment; filename="activity.csv"');
  return new Response(upstream.body, { status: 200, headers });
}
