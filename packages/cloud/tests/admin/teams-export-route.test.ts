import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };
vi.mock('next/headers', () => ({
  cookies: async () => jar.current.store,
  headers: async () => new Headers(),
}));
vi.mock('next/navigation', () => ({
  redirect: (url: string) => {
    throw new Error(`NEXT_REDIRECT ${url}`);
  },
}));

const { GET } = await import('@/app/admin/teams/export/route');
const { resetEnv } = await import('@/lib/env');

const fetchMock = vi.fn();
beforeEach(() => {
  vi.stubEnv('API_INTERNAL_URL', 'http://api.test');
  resetEnv();
  jar.current = cookieJar({ cr_session: 'crs_ok' });
  fetchMock.mockReset();
  vi.stubGlobal('fetch', fetchMock);
});

const call = (qs = '') => GET(new Request(`http://localhost:3000/admin/teams/export${qs}`));

describe('GET /admin/teams/export', () => {
  it('streams the API CSV with the session bearer, q and filter', async () => {
    fetchMock.mockResolvedValue(
      new Response('email,name\r\nada@acme.com,Ada\r\n', {
        status: 200,
        headers: {
          'content-type': 'text/csv; charset=utf-8',
          'content-disposition': 'attachment; filename="teams-2026-11-06.csv"',
        },
      }),
    );
    const res = await call('?q=ada&filter=suspended&cursor=IGNORED');
    expect(res.status).toBe(200);
    expect(res.headers.get('content-type')).toContain('text/csv');
    expect(res.headers.get('content-disposition')).toBe(
      'attachment; filename="teams-2026-11-06.csv"',
    );
    expect(res.headers.get('cache-control')).toBe('no-store');
    expect(res.headers.get('x-content-type-options')).toBe('nosniff');
    expect(await res.text()).toContain('ada@acme.com');
    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toBe('http://api.test/v1/admin/teams.csv?q=ada&filter=suspended');
    expect(new Headers(init.headers).get('authorization')).toBe('Bearer crs_ok');
  });

  it('falls back to all users for a filter it does not know', async () => {
    fetchMock.mockResolvedValue(new Response('email\r\n', { status: 200 }));
    await call('?filter=bogus');
    expect(String(fetchMock.mock.calls[0][0])).toBe('http://api.test/v1/admin/teams.csv');
  });

  it('sends a dead session to sign in and back', async () => {
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ error: 'unauthorized', message: '' }), { status: 401 }),
    );
    await expect(call('?q=ada')).rejects.toThrow(
      'NEXT_REDIRECT /sign-in/expired?return_to=%2Fadmin%2Fteams%3Fq%3Dada',
    );
  });

  it('is a plain 404 for a non-admin', async () => {
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ error: 'not_found', message: '' }), { status: 404 }),
    );
    const res = await call();
    expect(res.status).toBe(404);
    expect(await res.text()).toBe('Not found.');
  });

  it('says 503 when the API is down, and never leaks the API URL', async () => {
    fetchMock.mockRejectedValue(new Error('connect ECONNREFUSED http://api.test'));
    const res = await call();
    expect(res.status).toBe(503);
    expect(await res.text()).not.toContain('api.test');
  });
});
