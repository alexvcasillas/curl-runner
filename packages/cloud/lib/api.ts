import 'server-only';
import { headers } from 'next/headers';
import { redirect } from 'next/navigation';
import { PATHNAME_HEADER } from './constants';
import { env } from './env';
import { ApiError, toErrorCode } from './errors';
import { getSession } from './session';

export { ApiError } from './errors';

export type ApiInit = {
  method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  json?: unknown;
  /**
   * `session` (default) needs a signed-in user and sends them to sign in on a 401.
   * `optional` sends the session when there is one. `none` never does.
   */
  auth?: 'session' | 'optional' | 'none';
};

/**
 * The browser's IP: the right-most `X-Forwarded-For` entry (the one our proxy
 * appended), else `X-Real-IP`, else null.
 */
export function clientIp(h: Headers): string | null {
  const forwarded = h
    .get('x-forwarded-for')
    ?.split(',')
    .map((s) => s.trim())
    .filter(Boolean);
  if (forwarded?.length) {
    return forwarded[forwarded.length - 1];
  }
  return h.get('x-real-ip')?.trim() || null;
}

/**
 * Sends the person to sign in and back. A render can't delete the dead cookie, so
 * a rejected session goes through `/sign-in/expired`, a route handler that does.
 */
function signInAgain(incoming: Headers, sessionRejected: boolean): never {
  const here = encodeURIComponent(incoming.get(PATHNAME_HEADER) || '/');
  redirect(sessionRejected ? `/sign-in/expired?return_to=${here}` : `/sign-in?return_to=${here}`);
}

function retryAfter(res: Response): number | undefined {
  const value = res.headers.get('retry-after');
  if (!value) {
    return undefined;
  }
  const seconds = Number(value);
  return Number.isFinite(seconds) && seconds >= 0 ? Math.ceil(seconds) : undefined;
}

async function toApiError(res: Response): Promise<ApiError> {
  let body: unknown = null;
  try {
    body = await res.json();
  } catch {
    // Not JSON: a proxy's error page, most likely.
  }
  if (body && typeof body === 'object' && 'error' in body) {
    const { error, message } = body as { error: unknown; message?: unknown };
    return new ApiError(
      res.status,
      toErrorCode(error),
      typeof message === 'string' ? message : '',
      retryAfter(res),
    );
  }
  const code = res.status >= 500 ? 'unavailable' : 'unknown';
  return new ApiError(res.status, code, '', retryAfter(res));
}

/** Calls the API on the private network. Server only: the session token stays here. */
export async function api<T>(path: string, init: ApiInit = {}): Promise<T> {
  const auth = init.auth ?? 'session';
  const incoming = await headers();
  const token = auth === 'none' ? null : await getSession();
  if (auth === 'session' && !token) {
    return signInAgain(incoming, false);
  }

  const h = new Headers({ accept: 'application/json' });
  if (init.json !== undefined) {
    h.set('content-type', 'application/json');
  }
  if (token) {
    h.set('authorization', `Bearer ${token}`);
  }
  const ip = clientIp(incoming);
  if (ip) {
    h.set('x-forwarded-for', ip);
  }

  let res: Response;
  try {
    res = await fetch(`${env().API_INTERNAL_URL}${path}`, {
      method: init.method ?? 'GET',
      headers: h,
      body: init.json === undefined ? undefined : JSON.stringify(init.json),
      cache: 'no-store',
      redirect: 'manual',
    });
  } catch {
    throw new ApiError(503, 'unavailable', '');
  }

  if (!res.ok) {
    const err = await toApiError(res);
    if (err.code === 'unauthorized' && auth === 'session') {
      return signInAgain(incoming, true);
    }
    throw err;
  }
  if (res.status === 204) {
    return undefined as T;
  }
  const text = await res.text();
  return (text ? JSON.parse(text) : undefined) as T;
}
