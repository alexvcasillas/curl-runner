// Next.js 16 renamed `middleware.ts` to `proxy.ts`; it runs on the Node.js runtime.
import { type NextRequest, NextResponse } from 'next/server';
import { PATHNAME_HEADER, SESSION_COOKIE } from './lib/constants';

// Prefix routes match the path itself and anything under it.
const PUBLIC_PREFIXES = ['/s/', '/invites/', '/.well-known/', '/_next/'];
const PUBLIC_EXACT = new Set(['/healthz', '/robots.txt', '/favicon.ico']);
// Files in `public/` sit at the top level.
const STATIC_FILE = /^\/[^/]+\.(?:png|svg|ico|txt|webmanifest|jpg|jpeg|webp|woff2?)$/i;

/** Routes anyone can open, signed in or not. */
export function isPublicPath(path: string): boolean {
  if (path === '/sign-in' || path.startsWith('/sign-in/')) {
    return true;
  }
  if (PUBLIC_EXACT.has(path) || STATIC_FILE.test(path)) {
    return true;
  }
  if (PUBLIC_PREFIXES.some((p) => path.startsWith(p))) {
    return true;
  }
  // The component kit, for checking the design locally.
  return process.env.NODE_ENV !== 'production' && path.startsWith('/dev/');
}

/**
 * Passes the current path to Server Components as `x-pathname`, for the API
 * client's 401 redirect, and sends signed-out requests for app routes to sign in.
 * Only checks that a session cookie exists: the API decides whether it's valid.
 */
export function proxy(request: NextRequest) {
  const { pathname, search } = request.nextUrl;
  const here = `${pathname}${search}`;

  if (!isPublicPath(pathname) && !request.cookies.get(SESSION_COOKIE)?.value) {
    const to = request.nextUrl.clone();
    to.pathname = '/sign-in';
    to.search = here === '/' ? '' : `?return_to=${encodeURIComponent(here)}`;
    return NextResponse.redirect(to);
  }

  const headers = new Headers(request.headers);
  headers.set(PATHNAME_HEADER, here);
  return NextResponse.next({ request: { headers } });
}

export const config = {
  matcher: ['/((?!_next/static|_next/image|favicon.ico|robots.txt).*)'],
};
