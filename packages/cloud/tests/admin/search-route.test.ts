import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };
vi.mock('next/headers', () => ({
  cookies: async () => jar.current.store,
  headers: async () => new Headers(),
}));
vi.mock('next/navigation', () => ({
  notFound: () => {
    throw new Error('NEXT_NOT_FOUND');
  },
  redirect: (url: string) => {
    throw new Error(`NEXT_REDIRECT ${url}`);
  },
}));

const { GET } = await import('@/app/admin/search/route');
const { resetEnv } = await import('@/lib/env');

const fetchMock = vi.fn();
const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

beforeEach(() => {
  vi.stubEnv('API_INTERNAL_URL', 'http://api.test');
  vi.stubEnv('PUBLIC_APP_URL', 'http://localhost:3000');
  resetEnv();
  fetchMock.mockReset();
  vi.stubGlobal('fetch', fetchMock);
});

const ME = { role: 'support', permissions: [] };
const me = (url: string) => new URL(url).pathname === '/v1/admin/me';

const call = (q: string) =>
  GET(new Request(`http://localhost:3000/admin/search?q=${encodeURIComponent(q)}`));

describe('GET /admin/search', () => {
  it('asks for five of each and keeps only what the palette shows', async () => {
    const urls: string[] = [];
    fetchMock.mockImplementation(async (url: string) => {
      const u = new URL(url);
      if (u.pathname === '/v1/admin/me') {
        return json(ME);
      }
      urls.push(url);
      if (u.pathname === '/v1/admin/users') {
        return json({
          items: [{ id: 'u1', email: 'a@b.co', name: null, teams: [], methods: ['email'] }],
          total: 1,
        });
      }
      if (u.pathname === '/v1/admin/teams') {
        return json({
          items: [{ slug: 'acme', name: 'Acme', plan: 'team', owner: { email: 'o@b.co' } }],
          total: 1,
        });
      }
      return json({ items: [{ team: 'acme', slug: 'api', name: 'API', size_bytes: 3 }], total: 1 });
    });
    const res = await call('ac me');
    expect(await res.json()).toEqual({
      users: [{ id: 'u1', email: 'a@b.co', name: null }],
      teams: [{ slug: 'acme', name: 'Acme', plan: 'team' }],
      collections: [{ team: 'acme', slug: 'api', name: 'API' }],
    });
    expect(urls).toHaveLength(3);
    for (const u of urls) {
      expect(new URL(u).searchParams.get('q')).toBe('ac me');
      expect(new URL(u).searchParams.get('limit')).toBe('5');
    }
  });

  it('is a 404 for a non-admin', async () => {
    fetchMock.mockImplementation(async () => json({ error: 'not_found', message: '' }, 404));
    await expect(call('x')).rejects.toThrow('NEXT_NOT_FOUND');
  });

  it('is a 404 for a non-admin even with an empty query', async () => {
    fetchMock.mockImplementation(async () => json({ error: 'not_found', message: '' }, 404));
    await expect(call('  ')).rejects.toThrow('NEXT_NOT_FOUND');
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(me(String(fetchMock.mock.calls[0][0]))).toBe(true);
  });

  it('answers an empty query from an admin without searching', async () => {
    fetchMock.mockImplementation(async () => json(ME));
    const res = await call('  ');
    expect(await res.json()).toEqual({ users: [], teams: [], collections: [] });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(me(String(fetchMock.mock.calls[0][0]))).toBe(true);
  });

  it('passes an API outage on as a 503', async () => {
    fetchMock.mockImplementation(async () => json({ error: 'unavailable', message: '' }, 503));
    expect((await call('x')).status).toBe(503);
  });
});
