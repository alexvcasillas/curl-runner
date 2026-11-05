import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';

const jar = { current: cookieJar() };
const incoming = { current: new Headers() };

vi.mock('next/headers', () => ({
  cookies: async () => jar.current.store,
  headers: async () => incoming.current,
}));

class RedirectSignal extends Error {
  constructor(public url: string) {
    super(`NEXT_REDIRECT ${url}`);
  }
}
vi.mock('next/navigation', () => ({
  redirect: (url: string) => {
    throw new RedirectSignal(url);
  },
}));

const { api } = await import('@/lib/api');
const { ApiError } = await import('@/lib/errors');
const { resetEnv } = await import('@/lib/env');

function json(status: number, body: unknown, init: ResponseInit = {}) {
  return new Response(JSON.stringify(body), {
    ...init,
    status,
    headers: { 'content-type': 'application/json', ...(init.headers ?? {}) },
  });
}

describe('api', () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    resetEnv();
    vi.stubEnv('API_INTERNAL_URL', 'http://api.internal:8080');
    vi.stubEnv('PUBLIC_APP_URL', 'https://cloud.example.com');
    vi.stubGlobal('fetch', fetchMock);
    fetchMock.mockReset();
    jar.current = cookieJar({ cr_session: 'crs_token' });
    incoming.current = new Headers({
      'x-forwarded-for': '203.0.113.9, 198.51.100.7',
      'x-pathname': '/t/acme/tokens?x=1',
    });
  });
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.unstubAllGlobals();
    resetEnv();
  });

  it('sends the bearer, X-Forwarded-For and JSON to the internal URL', async () => {
    fetchMock.mockResolvedValue(json(200, { ok: true }));
    const out = await api<{ ok: boolean }>('/v1/teams', { method: 'POST', json: { name: 'Acme' } });
    expect(out).toEqual({ ok: true });
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('http://api.internal:8080/v1/teams');
    expect(init.method).toBe('POST');
    expect(init.cache).toBe('no-store');
    expect(init.body).toBe('{"name":"Acme"}');
    const h = new Headers(init.headers);
    expect(h.get('authorization')).toBe('Bearer crs_token');
    expect(h.get('x-forwarded-for')).toBe('198.51.100.7');
    expect(h.get('content-type')).toBe('application/json');
    expect(h.get('accept')).toBe('application/json');
  });

  it('leaves out the bearer with auth none, and X-Forwarded-For without a client ip', async () => {
    incoming.current = new Headers();
    fetchMock.mockResolvedValue(json(200, {}));
    await api('/v1/info', { auth: 'none' });
    const h = new Headers(fetchMock.mock.calls[0][1].headers);
    expect(h.get('authorization')).toBeNull();
    expect(h.get('x-forwarded-for')).toBeNull();
    expect(fetchMock.mock.calls[0][1].method).toBe('GET');
  });

  it('gives undefined for 204', async () => {
    fetchMock.mockResolvedValue(new Response(null, { status: 204 }));
    await expect(api('/v1/x', { method: 'DELETE' })).resolves.toBeUndefined();
  });

  it('throws ApiError with code and message from the body', async () => {
    fetchMock.mockResolvedValue(json(409, { error: 'conflict', message: 'That slug is taken.' }));
    const err = await api('/v1/teams').catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect(err).toMatchObject({ status: 409, code: 'conflict', message: 'That slug is taken.' });
  });

  it('maps an unknown code and a non-JSON body', async () => {
    fetchMock.mockResolvedValueOnce(json(400, { error: 'brand_new', message: 'New.' }));
    await expect(api('/v1/x')).rejects.toMatchObject({ code: 'unknown', message: 'New.' });
    fetchMock.mockResolvedValueOnce(new Response('<html>bad gateway</html>', { status: 502 }));
    await expect(api('/v1/x')).rejects.toMatchObject({ status: 502, code: 'unavailable' });
  });

  it('turns a network failure into unavailable', async () => {
    fetchMock.mockRejectedValue(new TypeError('fetch failed'));
    await expect(api('/v1/x')).rejects.toMatchObject({ status: 503, code: 'unavailable' });
  });

  it('reads retryAfter from Retry-After', async () => {
    fetchMock.mockResolvedValue(
      json(
        429,
        { error: 'rate_limited', message: 'Slow down.' },
        { headers: { 'retry-after': '720' } },
      ),
    );
    await expect(api('/v1/x')).rejects.toMatchObject({ code: 'rate_limited', retryAfter: 720 });
  });

  it('on 401 with auth session, clears the cookie and redirects with return_to', async () => {
    fetchMock.mockResolvedValue(json(401, { error: 'unauthorized', message: 'Sign in.' }));
    const err = await api('/v1/me').catch((e: unknown) => e);
    expect(err).toBeInstanceOf(RedirectSignal);
    expect((err as RedirectSignal).url).toBe(
      `/sign-in?return_to=${encodeURIComponent('/t/acme/tokens?x=1')}`,
    );
    expect(await jar.current.store.get('cr_session')).toBeUndefined();
  });

  it('still redirects when the cookie cannot be changed (a Server Component render)', async () => {
    jar.current.store.set.mockImplementation(() => {
      throw new Error('Cookies can only be modified in a Server Action or Route Handler.');
    });
    fetchMock.mockResolvedValue(json(401, { error: 'unauthorized', message: 'Sign in.' }));
    await expect(api('/v1/me')).rejects.toBeInstanceOf(RedirectSignal);
  });

  it('redirects without calling the API when there is no session', async () => {
    jar.current = cookieJar();
    await expect(api('/v1/me')).rejects.toBeInstanceOf(RedirectSignal);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('throws on 401 with auth optional, without redirecting', async () => {
    fetchMock.mockResolvedValue(json(401, { error: 'unauthorized', message: 'Sign in.' }));
    await expect(api('/v1/me', { auth: 'optional' })).rejects.toBeInstanceOf(ApiError);
    expect(await jar.current.store.get('cr_session')).toEqual({
      name: 'cr_session',
      value: 'crs_token',
    });
  });

  it('throws code_invalid (also a 401) without redirecting', async () => {
    fetchMock.mockResolvedValue(json(401, { error: 'code_invalid', message: 'Wrong code.' }));
    await expect(api('/v1/auth/email/verify')).rejects.toMatchObject({ code: 'code_invalid' });
  });
});
