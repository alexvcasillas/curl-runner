import { redirect } from 'next/navigation';
import { env } from './env';
import { getSession } from './session';

function text(status: number, body: string): Response {
  return new Response(body, {
    status,
    headers: { 'content-type': 'text/plain; charset=utf-8', 'cache-control': 'no-store' },
  });
}

/**
 * An admin CSV, streamed from the API with the session as the bearer. The
 * browser never sees the API's address. The API decides who may read it: a
 * non-admin gets a plain 404, like every admin path. `back` is where sign-in
 * returns to; `fallbackName` is the file name when the API sends none.
 */
export async function proxyCsv(
  path: string,
  params: URLSearchParams,
  back: string,
  fallbackName: string,
): Promise<Response> {
  const token = await getSession();
  if (!token) {
    redirect(`/sign-in?return_to=${encodeURIComponent(back)}`);
  }
  const qs = params.toString();

  let upstream: Response;
  try {
    upstream = await fetch(`${env().API_INTERNAL_URL}${path}${qs ? `?${qs}` : ''}`, {
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
  headers.set('content-disposition', disposition ?? `attachment; filename="${fallbackName}"`);
  return new Response(upstream.body, { status: 200, headers });
}
