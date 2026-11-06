import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };
vi.mock('next/headers', () => ({
  cookies: async () => jar.current.store,
  headers: async () => new Headers(),
}));
const revalidatePath = vi.fn();
vi.mock('next/cache', () => ({ revalidatePath: (p: string) => revalidatePath(p) }));
vi.mock('next/navigation', () => ({
  notFound: () => {
    throw new Error('NEXT_NOT_FOUND');
  },
  redirect: (url: string) => {
    throw new Error(`NEXT_REDIRECT ${url}`);
  },
}));

const actions = await import('@/app/admin/collections/actions');
const { resetEnv } = await import('@/lib/env');

const fetchMock = vi.fn();
const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

let perms = ['teams'];
function answer(res: Response) {
  fetchMock.mockImplementation(async (url: string) =>
    String(url).endsWith('/v1/admin/me')
      ? json({ role: 'admin', permissions: perms })
      : res.clone(),
  );
}
const calls = () =>
  fetchMock.mock.calls
    .filter(([url]) => !String(url).endsWith('/v1/admin/me'))
    .map(([url, init]) => ({
      url: String(url).replace('http://api.test', ''),
      method: init?.method ?? 'GET',
      body: init?.body ? JSON.parse(init.body) : undefined,
    }));

beforeEach(() => {
  vi.stubEnv('API_INTERNAL_URL', 'http://api.test');
  resetEnv();
  fetchMock.mockReset();
  perms = ['teams'];
  answer(json({ team: 'acme', slug: 'payments-api', link_disabled: true }));
  revalidatePath.mockReset();
  vi.stubGlobal('fetch', fetchMock);
});

describe('the guard', () => {
  it('stops both actions without the teams permission, before any request', async () => {
    perms = ['emails', 'users'];
    const bad = { not: 'a string' } as never;
    expect(await actions.disableLink(bad, bad, bad)).toMatchObject({
      ok: false,
      code: 'forbidden',
    });
    expect(await actions.enableLink(bad, bad)).toMatchObject({ ok: false, code: 'forbidden' });
    expect(calls()).toHaveLength(0);
  });
});

describe('disableLink', () => {
  it('posts the trimmed reason and refreshes the list', async () => {
    const r = await actions.disableLink('acme', 'payments-api', '  Phishing pages ');
    expect(r).toEqual({ ok: true });
    expect(calls()).toEqual([
      {
        url: '/v1/admin/collections/acme/payments-api/disable-link',
        method: 'POST',
        body: { reason: 'Phishing pages' },
      },
    ]);
    expect(revalidatePath).toHaveBeenCalledWith('/admin/collections');
  });

  it('asks for a reason, and refuses a long one, without calling the API', async () => {
    expect(await actions.disableLink('acme', 'x', '   ')).toMatchObject({
      ok: false,
      message: 'Add a reason. The team’s owners will read it.',
    });
    expect(await actions.disableLink('acme', 'x', 'a'.repeat(501))).toMatchObject({
      ok: false,
      message: 'The reason is too long.',
    });
    expect(calls()).toHaveLength(0);
  });

  it('refuses names that are not slugs', async () => {
    expect(await actions.disableLink('../etc', 'x', 'why')).toMatchObject({ ok: false });
    expect(await actions.disableLink('acme', 'A B', 'why')).toMatchObject({ ok: false });
    expect(calls()).toHaveLength(0);
  });

  it('says so when it was already disabled (409)', async () => {
    answer(json({ error: 'conflict', message: 'That share link is already disabled.' }, 409));
    const r = await actions.disableLink('acme', 'payments-api', 'why');
    expect(r).toMatchObject({ ok: false, code: 'conflict' });
    expect((r as { message: string }).message).toBe('That link is already disabled.');
  });
});

describe('enableLink', () => {
  it('posts and refreshes', async () => {
    answer(json({ team: 'acme', slug: 'payments-api', link_disabled: false }));
    expect(await actions.enableLink('acme', 'payments-api')).toEqual({ ok: true });
    expect(calls()).toEqual([
      {
        url: '/v1/admin/collections/acme/payments-api/enable-link',
        method: 'POST',
        body: undefined,
      },
    ]);
    expect(revalidatePath).toHaveBeenCalledWith('/admin/collections');
  });

  it('says so when it is not disabled (409)', async () => {
    answer(json({ error: 'conflict', message: 'x' }, 409));
    const r = await actions.enableLink('acme', 'payments-api');
    expect((r as { message: string }).message).toBe('That link isn’t disabled.');
  });
});
