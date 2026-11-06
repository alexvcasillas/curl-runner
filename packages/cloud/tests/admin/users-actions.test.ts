import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';
import { ADA, SESSION_A } from './users-fixtures';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };
vi.mock('next/headers', () => ({
  cookies: async () => jar.current.store,
  headers: async () => new Headers(),
}));
const revalidatePath = vi.fn();
vi.mock('next/cache', () => ({ revalidatePath: (p: string) => revalidatePath(p) }));
vi.mock('next/navigation', () => ({
  redirect: (url: string) => {
    throw new Error(`NEXT_REDIRECT ${url}`);
  },
}));

const actions = await import('@/app/admin/users/actions');
const { resetEnv } = await import('@/lib/env');

const fetchMock = vi.fn();
const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
const empty = (status = 204) => new Response(null, { status });

const calls = () =>
  fetchMock.mock.calls.map(([url, init]) => ({
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

describe('suspendUser', () => {
  it('posts the trimmed reason and notify, then refreshes the pages', async () => {
    fetchMock.mockResolvedValue(json({ id: ADA, status: 'suspended' }));
    const r = await actions.suspendUser(ADA, '  Chargeback  ', true);
    expect(r).toEqual({ ok: true });
    expect(calls()).toEqual([
      {
        url: `/v1/admin/users/${ADA}/suspend`,
        method: 'POST',
        body: { reason: 'Chargeback', notify: true },
      },
    ]);
    expect(revalidatePath).toHaveBeenCalledWith('/admin/users');
    expect(revalidatePath).toHaveBeenCalledWith(`/admin/users/${ADA}`);
  });

  it('refuses a bad id or an over-long reason before any request', async () => {
    expect((await actions.suspendUser('nope', '', false)).ok).toBe(false);
    const long = await actions.suspendUser(ADA, 'x'.repeat(501), false);
    expect(long).toMatchObject({ ok: false, message: 'The reason is too long.' });
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('shows the API message for an owner or yourself', async () => {
    fetchMock.mockResolvedValue(
      json(
        {
          error: 'conflict',
          message: 'Owners come from OPERATOR_EMAILS and can’t be suspended or deleted here.',
        },
        409,
      ),
    );
    expect(await actions.suspendUser(ADA, '', false)).toEqual({
      ok: false,
      message: 'Owners come from OPERATOR_EMAILS and can’t be suspended or deleted here.',
      code: 'conflict',
    });
  });

  it('shows the API message for a missing permission', async () => {
    fetchMock.mockResolvedValue(
      json({ error: 'forbidden', message: 'You don’t have permission to do that.' }, 403),
    );
    expect(await actions.suspendUser(ADA, '', false)).toMatchObject({
      ok: false,
      message: 'You don’t have permission to do that.',
      code: 'forbidden',
    });
  });
});

describe('simple actions', () => {
  it('lifts a suspension', async () => {
    fetchMock.mockResolvedValue(json({ id: ADA, status: 'active' }));
    expect(await actions.unsuspendUser(ADA)).toEqual({ ok: true });
    expect(calls()[0]).toMatchObject({ url: `/v1/admin/users/${ADA}/unsuspend`, method: 'POST' });
  });

  it('signs a user out everywhere', async () => {
    fetchMock.mockResolvedValue(empty());
    expect(await actions.signOutEverywhere(ADA)).toEqual({ ok: true });
    expect(calls()[0]).toMatchObject({
      url: `/v1/admin/users/${ADA}/sign-out-everywhere`,
      method: 'POST',
    });
  });

  it('revokes one device', async () => {
    fetchMock.mockResolvedValue(empty());
    expect(await actions.revokeSession(ADA, SESSION_A)).toEqual({ ok: true });
    expect(calls()[0]).toMatchObject({
      url: `/v1/admin/users/${ADA}/sessions/${SESSION_A}`,
      method: 'DELETE',
    });
    expect((await actions.revokeSession(ADA, '../x')).ok).toBe(false);
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('unlinks a provider, never email, never a path trick', async () => {
    fetchMock.mockResolvedValue(empty());
    expect(await actions.unlinkIdentity(ADA, 'github')).toEqual({ ok: true });
    expect(calls()[0]).toMatchObject({
      url: `/v1/admin/users/${ADA}/identities/github`,
      method: 'DELETE',
    });
    expect((await actions.unlinkIdentity(ADA, 'email')).ok).toBe(false);
    expect((await actions.unlinkIdentity(ADA, 'github/../x')).ok).toBe(false);
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });
});

describe('deleteUser', () => {
  it('goes back to the list once the user is gone', async () => {
    fetchMock.mockResolvedValue(empty());
    await expect(actions.deleteUser(ADA, { redirectTo: 'list' })).rejects.toThrow(
      'NEXT_REDIRECT /admin/users',
    );
    expect(calls()[0]).toMatchObject({ url: `/v1/admin/users/${ADA}`, method: 'DELETE' });
  });

  it('stays put and refreshes when the list deletes', async () => {
    fetchMock.mockResolvedValue(empty());
    expect(await actions.deleteUser(ADA)).toEqual({ ok: true });
    expect(revalidatePath).toHaveBeenCalledWith('/admin/users');
  });

  it('returns the sole-owner copy from the API', async () => {
    const message =
      'Ada is the only owner of Acme. Make someone else an owner, or delete Acme first.';
    fetchMock.mockResolvedValue(json({ error: 'conflict', message }, 409));
    expect(await actions.deleteUser(ADA, { redirectTo: 'list' })).toEqual({
      ok: false,
      message,
      code: 'conflict',
    });
  });
});

describe('emailUser', () => {
  it('posts a trimmed subject and the Markdown body', async () => {
    fetchMock.mockResolvedValue(json({}, 202));
    expect(await actions.emailUser(ADA, ' Hello ', ' **Hi** there ')).toEqual({ ok: true });
    expect(calls()[0]).toEqual({
      url: `/v1/admin/users/${ADA}/email`,
      method: 'POST',
      body: { subject: 'Hello', body_md: '**Hi** there' },
    });
  });

  it('checks the lengths the server checks', async () => {
    expect(await actions.emailUser(ADA, '', 'x')).toMatchObject({
      ok: false,
      message: 'The subject must be 1 to 200 characters.',
    });
    expect(await actions.emailUser(ADA, 'x'.repeat(201), 'x')).toMatchObject({ ok: false });
    expect(await actions.emailUser(ADA, 'x', '')).toMatchObject({
      ok: false,
      message: 'The message must be 1 to 10000 characters.',
    });
    expect(await actions.emailUser(ADA, 'x', 'y'.repeat(10001))).toMatchObject({ ok: false });
    expect(fetchMock).not.toHaveBeenCalled();
  });
});

describe('bulkUsers', () => {
  const ids = [ADA, 'a0000000-0000-4000-8000-000000000003'];

  it('signs out and reports done and skipped', async () => {
    fetchMock.mockResolvedValue(json({ done: 1, skipped: [ids[1]] }));
    expect(await actions.bulkUsers(ids, 'sign_out')).toEqual({
      ok: true,
      done: 1,
      skipped: [ids[1]],
    });
    expect(calls()[0]).toEqual({
      url: '/v1/admin/users/bulk',
      method: 'POST',
      body: { ids, action: 'sign_out' },
    });
  });

  it('suspends with a trimmed reason', async () => {
    fetchMock.mockResolvedValue(json({ done: 2, skipped: [] }));
    await actions.bulkUsers(ids, 'suspend', ' spam ');
    expect(calls()[0].body).toEqual({ ids, action: 'suspend', reason: 'spam' });
  });

  it('refuses no ids, more than 200, a bad id, an unknown action or a long reason', async () => {
    const many = Array.from(
      { length: 201 },
      (_, i) => `a0000000-0000-4000-8000-${String(i).padStart(12, '0')}`,
    );
    expect((await actions.bulkUsers([], 'sign_out')).ok).toBe(false);
    expect((await actions.bulkUsers(many, 'sign_out')).ok).toBe(false);
    expect((await actions.bulkUsers(['x'], 'sign_out')).ok).toBe(false);
    expect((await actions.bulkUsers(ids, 'delete' as never)).ok).toBe(false);
    expect((await actions.bulkUsers(ids, 'suspend', 'x'.repeat(501))).ok).toBe(false);
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
