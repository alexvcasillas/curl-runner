import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';
import { ADA } from './teams-fixtures';

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

const actions = await import('@/app/admin/teams/actions');
const { resetEnv } = await import('@/lib/env');

const fetchMock = vi.fn();
const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
const empty = (status = 204) => new Response(null, { status });

let perms = ['teams', 'billing', 'emails'];
const ME = () => json({ role: 'admin', permissions: perms });

function answer(res: Response) {
  fetchMock.mockImplementation(async (url: string) =>
    String(url).endsWith('/v1/admin/me') ? ME() : res.clone(),
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

const FUTURE = '2099-03-31T23:59:59.000Z';

beforeEach(() => {
  vi.stubEnv('API_INTERNAL_URL', 'http://api.test');
  resetEnv();
  fetchMock.mockReset();
  perms = ['teams', 'billing', 'emails'];
  answer(empty());
  revalidatePath.mockReset();
  vi.stubGlobal('fetch', fetchMock);
});

describe('the guard', () => {
  it('stops every action without its permission, before any parsing or request', async () => {
    perms = [];
    const bad = { not: 'a string' } as never;
    const results = [
      await actions.grantOverride(bad, bad),
      await actions.removeOverride(bad),
      await actions.suspendTeam(bad, bad, bad),
      await actions.unsuspendTeam(bad),
      await actions.deleteTeamAndReturn(bad),
      await actions.setMemberRole(bad, bad, bad),
      await actions.stripeSync(bad),
      await actions.resendInvoice(bad),
      await actions.emailOwners(bad, bad, bad),
    ];
    for (const r of results) {
      expect(r).toMatchObject({ ok: false, code: 'forbidden' });
    }
    expect(calls()).toHaveLength(0);
  });

  it('keeps billing, teams and emails apart', async () => {
    perms = ['teams'];
    expect(
      await actions.grantOverride('acme', { plan: 'team', until: null, reason: '', notify: false }),
    ).toMatchObject({ code: 'forbidden' });
    expect(await actions.stripeSync('acme')).toMatchObject({ code: 'forbidden' });
    perms = ['billing'];
    expect(await actions.suspendTeam('acme', '', false)).toMatchObject({ code: 'forbidden' });
    expect(await actions.emailOwners('acme', 's', 'b')).toMatchObject({ code: 'forbidden' });
    expect(calls()).toHaveLength(0);
  });
});

describe('grantOverride', () => {
  it('puts the plan, end, trimmed reason and notify, and says whether Stripe still bills', async () => {
    answer(json({ plan: 'team', plan_source: 'override', override: null, stripe_conflict: true }));
    const r = await actions.grantOverride('acme', {
      plan: 'team',
      until: FUTURE,
      reason: '  Design partner ',
      notify: true,
    });
    expect(r).toEqual({ ok: true, stripeConflict: true });
    expect(calls()).toEqual([
      {
        url: '/v1/admin/teams/acme/override',
        method: 'PUT',
        body: { plan: 'team', until: FUTURE, reason: 'Design partner', notify: true },
      },
    ]);
    expect(revalidatePath).toHaveBeenCalledWith('/admin/teams');
    expect(revalidatePath).toHaveBeenCalledWith('/admin/teams/acme');
  });

  it('sends null for no end date and leaves an empty reason out as empty', async () => {
    answer(json({ stripe_conflict: false }));
    await actions.grantOverride('acme', { plan: 'free', until: null, reason: '', notify: false });
    expect(calls()[0].body).toEqual({ plan: 'free', until: null, reason: '', notify: false });
  });

  it('refuses a bad slug, plan, a past or unreadable end and a long reason before any request', async () => {
    const ok = { plan: 'team' as const, until: null, reason: '', notify: false };
    expect((await actions.grantOverride('Bad Slug', ok)).ok).toBe(false);
    expect((await actions.grantOverride('acme', { ...ok, plan: 'gold' as never })).ok).toBe(false);
    expect(
      await actions.grantOverride('acme', { ...ok, until: '2020-01-01T00:00:00.000Z' }),
    ).toMatchObject({ ok: false, message: 'The end date must be in the future.' });
    expect((await actions.grantOverride('acme', { ...ok, until: 'tomorrow' })).ok).toBe(false);
    expect(await actions.grantOverride('acme', { ...ok, reason: 'x'.repeat(501) })).toMatchObject({
      ok: false,
      message: 'The reason is too long.',
    });
    expect(calls()).toHaveLength(0);
  });

  it('passes the API refusal through', async () => {
    answer(json({ error: 'invalid_request', message: 'The end date must be in the future.' }, 400));
    const r = await actions.grantOverride('acme', {
      plan: 'team',
      until: FUTURE,
      reason: '',
      notify: false,
    });
    expect(r).toMatchObject({ ok: false, message: 'The end date must be in the future.' });
  });
});

describe('removeOverride', () => {
  it('deletes the override and refreshes', async () => {
    answer(json({ plan: 'free', plan_source: 'none', override: null, stripe_conflict: false }));
    expect(await actions.removeOverride('acme')).toEqual({ ok: true });
    expect(calls()).toEqual([
      { url: '/v1/admin/teams/acme/override', method: 'DELETE', body: undefined },
    ]);
  });

  it('shows the API 409 when there is none', async () => {
    answer(json({ error: 'conflict', message: 'This team has no plan override.' }, 409));
    expect(await actions.removeOverride('acme')).toMatchObject({
      ok: false,
      message: 'This team has no plan override.',
    });
  });
});

describe('suspension', () => {
  it('suspends with the trimmed reason and notify', async () => {
    answer(json({ slug: 'acme', status: 'suspended' }));
    expect(await actions.suspendTeam('acme', ' Phishing ', true)).toEqual({ ok: true });
    expect(calls()).toEqual([
      {
        url: '/v1/admin/teams/acme/suspend',
        method: 'POST',
        body: { reason: 'Phishing', notify: true },
      },
    ]);
  });

  it('rejects a long reason first', async () => {
    expect(await actions.suspendTeam('acme', 'x'.repeat(501), false)).toMatchObject({
      message: 'The reason is too long.',
    });
    expect(calls()).toHaveLength(0);
  });

  it('lifts it', async () => {
    answer(json({ slug: 'acme', status: 'active' }));
    expect(await actions.unsuspendTeam('acme')).toEqual({ ok: true });
    expect(calls()[0]).toMatchObject({ url: '/v1/admin/teams/acme/unsuspend', method: 'POST' });
  });
});

describe('deleteTeam', () => {
  it('deletes, refreshes the list and goes back to it', async () => {
    await expect(actions.deleteTeamAndReturn('acme')).rejects.toThrow('NEXT_REDIRECT /admin/teams');
    expect(calls()).toEqual([{ url: '/v1/admin/teams/acme', method: 'DELETE', body: undefined }]);
    expect(revalidatePath).toHaveBeenCalledWith('/admin/teams');
  });

  it('stays put on an API refusal', async () => {
    answer(json({ error: 'not_found', message: 'Not found.' }, 404));
    expect(await actions.deleteTeamAndReturn('acme')).toMatchObject({ ok: false });
  });
});

describe('setMemberRole', () => {
  it('patches the role', async () => {
    answer(json({ user_id: ADA, role: 'member' }));
    expect(await actions.setMemberRole('acme', ADA, 'member')).toEqual({ ok: true });
    expect(calls()).toEqual([
      { url: `/v1/admin/teams/acme/members/${ADA}`, method: 'PATCH', body: { role: 'member' } },
    ]);
  });

  it('refuses a bad user id or role', async () => {
    expect((await actions.setMemberRole('acme', 'nope', 'member')).ok).toBe(false);
    expect((await actions.setMemberRole('acme', ADA, 'boss' as never)).ok).toBe(false);
    expect(calls()).toHaveLength(0);
  });

  it('shows the last-owner 409', async () => {
    answer(json({ error: 'conflict', message: 'A team needs at least one owner.' }, 409));
    expect(await actions.setMemberRole('acme', ADA, 'member')).toMatchObject({
      ok: false,
      message: 'A team needs at least one owner.',
    });
  });
});

describe('Stripe', () => {
  it('syncs', async () => {
    answer(json({ plan: 'team', plan_source: 'stripe', stripe: null }));
    expect(await actions.stripeSync('acme')).toEqual({ ok: true });
    expect(calls()[0]).toMatchObject({ url: '/v1/admin/teams/acme/stripe-sync', method: 'POST' });
  });

  it('resends the invoice', async () => {
    answer(json({ invoice_id: 'in_1' }));
    expect(await actions.resendInvoice('acme')).toEqual({ ok: true });
    expect(calls()[0]).toMatchObject({
      url: '/v1/admin/teams/acme/resend-invoice',
      method: 'POST',
    });
  });

  it('says there is no invoice yet on a 409', async () => {
    answer(json({ error: 'conflict', message: 'Stripe has no invoice for this team yet.' }, 409));
    expect(await actions.resendInvoice('acme')).toMatchObject({
      ok: false,
      message: 'There’s no finished invoice to resend yet.',
    });
  });
});

describe('emailOwners', () => {
  it('posts the trimmed subject and body', async () => {
    answer(json({}, 202));
    expect(await actions.emailOwners('acme', ' Hello ', ' **Hi** ')).toEqual({ ok: true });
    expect(calls()).toEqual([
      {
        url: '/v1/admin/teams/acme/email-owners',
        method: 'POST',
        body: { subject: 'Hello', body_md: '**Hi**' },
      },
    ]);
  });

  it('checks the lengths first', async () => {
    expect((await actions.emailOwners('acme', '', 'b')).ok).toBe(false);
    expect((await actions.emailOwners('acme', 's', 'x'.repeat(10_001))).ok).toBe(false);
    expect(calls()).toHaveLength(0);
  });
});
