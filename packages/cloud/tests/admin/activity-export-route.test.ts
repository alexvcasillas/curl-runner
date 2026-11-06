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

const { GET } = await import('@/app/admin/activity/export/route');
const { resetEnv } = await import('@/lib/env');

const fetchMock = vi.fn();
beforeEach(() => {
  vi.stubEnv('API_INTERNAL_URL', 'http://api.test');
  resetEnv();
  jar.current = cookieJar({ cr_session: 'crs_ok' });
  fetchMock.mockReset();
  vi.stubGlobal('fetch', fetchMock);
  vi.useRealTimers();
});

const ADMIN = 'a0000000-0000-4000-8000-000000000001';
const call = (qs = '') => GET(new Request(`http://localhost:3000/admin/activity/export${qs}`));

describe('GET /admin/activity/export', () => {
  it('streams the API CSV with the bearer and the filters, since as a time', async () => {
    vi.useFakeTimers({ now: new Date('2026-11-06T12:00:00Z'), toFake: ['Date'] });
    fetchMock.mockResolvedValue(
      new Response('at,admin_email\r\n', {
        status: 200,
        headers: { 'content-disposition': 'attachment; filename="activity-2026-11-06.csv"' },
      }),
    );
    const res = await call(`?admin=${ADMIN}&group=email&since=30d&cursor=IGNORED`);
    expect(res.status).toBe(200);
    expect(res.headers.get('content-type')).toContain('text/csv');
    expect(res.headers.get('x-content-type-options')).toBe('nosniff');
    expect(res.headers.get('cache-control')).toBe('no-store');
    expect(res.headers.get('content-disposition')).toBe(
      'attachment; filename="activity-2026-11-06.csv"',
    );
    const [url, init] = fetchMock.mock.calls[0];
    const u = new URL(String(url));
    expect(u.origin + u.pathname).toBe('http://api.test/v1/admin/activity.csv');
    expect(u.searchParams.get('admin')).toBe(ADMIN);
    expect(u.searchParams.get('action_group')).toBe('email');
    expect(u.searchParams.get('since')).toBe('2026-10-07T12:00:00.000Z');
    expect(u.searchParams.has('cursor')).toBe(false);
    expect(new Headers(init.headers).get('authorization')).toBe('Bearer crs_ok');
  });

  it('exports the default 7 days when asked for nothing, and all time with no bound', async () => {
    vi.useFakeTimers({ now: new Date('2026-11-06T12:00:00Z'), toFake: ['Date'] });
    fetchMock.mockResolvedValue(new Response('x', { status: 200 }));
    await call();
    expect(new URL(String(fetchMock.mock.calls[0][0])).searchParams.get('since')).toBe(
      '2026-10-30T12:00:00.000Z',
    );
    await call('?since=all&group=bogus&admin=nope');
    const u = new URL(String(fetchMock.mock.calls[1][0]));
    expect(u.search).toBe('');
  });

  it('sends a dead session to sign in and back', async () => {
    fetchMock.mockResolvedValue(new Response(null, { status: 401 }));
    await expect(call('?group=email')).rejects.toThrow(
      /NEXT_REDIRECT \/sign-in\/expired\?return_to=/,
    );
    jar.current = cookieJar();
    await expect(call()).rejects.toThrow(/NEXT_REDIRECT \/sign-in\?return_to=/);
  });

  it('is a plain 404 for a non-admin and a 503 when Cloud is down', async () => {
    fetchMock.mockResolvedValue(new Response(null, { status: 404 }));
    expect((await call()).status).toBe(404);
    fetchMock.mockRejectedValue(new Error('down'));
    expect((await call()).status).toBe(503);
  });
});
