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

const { GET } = await import('@/app/admin/collections/export/route');
const { resetEnv } = await import('@/lib/env');

const fetchMock = vi.fn();
beforeEach(() => {
  vi.stubEnv('API_INTERNAL_URL', 'http://api.test');
  resetEnv();
  jar.current = cookieJar({ cr_session: 'crs_ok' });
  fetchMock.mockReset();
  vi.stubGlobal('fetch', fetchMock);
});

const call = (qs = '') => GET(new Request(`http://localhost:3000/admin/collections/export${qs}`));

describe('GET /admin/collections/export', () => {
  it('streams the API CSV with the session bearer, q and filter', async () => {
    fetchMock.mockResolvedValue(
      new Response('email,name\r\nada@acme.com,Ada\r\n', {
        status: 200,
        headers: {
          'content-type': 'text/csv; charset=utf-8',
          'content-disposition': 'attachment; filename="collections-2026-11-06.csv"',
        },
      }),
    );
    const res = await call('?q=ada&filter=disabled&cursor=IGNORED');
    expect(res.status).toBe(200);
    expect(res.headers.get('content-type')).toContain('text/csv');
    expect(res.headers.get('content-disposition')).toBe(
      'attachment; filename="collections-2026-11-06.csv"',
    );
    expect(res.headers.get('cache-control')).toBe('no-store');
    expect(res.headers.get('x-content-type-options')).toBe('nosniff');
    expect(await res.text()).toContain('ada@acme.com');
    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toBe('http://api.test/v1/admin/collections.csv?q=ada&filter=disabled');
    expect(new Headers(init.headers).get('authorization')).toBe('Bearer crs_ok');
  });

  it('falls back to all collections for a filter it does not know', async () => {
    fetchMock.mockResolvedValue(new Response('email\r\n', { status: 200 }));
    await call('?filter=bogus');
    expect(String(fetchMock.mock.calls[0][0])).toBe('http://api.test/v1/admin/collections.csv');
  });

  it('sends a dead session to sign in and back', async () => {
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ error: 'unauthorized', message: '' }), { status: 401 }),
    );
    await expect(call('?q=ada')).rejects.toThrow(
      'NEXT_REDIRECT /sign-in/expired?return_to=%2Fadmin%2Fcollections%3Fq%3Dada',
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
