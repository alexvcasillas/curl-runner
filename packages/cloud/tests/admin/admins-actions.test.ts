import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };
vi.mock('next/headers', () => ({
  cookies: async () => jar.current.store,
  headers: async () => new Headers(),
}));
const revalidatePath = vi.fn();
vi.mock('next/cache', () => ({ revalidatePath: (p: string, t?: string) => revalidatePath(p, t) }));
vi.mock('next/navigation', () => ({
  notFound: () => {
    throw new Error('NEXT_NOT_FOUND');
  },
  redirect: (url: string) => {
    throw new Error(`NEXT_REDIRECT ${url}`);
  },
}));
const { resetEnv } = await import('@/lib/env');

const fetchMock = vi.fn();
const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
const ME = (permissions: string[]) => json({ role: 'admin', permissions });
let perms: string[] = [];
function answer(res: Response) {
  fetchMock.mockImplementation(async (url: string) =>
    String(url).endsWith('/v1/admin/me') ? ME(perms) : res.clone(),
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
  revalidatePath.mockReset();
  vi.stubGlobal('fetch', fetchMock);
});

const actions = await import('@/app/admin/admins/actions');
const UID = 'a0000000-0000-4000-8000-000000000009';

beforeEach(() => {
  perms = ['admins'];
  answer(json({ email: 'rosa@example.com' }, 201));
});

describe('the guard', () => {
  it('needs the admins permission for add, update and remove', async () => {
    perms = ['users', 'settings'];
    expect(await actions.addAdmin('rosa@example.com', ['users'])).toMatchObject({
      ok: false,
      code: 'forbidden',
    });
    expect(await actions.updateAdmin(UID, ['users'])).toMatchObject({
      ok: false,
      code: 'forbidden',
    });
    expect(await actions.removeAdmin(UID)).toMatchObject({ ok: false, code: 'forbidden' });
    expect(calls()).toEqual([]);
  });
});

describe('addAdmin', () => {
  it('POSTs the email and permissions, and refreshes the list', async () => {
    expect(await actions.addAdmin('  Rosa@Example.com ', ['users', 'emails'])).toEqual({
      ok: true,
    });
    expect(calls()).toEqual([
      {
        url: '/v1/admin/admins',
        method: 'POST',
        body: { email: 'rosa@example.com', permissions: ['users', 'emails'] },
      },
    ]);
    expect(revalidatePath).toHaveBeenCalledWith('/admin/admins', undefined);
  });

  it('refuses a bad email or an unknown permission before calling', async () => {
    expect(await actions.addAdmin('nope', ['users'])).toMatchObject({
      ok: false,
      code: 'invalid_request',
    });
    expect(await actions.addAdmin('a@b.co', ['root' as never])).toMatchObject({
      ok: false,
      code: 'invalid_request',
    });
    expect(calls()).toEqual([]);
  });

  it('shows the API’s copy for an unknown account and for an owner', async () => {
    answer(
      json(
        {
          error: 'not_found',
          message: 'No account with that email yet. Ask them to sign in once.',
        },
        404,
      ),
    );
    expect(await actions.addAdmin('who@x.co', [])).toEqual({
      ok: false,
      code: 'not_found',
      message: 'No account with that email yet. Ask them to sign in once.',
    });
    answer(
      json(
        {
          error: 'conflict',
          message: 'Owners come from OPERATOR_EMAILS and can’t be changed here.',
        },
        409,
      ),
    );
    expect(await actions.addAdmin('ada@example.com', [])).toMatchObject({
      ok: false,
      code: 'conflict',
    });
  });
});

describe('updateAdmin and removeAdmin', () => {
  it('PATCHes the permissions', async () => {
    answer(json({}));
    await actions.updateAdmin(UID, ['teams']);
    expect(calls()[0]).toEqual({
      url: `/v1/admin/admins/${UID}`,
      method: 'PATCH',
      body: { permissions: ['teams'] },
    });
  });

  it('DELETEs, and a bad id never leaves the dashboard', async () => {
    answer(new Response(null, { status: 204 }));
    expect(await actions.removeAdmin(UID)).toEqual({ ok: true });
    expect(calls()[0]).toMatchObject({ url: `/v1/admin/admins/${UID}`, method: 'DELETE' });
    expect(await actions.removeAdmin('../x')).toMatchObject({ ok: false });
    expect(calls()).toHaveLength(1);
  });
});
