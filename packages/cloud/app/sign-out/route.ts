import { NextResponse } from 'next/server';
import { ApiError, api } from '@/lib/api';
import { env } from '@/lib/env';
import { clearSession, getSession } from '@/lib/session';

/** The account menu posts here. Ends the session on the server, then drops the cookie. */
export async function POST() {
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
  // 303 turns the POST into a GET of the sign-in page.
  return NextResponse.redirect(new URL('/sign-in', env().PUBLIC_APP_URL), 303);
}
