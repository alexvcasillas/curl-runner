import { NextResponse } from 'next/server';
import { env } from '@/lib/env';
import { safeReturnTo } from '@/lib/return-to';
import { clearSession } from '@/lib/session';

/**
 * Where `api()` sends a rejected session. A Server Component can't delete the
 * cookie, so this does, then sends the person to sign in and back.
 */
export async function GET(request: Request) {
  await clearSession();
  const returnTo = safeReturnTo(new URL(request.url).searchParams.get('return_to'));
  const to = new URL('/sign-in', env().PUBLIC_APP_URL);
  if (returnTo !== '/') {
    to.searchParams.set('return_to', returnTo);
  }
  return NextResponse.redirect(to);
}
