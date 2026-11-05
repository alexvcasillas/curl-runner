import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';

const jar = { current: cookieJar({ cr_session: 'crs_token' }) };
const revalidatePath = vi.fn();

vi.mock('next/headers', () => ({
  cookies: async () => jar.current.store,
  headers: async () => new Headers(),
}));
vi.mock('next/cache', () => ({ revalidatePath }));
vi.mock('next/navigation', () => ({
  redirect: (url: string) => {
    throw new Error(`NEXT_REDIRECT ${url}`);
  },
}));

const actions = await import('@/app/(app)/t/[team]/c/[slug]/actions');
const { resetEnv } = await import('@/lib/env');

describe('collection actions', () => {
  const fetchMock = vi.fn();
  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
    vi.stubEnv('API_INTERNAL_URL', 'http://api.test');
    vi.stubEnv('PUBLIC_APP_URL', 'http://localhost:3000');
    resetEnv();
  });
  const json = (status: number, body: unknown) =>
    new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

  it('switches the source with PATCH', async () => {
    fetchMock.mockResolvedValue(json(200, {}));
    expect(await actions.switchSource('acme', 'payments', 'git')).toEqual({ ok: true });
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('http://api.test/v1/teams/acme/collections/payments');
    expect(init.method).toBe('PATCH');
    expect(JSON.parse(init.body)).toEqual({ source: 'git' });
  });

  it('deletes with DELETE, then goes to the list', async () => {
    fetchMock.mockResolvedValue(new Response(null, { status: 204 }));
    await expect(actions.deleteCollection('acme', 'payments')).rejects.toThrow(
      'NEXT_REDIRECT /t/acme',
    );
    expect(fetchMock.mock.calls[0][1].method).toBe('DELETE');
  });

  it('returns the API message and code on a failure', async () => {
    fetchMock.mockResolvedValue(json(403, { error: 'forbidden', message: 'Owners only.' }));
    expect(await actions.deleteCollection('acme', 'payments')).toEqual({
      ok: false,
      message: 'Owners only.',
      code: 'forbidden',
    });
  });

  it('rejects bad params without calling the API', async () => {
    const results = await Promise.all([
      actions.switchSource('..', 'payments', 'git'),
      actions.switchSource('acme', '..', 'git'),
      actions.switchSource('acme', 'Bad_Slug', 'git'),
      actions.switchSource('acme', 'payments', 'svn' as never),
      actions.deleteCollection('..', 'payments'),
      actions.deleteCollection('acme', '..'),
      actions.deleteCollection('acme', 'Bad_Slug'),
    ]);
    for (const r of results) {
      expect(r.ok).toBe(false);
    }
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
