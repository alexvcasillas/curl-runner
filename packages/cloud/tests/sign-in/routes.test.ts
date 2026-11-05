// @vitest-environment node
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';

const jar = { current: cookieJar() };
const incoming = { current: new Headers() };

vi.mock('next/headers', () => ({
  cookies: async () => jar.current.store,
  headers: async () => incoming.current,
}));
vi.mock('next/navigation', () => ({
  redirect: (url: string) => {
    throw new Error(`NEXT_REDIRECT ${url}`);
  },
}));

const expired = await import('@/app/sign-in/expired/route');
const signOut = await import('@/app/sign-out/route');
const { resetEnv } = await import('@/lib/env');

const fetchMock = vi.fn();

beforeEach(() => {
  resetEnv();
  vi.stubEnv('API_INTERNAL_URL', 'http://api.internal:8080');
  vi.stubEnv('PUBLIC_APP_URL', 'https://cloud.example.com');
  vi.stubGlobal('fetch', fetchMock);
  fetchMock.mockReset();
  jar.current = cookieJar({ cr_session: 'crs_old' });
  incoming.current = new Headers();
});
afterEach(() => {
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
  resetEnv();
});

const get = (qs: string) =>
  expired.GET(new Request(`https://cloud.example.com/sign-in/expired${qs}`));

describe('GET /sign-in/expired', () => {
  it('clears the cookie and goes to sign in with the return_to', async () => {
    const res = await get(`?return_to=${encodeURIComponent('/t/acme/tokens?x=1')}`);
    expect(res.status).toBe(307);
    expect(res.headers.get('location')).toBe(
      `https://cloud.example.com/sign-in?return_to=${encodeURIComponent('/t/acme/tokens?x=1')}`,
    );
    expect(jar.current.values.has('cr_session')).toBe(false);
    expect(jar.current.sets[0]).toMatchObject({ name: 'cr_session', value: '' });
  });

  it('sends a hostile return_to to /', async () => {
    for (const bad of ['//evil.example', 'https://evil.example', '/\\evil', '%2F%2Fevil']) {
      const res = await get(`?return_to=${encodeURIComponent(bad)}`);
      expect(res.headers.get('location')).toBe('https://cloud.example.com/sign-in');
    }
  });

  it('does not add return_to when there is none', async () => {
    const res = await get('');
    expect(res.headers.get('location')).toBe('https://cloud.example.com/sign-in');
  });
});

describe('POST /sign-out', () => {
  it('calls the API, clears the cookie and goes to sign in with a 303', async () => {
    fetchMock.mockResolvedValueOnce(new Response(null, { status: 204 }));
    const res = await signOut.POST();
    expect(fetchMock.mock.calls[0][0]).toBe('http://api.internal:8080/v1/auth/sign-out');
    expect(fetchMock.mock.calls[0][1].method).toBe('POST');
    expect(new Headers(fetchMock.mock.calls[0][1].headers).get('authorization')).toBe(
      'Bearer crs_old',
    );
    expect(res.status).toBe(303);
    expect(res.headers.get('location')).toBe('https://cloud.example.com/sign-in');
    expect(jar.current.values.has('cr_session')).toBe(false);
  });

  it('clears the cookie even when the API says 401', async () => {
    fetchMock.mockResolvedValueOnce(
      new Response(JSON.stringify({ error: 'unauthorized', message: 'Sign in.' }), {
        status: 401,
        headers: { 'content-type': 'application/json' },
      }),
    );
    const res = await signOut.POST();
    expect(res.status).toBe(303);
    expect(jar.current.values.has('cr_session')).toBe(false);
  });

  it('clears the cookie when the API is down, and skips the call without one', async () => {
    fetchMock.mockRejectedValueOnce(new TypeError('fetch failed'));
    expect((await signOut.POST()).status).toBe(303);
    expect(jar.current.values.has('cr_session')).toBe(false);
    jar.current = cookieJar();
    fetchMock.mockReset();
    expect((await signOut.POST()).status).toBe(303);
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
