import { NextResponse } from 'next/server';
import { ApiError, api } from '@/lib/api';
import { env } from '@/lib/env';
import { safeReturnTo } from '@/lib/return-to';
import { clearSession, getSession } from '@/lib/session';

/** The account menu posts here. Ends the session on the server, then drops the cookie. */
export async function POST(request?: Request) {
  // A form on another site can post here with the cookie attached by some browsers.
  const origin = request?.headers.get('origin');
  if (origin && origin !== new URL(env().PUBLIC_APP_URL).origin) {
    return new NextResponse(null, { status: 403 });
  }
  if (await getSession()) {
    try {
      await api('/v1/auth/sign-out', { method: 'POST', auth: 'optional' });
    } catch (e) {
      // Already revoked (401) or the API is down: the cookie goes either way.
      if (!(e instanceof ApiError)) {
        throw e;
      }
    }
  }
  await clearSession();
  // A page that offers "Use another account" says where to come back to.
  const returnTo = safeReturnTo(
    request ? new URL(request.url).searchParams.get('return_to') : null,
  );
  const to = new URL('/sign-in', env().PUBLIC_APP_URL);
  if (returnTo !== '/') {
    to.searchParams.set('return_to', returnTo);
  }
  // 303 turns the POST into a GET of the sign-in page.
  return NextResponse.redirect(to, 303);
}
