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

import { draft, ID, preview } from './emails-fixtures';

const actions = await import('@/app/admin/emails/actions');
const { resetEnv } = await import('@/lib/env');

const fetchMock = vi.fn();
const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

let perms = ['emails'];
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
  perms = ['emails'];
  answer(json(draft));
  revalidatePath.mockReset();
  vi.stubGlobal('fetch', fetchMock);
});

const FUTURE = '2099-03-31T10:00:00.000Z';
const input = {
  subject: ' Hello ',
  body_md: 'Hi {{first_name}}',
  audience: 'team_owners' as const,
};

describe('the guard', () => {
  it('stops every write without the emails permission, before any parsing or request', async () => {
    perms = ['teams'];
    const bad = { not: 'a string' } as never;
    for (const r of [
      await actions.createBroadcast(bad),
      await actions.saveBroadcast(bad, bad),
      await actions.sendTest(bad),
      await actions.sendBroadcast(bad, bad),
    ]) {
      expect(r).toMatchObject({ ok: false, code: 'forbidden' });
    }
    expect(calls()).toHaveLength(0);
  });

  it('lets any admin read a preview and a count, but still 404s a non-admin', async () => {
    perms = [];
    answer(json(preview));
    expect(await actions.previewBroadcast(ID, '')).toMatchObject({ ok: true });
    answer(json({ audience: 'everyone', count: 3 }));
    expect(await actions.audienceCount('everyone')).toEqual({ ok: true, count: 3 });
    fetchMock.mockImplementation(async () => json({ error: 'not_found', message: 'x' }, 404));
    await expect(actions.previewBroadcast(ID, '')).rejects.toThrow('NEXT_NOT_FOUND');
  });
});

describe('createBroadcast', () => {
  it('posts a draft and gives it back', async () => {
    const r = await actions.createBroadcast({ audience: 'free_teams' });
    expect(r).toEqual({ ok: true, broadcast: draft });
    expect(calls()).toEqual([
      { url: '/v1/admin/broadcasts', method: 'POST', body: { audience: 'free_teams' } },
    ]);
    expect(revalidatePath).toHaveBeenCalledWith('/admin/emails');
  });

  it('refuses an audience the server does not know', async () => {
    expect(await actions.createBroadcast({ audience: 'one_team' as never })).toMatchObject({
      ok: false,
    });
    expect(calls()).toHaveLength(0);
  });
});

describe('saveBroadcast', () => {
  it('patches the draft with a trimmed subject', async () => {
    const r = await actions.saveBroadcast(ID, input);
    expect(r).toMatchObject({ ok: true });
    expect(calls()).toEqual([
      {
        url: `/v1/admin/broadcasts/${ID}`,
        method: 'PATCH',
        body: { subject: 'Hello', body_md: 'Hi {{first_name}}', audience: 'team_owners' },
      },
    ]);
  });

  it('checks the ids and limits first', async () => {
    expect(await actions.saveBroadcast('nope', input)).toMatchObject({ ok: false });
    expect(await actions.saveBroadcast(ID, { ...input, subject: 'a'.repeat(201) })).toMatchObject({
      ok: false,
      message: 'The subject can be at most 200 characters.',
    });
    expect(
      await actions.saveBroadcast(ID, { ...input, body_md: 'a'.repeat(20_001) }),
    ).toMatchObject({ ok: false, message: 'The message can be at most 20000 characters.' });
    expect(await actions.saveBroadcast(ID, { ...input, audience: 'list' as never })).toMatchObject({
      ok: false,
    });
    expect(calls()).toHaveLength(0);
  });

  it('turns the 409 of a sending broadcast into copy', async () => {
    answer(json({ error: 'conflict', message: 'This broadcast is already sending.' }, 409));
    const r = await actions.saveBroadcast(ID, input);
    expect(r).toMatchObject({ ok: false, code: 'conflict' });
    expect((r as { message: string }).message).toBe(
      'This broadcast is already going out, so it can’t change.',
    );
  });
});

describe('sendTest', () => {
  it('posts and says who got it', async () => {
    answer(json({ to: 'ada@example.com' }, 202));
    expect(await actions.sendTest(ID)).toEqual({ ok: true, to: 'ada@example.com' });
    expect(calls()).toEqual([
      { url: `/v1/admin/broadcasts/${ID}/test`, method: 'POST', body: undefined },
    ]);
  });
});

describe('sendBroadcast', () => {
  it('sends now with no time', async () => {
    answer(json({ ...draft, status: 'sending' }, 202));
    const r = await actions.sendBroadcast(ID, null);
    expect(r).toMatchObject({ ok: true, broadcast: { status: 'sending' } });
    expect(calls()).toEqual([{ url: `/v1/admin/broadcasts/${ID}/send`, method: 'POST', body: {} }]);
    expect(revalidatePath).toHaveBeenCalledWith('/admin/emails');
  });

  it('schedules with a future instant, as UTC', async () => {
    answer(json({ ...draft, status: 'scheduled', scheduled_at: FUTURE }, 202));
    await actions.sendBroadcast(ID, '2099-03-31T12:00:00+02:00');
    expect(calls()[0].body).toEqual({ at: FUTURE });
  });

  it('refuses a past or junk time without calling the API', async () => {
    expect(await actions.sendBroadcast(ID, '2020-01-01T00:00:00Z')).toMatchObject({
      ok: false,
      message: 'Pick a time in the future.',
    });
    expect(await actions.sendBroadcast(ID, 'soon')).toMatchObject({ ok: false });
    expect(calls()).toHaveLength(0);
  });

  it('says so on a second send (409)', async () => {
    answer(json({ error: 'conflict', message: 'This broadcast was already sent.' }, 409));
    const r = await actions.sendBroadcast(ID, null);
    expect((r as { message: string }).message).toBe(
      'This broadcast is already going out, so it can’t change.',
    );
  });
});

describe('previewBroadcast', () => {
  it('asks the server for the rendered copy, as a user', async () => {
    answer(json(preview));
    const r = await actions.previewBroadcast(ID, ' linus@acme.com ');
    expect(r).toEqual({ ok: true, preview });
    expect(calls()).toEqual([
      {
        url: `/v1/admin/broadcasts/${ID}/preview?as_user=linus%40acme.com`,
        method: 'GET',
        body: undefined,
      },
    ]);
  });

  it('previews as yourself when nobody is named', async () => {
    answer(json(preview));
    await actions.previewBroadcast(ID, '');
    expect(calls()[0].url).toBe(`/v1/admin/broadcasts/${ID}/preview`);
  });

  it('keeps the server’s message for an unknown user', async () => {
    answer(json({ error: 'not_found', message: 'No user with that id or email.' }, 404));
    const r = await actions.previewBroadcast(ID, 'nobody@x.dev');
    expect(r).toMatchObject({ ok: false, code: 'not_found' });
    expect((r as { message: string }).message).toBe('No user with that id or email.');
  });
});

describe('audienceCount', () => {
  it('reads the count for an audience', async () => {
    answer(json({ audience: 'paying_teams', count: 47 }));
    expect(await actions.audienceCount('paying_teams')).toEqual({ ok: true, count: 47 });
    expect(calls()[0].url).toBe('/v1/admin/audience-count?audience=paying_teams');
  });

  it('refuses audiences that are not built', async () => {
    expect(await actions.audienceCount('one_team' as never)).toMatchObject({ ok: false });
    expect(calls()).toHaveLength(0);
  });
});
