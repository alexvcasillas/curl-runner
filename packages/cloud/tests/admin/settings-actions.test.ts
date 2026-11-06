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

const actions = await import('@/app/admin/settings/actions');
const UID = 'a0000000-0000-4000-8000-000000000001';
void UID;

beforeEach(() => {
  perms = ['settings'];
  answer(json({ signup_mode: 'open' }));
});

describe('the guard', () => {
  it('refuses every write without the settings permission, and sends nothing', async () => {
    perms = ['users', 'admins'];
    for (const run of [
      () => actions.saveSignup('open', []),
      () =>
        actions.savePlans({
          defaultPlan: 'free',
          memberLimit: 3,
          restricted: false,
          graceDays: 14,
        }),
      () => actions.saveBanner({ message: 'Hi', tone: 'info', audience: 'everyone', link: '' }),
      () => actions.setPublishing(true),
      () => actions.sendTestEmail(),
    ]) {
      expect(await run()).toMatchObject({ ok: false, code: 'forbidden' });
    }
    expect(calls()).toEqual([]);
  });
});

describe('saveSignup', () => {
  it('PUTs the mode and cleaned domains, then refreshes the page and the layout', async () => {
    expect(
      await actions.saveSignup('domains', [' Acme.example ', 'acme.example', 'globex.example']),
    ).toEqual({
      ok: true,
    });
    expect(calls()).toEqual([
      {
        url: '/v1/admin/settings/signup',
        method: 'PUT',
        body: { mode: 'domains', domains: ['acme.example', 'globex.example'] },
      },
    ]);
    expect(revalidatePath).toHaveBeenCalledWith('/admin/settings', undefined);
    expect(revalidatePath).toHaveBeenCalledWith('/', 'layout');
  });

  it('refuses a mode it does not know', async () => {
    expect(await actions.saveSignup('wide-open' as never, [])).toMatchObject({
      ok: false,
      code: 'invalid_request',
    });
    expect(calls()).toEqual([]);
  });

  it('shows the API message when it says no', async () => {
    answer(json({ error: 'invalid_request', message: 'x.y is not a valid domain.' }, 400));
    expect(await actions.saveSignup('domains', ['x.example'])).toEqual({
      ok: false,
      code: 'invalid_request',
      message: 'x.y is not a valid domain.',
    });
  });
});

describe('guards', () => {
  it('refuses a javascript: banner link and a malformed domain', async () => {
    expect(
      await actions.saveBanner({
        message: 'a',
        tone: 'info',
        audience: 'everyone',
        link: 'javascript:alert(1)',
      }),
    ).toMatchObject({ ok: false, code: 'invalid_request' });
    expect(await actions.saveSignup('domains', ['not a domain'])).toMatchObject({
      ok: false,
      code: 'invalid_request',
    });
    expect(await actions.saveSignup('domains', ['nodot'])).toMatchObject({ ok: false });
    expect(calls()).toEqual([]);
  });
});

describe('savePlans', () => {
  it('maps the form to the API fields', async () => {
    await actions.savePlans({
      defaultPlan: 'team',
      memberLimit: 5,
      restricted: true,
      graceDays: 7,
    });
    expect(calls()[0]).toEqual({
      url: '/v1/admin/settings/plans',
      method: 'PUT',
      body: { default_plan: 'team', free_member_limit: 5, free_restricted: true, grace_days: 7 },
    });
  });

  it('refuses numbers outside the server’s range before calling it', async () => {
    for (const bad of [
      { defaultPlan: 'free', memberLimit: 0, restricted: false, graceDays: 14 },
      { defaultPlan: 'free', memberLimit: 101, restricted: false, graceDays: 14 },
      { defaultPlan: 'free', memberLimit: 3, restricted: false, graceDays: -1 },
      { defaultPlan: 'free', memberLimit: 3.5, restricted: false, graceDays: 14 },
      { defaultPlan: 'gold', memberLimit: 3, restricted: false, graceDays: 14 },
    ] as const) {
      expect(await actions.savePlans(bad as never)).toMatchObject({
        ok: false,
        code: 'invalid_request',
      });
    }
    expect(calls()).toEqual([]);
  });
});

describe('saveBanner', () => {
  it('trims, sends null for an empty message and an empty link', async () => {
    await actions.saveBanner({
      message: '  Maintenance at 5  ',
      tone: 'warning',
      audience: 'signed_in',
      link: ' ',
    });
    expect(calls()[0].body).toEqual({
      message: 'Maintenance at 5',
      tone: 'warning',
      audience: 'signed_in',
      link: null,
    });
    fetchMock.mockClear();
    await actions.saveBanner({
      message: '',
      tone: 'info',
      audience: 'everyone',
      link: 'https://a.com/x',
    });
    expect(calls()[0].body).toEqual({
      message: null,
      tone: 'info',
      audience: 'everyone',
      link: 'https://a.com/x',
    });
  });

  it('refuses a message over 280 characters and a tone it does not know', async () => {
    expect(
      await actions.saveBanner({
        message: 'x'.repeat(281),
        tone: 'info',
        audience: 'everyone',
        link: '',
      }),
    ).toMatchObject({ ok: false });
    expect(
      await actions.saveBanner({
        message: 'a',
        tone: 'loud' as never,
        audience: 'everyone',
        link: '',
      }),
    ).toMatchObject({ ok: false });
    expect(calls()).toEqual([]);
  });
});

describe('setPublishing and the test email', () => {
  it('PUTs paused', async () => {
    await actions.setPublishing(true);
    expect(calls()[0]).toEqual({
      url: '/v1/admin/settings/publishing',
      method: 'PUT',
      body: { paused: true },
    });
  });

  it('POSTs the test email and says where it went', async () => {
    answer(json({ to: 'ada@example.com' }, 202));
    expect(await actions.sendTestEmail()).toEqual({ ok: true, to: 'ada@example.com' });
    expect(calls()[0]).toMatchObject({ url: '/v1/admin/settings/test-email', method: 'POST' });
  });

  it('passes a rate limit through', async () => {
    answer(json({ error: 'rate_limited', message: 'Slow down.' }, 429));
    expect(await actions.sendTestEmail()).toMatchObject({ ok: false, code: 'rate_limited' });
  });
});
