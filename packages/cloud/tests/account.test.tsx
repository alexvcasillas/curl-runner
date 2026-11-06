import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from './cookies';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };

vi.mock('next/headers', () => ({
  cookies: async () => jar.current.store,
  headers: async () => new Headers(),
}));
vi.mock('next/navigation', () => ({
  redirect: (url: string) => {
    throw new Error(`NEXT_REDIRECT ${url}`);
  },
  notFound: () => {
    throw new Error('NEXT_NOT_FOUND');
  },
}));
const revalidatePath = vi.fn();
vi.mock('next/cache', () => ({ revalidatePath: (...a: unknown[]) => revalidatePath(...a) }));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { AccountView } = await import('@/components/AccountView');
const { ToastProvider } = await import('@/components/Toast');
const actions = await import('@/app/(app)/account/actions');
const { resetEnv } = await import('@/lib/env');

const CURRENT = '3f2b8a52-8f2d-4a3e-9f53-0e2d2f5a7c11';
const OTHER = '9a1b8a52-8f2d-4a3e-9f53-0e2d2f5a7c22';

const sessions = [
  {
    id: CURRENT,
    label: 'Chrome on macOS',
    kind: 'web',
    meta: 'Active now · expires Feb 3, 2027',
    current: true,
  },
  {
    id: OTHER,
    label: 'CLI on ada-mbp',
    kind: 'device',
    meta: 'Last used 2 hours ago · expires Feb 3, 2027',
    current: false,
  },
];

function setup(over: Partial<React.ComponentProps<typeof AccountView>> = {}) {
  const onRevoke = vi.fn(async (_id: string) => ({ ok: true as const }));
  const onLeave = vi.fn(async (_team: string) => ({ ok: true as const }));
  const user = userEvent.setup();
  render(
    <ToastProvider>
      <AccountView
        email="ada@acme.example"
        name="Ada Lovelace"
        providers={[
          { provider: 'email', detail: 'ada@acme.example', linked: true },
          { provider: 'github', detail: 'Linked Oct 2 with ada@acme.example', linked: true },
          { provider: 'google', detail: 'Not linked.', linked: false },
        ]}
        sessions={sessions}
        teams={[
          { slug: 'acme', name: 'Acme', role: 'owner', plan: 'team', suspended: false },
          {
            slug: 'globex',
            name: 'Globex Partners',
            role: 'member',
            plan: 'free',
            suspended: false,
          },
        ]}
        showPlan
        onRevoke={onRevoke}
        onLeave={onLeave}
        {...over}
      />
    </ToastProvider>,
  );
  return { user, onRevoke, onLeave };
}

describe('AccountView', () => {
  it('shows who you are', () => {
    setup();
    expect(screen.getByRole('heading', { name: 'Ada Lovelace' })).toBeInTheDocument();
    expect(screen.getAllByText('ada@acme.example').length).toBeGreaterThan(0);
  });

  it('lists the email, linked providers and the unlinked one', () => {
    setup();
    const section = screen.getByRole('region', { name: 'Sign-in methods' });
    expect(within(section).getByText('Email code')).toBeInTheDocument();
    expect(within(section).getByText('Always on')).toBeInTheDocument();
    expect(within(section).getByText('GitHub')).toBeInTheDocument();
    expect(within(section).getByText('Linked')).toBeInTheDocument();
    expect(within(section).getByText('Google')).toBeInTheDocument();
    expect(within(section).getByText('Not linked.')).toBeInTheDocument();
  });

  it('marks this browser and keeps its button as Sign out', () => {
    setup();
    const devices = screen.getByRole('region', { name: 'Devices' });
    expect(within(devices).getByText('This browser')).toBeInTheDocument();
    expect(within(devices).getByRole('button', { name: 'Sign out Chrome on macOS' })).toBeVisible();
    expect(within(devices).getByRole('button', { name: 'Revoke CLI on ada-mbp' })).toBeVisible();
  });

  it('confirms before it revokes another device, then says so', async () => {
    const { user, onRevoke } = setup();
    await user.click(screen.getByRole('button', { name: 'Revoke CLI on ada-mbp' }));
    const dialog = await screen.findByRole('alertdialog', { name: 'Sign out CLI on ada-mbp?' });
    expect(dialog).toHaveTextContent('Its next request fails.');
    expect(onRevoke).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole('button', { name: 'Sign out device' }));
    expect(onRevoke).toHaveBeenCalledWith(OTHER);
    expect(await screen.findByText('CLI on ada-mbp is signed out.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Revoke CLI on ada-mbp' })).toBeNull();
  });

  it('shows the API message when a revoke fails and keeps the device', async () => {
    const { user } = setup({
      onRevoke: vi.fn(async () => ({ ok: false as const, message: 'That session is gone.' })),
    });
    await user.click(screen.getByRole('button', { name: 'Revoke CLI on ada-mbp' }));
    const dialog = await screen.findByRole('alertdialog');
    await user.click(within(dialog).getByRole('button', { name: 'Sign out device' }));
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('That session is gone.');
    expect(screen.getByRole('button', { name: 'Revoke CLI on ada-mbp' })).toBeInTheDocument();
  });

  it('signs out this browser through the revoke action', async () => {
    const { user, onRevoke } = setup();
    await user.click(screen.getByRole('button', { name: 'Sign out Chrome on macOS' }));
    const dialog = await screen.findByRole('alertdialog', { name: 'Sign out of this browser?' });
    await user.click(within(dialog).getByRole('button', { name: 'Sign out' }));
    expect(onRevoke).toHaveBeenCalledWith(CURRENT);
  });

  it('has a Sign out form that posts to /sign-out', () => {
    setup();
    const form = screen.getByRole('button', { name: 'Sign out' }).closest('form');
    expect(form).toHaveAttribute('action', '/sign-out');
    expect(form).toHaveAttribute('method', 'post');
  });

  it('lists teams with role and plan, and Leave only for members', async () => {
    const { user, onLeave } = setup();
    const teams = screen.getByRole('region', { name: 'Teams' });
    expect(within(teams).getByText('Owner · Team plan')).toBeInTheDocument();
    expect(within(teams).getByText('Member · Free plan')).toBeInTheDocument();
    expect(within(teams).getAllByRole('button', { name: /Leave/ })).toHaveLength(1);
    await user.click(within(teams).getByRole('button', { name: 'Leave Globex Partners' }));
    const dialog = await screen.findByRole('alertdialog', { name: 'Leave Globex Partners?' });
    await user.click(within(dialog).getByRole('button', { name: 'Leave team' }));
    expect(onLeave).toHaveBeenCalledWith('globex');
  });

  it('drops the plan when billing is off', () => {
    setup({ showPlan: false });
    expect(screen.getByText('Owner')).toBeInTheDocument();
    expect(screen.queryByText(/plan/)).toBeNull();
  });
});

describe('revokeSession', () => {
  const fetchMock = vi.fn();
  const json = (status: number, body: unknown) =>
    new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
  const list = [
    { id: CURRENT, kind: 'web', label: 'x', current: true },
    { id: OTHER, kind: 'device', label: 'y', current: false },
  ];
  beforeEach(() => {
    fetchMock.mockReset();
    revalidatePath.mockReset();
    jar.current = cookieJar({ cr_session: 'crs_ok' });
    vi.stubGlobal('fetch', fetchMock);
    vi.stubEnv('API_INTERNAL_URL', 'http://api.test');
    vi.stubEnv('PUBLIC_APP_URL', 'http://localhost:3000');
    resetEnv();
  });

  it('revoking another session deletes it and stays', async () => {
    fetchMock
      .mockResolvedValueOnce(json(200, list))
      .mockResolvedValueOnce(new Response(null, { status: 204 }));
    expect(await actions.revokeSession(OTHER)).toEqual({ ok: true });
    expect(fetchMock.mock.calls[1][0]).toBe(`http://api.test/v1/sessions/${OTHER}`);
    expect(fetchMock.mock.calls[1][1].method).toBe('DELETE');
    expect(jar.current.values.has('cr_session')).toBe(true);
    expect(revalidatePath).toHaveBeenCalledWith('/account');
  });

  it('revoking the current session clears the cookie and goes to sign-in', async () => {
    fetchMock
      .mockResolvedValueOnce(json(200, list))
      .mockResolvedValueOnce(new Response(null, { status: 204 }));
    await expect(actions.revokeSession(CURRENT)).rejects.toThrow('NEXT_REDIRECT /sign-in');
    expect(fetchMock.mock.calls[1][0]).toBe(`http://api.test/v1/sessions/${CURRENT}`);
    expect(jar.current.values.has('cr_session')).toBe(false);
  });

  it('refuses an id that is not a uuid without calling the API', async () => {
    expect((await actions.revokeSession('../teams')).ok).toBe(false);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('keeps the cookie and returns the message when the API refuses', async () => {
    fetchMock
      .mockResolvedValueOnce(json(200, list))
      .mockResolvedValueOnce(
        json(404, { error: { code: 'not_found', message: 'No such session.' } }),
      );
    const result = await actions.revokeSession(CURRENT);
    expect(result.ok).toBe(false);
    expect(jar.current.values.has('cr_session')).toBe(true);
  });
});
