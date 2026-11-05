import 'server-only';
import { cookies } from 'next/headers';
import { SESSION_COOKIE } from './constants';
import { env } from './env';

/** 90 days. */
export const SESSION_MAX_AGE = 7_776_000;

/** Plain http only for local development: `http://localhost`, any port. */
function isLocalHttp(appUrl: string): boolean {
  const u = new URL(appUrl);
  return u.protocol === 'http:' && u.hostname === 'localhost';
}

/** The options every cookie here shares. `path` narrows a short-lived one to its pages. */
export function cookieOptions(maxAge: number, path = '/') {
  return {
    httpOnly: true,
    secure: !isLocalHttp(env().PUBLIC_APP_URL),
    sameSite: 'lax' as const,
    path,
    maxAge,
  };
}

/** The `crs_` session token. Never send it to the browser. */
export async function getSession(): Promise<string | null> {
  return (await cookies()).get(SESSION_COOKIE)?.value || null;
}

/** Works only in a Server Action or Route Handler. */
export async function setSession(token: string): Promise<void> {
  (await cookies()).set(SESSION_COOKIE, token, cookieOptions(SESSION_MAX_AGE));
}

/** Works only in a Server Action or Route Handler. */
export async function clearSession(): Promise<void> {
  (await cookies()).set(SESSION_COOKIE, '', cookieOptions(0));
}
