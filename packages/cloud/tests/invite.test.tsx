import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from './cookies';

const jar = { current: cookieJar() };
const incoming = { current: new Headers({ 'x-pathname': '/invites/x' }) };

vi.mock('next/headers', () => ({
  cookies: async () => jar.current.store,
  headers: async () => incoming.current,
}));
class RedirectSignal extends Error {
  constructor(public url: string) {
    super(`NEXT_REDIRECT ${url}`);
  }
}
vi.mock('next/navigation', () => ({
  redirect: (url: string) => {
    throw new RedirectSignal(url);
  },
  notFound: () => {
    throw new Error('NEXT_NOT_FOUND');
  },
}));
vi.mock('next/cache', () => ({ revalidatePath: vi.fn() }));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const Invite = await import('@/app/invites/[token]/page');
const InviteNotFound = (await import('@/app/invites/[token]/not-found')).default;
const actions = await import('@/app/invites/[token]/actions');
const { resetEnv } = await import('@/lib/env');

const TOKEN = `cri_${'B'.repeat(43)}`;
const json = (status: number, body: unknown) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
const preview = {
  team_name: 'Acme',
  team_slug: 'acme',
  inviter_name: 'Grace Hopper',
  role: 'member',
  email: 'ada@acme.com',
  expires_at: new Date(Date.now() + 5 * 86_400_000 + 3_600_000).toISOString(),
};
const me = (email: string) =>
  json(200, { user: { id: 'u1', email, name: null }, teams: [], admin: false });

const fetchMock = vi.fn();
function routes(map: Record<string, () => Response>) {
  fetchMock.mockImplementation(async (url: string, init?: { method?: string }) => {
    const u = new URL(url);
    const make = map[`${init?.method ?? 'GET'} ${u.pathname}`];
    if (!make) {
      throw new Error(`unexpected ${init?.method ?? 'GET'} ${u.pathname}`);
    }
    return make();
  });
}
const PREVIEW = `GET /v1/invites/${TOKEN}`;
const ACCEPT = `POST /v1/invites/${TOKEN}/accept`;
const params = (token = TOKEN) => ({ params: Promise.resolve({ token }) });

beforeEach(() => {
  resetEnv();
  vi.stubEnv('API_INTERNAL_URL', 'http://api.internal:8080');
  vi.stubEnv('PUBLIC_API_URL', 'https://api.example.com');
  vi.stubEnv('PUBLIC_APP_URL', 'https://cloud.example.com');
  vi.stubGlobal('fetch', fetchMock);
  fetchMock.mockReset();
  jar.current = cookieJar();
});

describe('/invites/[token], signed out', () => {
  it('shows the team, the inviter, the role and the email, and asks to sign in', async () => {
    routes({ [PREVIEW]: () => json(200, preview) });
    render(await Invite.default(params()));
    expect(
      screen.getByRole('heading', { level: 1, name: 'Join Acme on curl-runner Cloud' }),
    ).toBeInTheDocument();
    expect(screen.getByText('Grace Hopper')).toBeInTheDocument();
    expect(screen.getAllByText('ada@acme.com').length).toBeGreaterThan(0);
    expect(
      screen.getByText(/Members publish, share collections and make tokens/),
    ).toBeInTheDocument();
    expect(screen.getByText('Expires in 5 days')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Sign in to join' })).toHaveAttribute(
      'href',
      `/sign-in?return_to=${encodeURIComponent(`/invites/${TOKEN}`)}`,
    );
    expect(
      screen.getByText(/Sign in with ada@acme.com, or any account you use/),
    ).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Join Acme/ })).toBeNull();
  });

  it('loads the preview without a session', async () => {
    jar.current = cookieJar({ cr_session: 'crs_x' });
    routes({ [PREVIEW]: () => json(200, preview), 'GET /v1/me': () => me('ada@acme.com') });
    await Invite.default(params());
    const call = fetchMock.mock.calls.find(([u]) => String(u).includes('/v1/invites/'));
    expect(call?.[1].headers.get('authorization')).toBeNull();
  });
});

describe('/invites/[token], signed in', () => {
  beforeEach(() => {
    jar.current = cookieJar({ cr_session: 'crs_ok' });
  });

  it('offers Join, shows who is signed in and a way to switch', async () => {
    routes({ [PREVIEW]: () => json(200, preview), 'GET /v1/me': () => me('ada@acme.com') });
    render(await Invite.default(params()));
    expect(screen.getByRole('button', { name: 'Join Acme' })).toBeEnabled();
    expect(screen.getByText('Signed in as ada@acme.com')).toBeInTheDocument();
    const form = screen.getByRole('button', { name: 'Use another account' }).closest('form');
    expect(form).toHaveAttribute(
      'action',
      `/sign-out?return_to=${encodeURIComponent(`/invites/${TOKEN}`)}`,
    );
    expect(screen.queryByText(/Joining adds/)).toBeNull();
  });

  it('warns when the signed-in email is not the invited one', async () => {
    routes({ [PREVIEW]: () => json(200, preview), 'GET /v1/me': () => me('ada@gmail.com') });
    render(await Invite.default(params()));
    expect(
      screen.getByText(
        /This invite was sent to ada@acme.com, and you’re signed in as ada@gmail.com/,
      ),
    ).toBeInTheDocument();
    expect(screen.getByText(/Joining adds ada@gmail.com to Acme/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Join Acme' })).toBeEnabled();
  });

  it('does not warn when the emails differ only by case', async () => {
    routes({ [PREVIEW]: () => json(200, preview), 'GET /v1/me': () => me('Ada@Acme.com') });
    render(await Invite.default(params()));
    expect(screen.queryByText(/Joining adds/)).toBeNull();
  });

  it('shows Joining… while the accept runs, then lands on the team', async () => {
    const user = userEvent.setup();
    routes({ [PREVIEW]: () => json(200, preview), 'GET /v1/me': () => me('ada@acme.com') });
    const onJoin = vi.fn(() => new Promise<never>(() => {}));
    const { InviteCard } = await import('@/components/InviteCard');
    render(
      <InviteCard
        teamName="Acme"
        inviter="Grace Hopper"
        joinAs="member"
        email="ada@acme.com"
        expires="Expires in 5 days"
        viewer={{ email: 'ada@acme.com' }}
        returnTo={`/invites/${TOKEN}`}
        onJoin={onJoin}
      />,
    );
    await user.click(screen.getByRole('button', { name: 'Join Acme' }));
    expect(onJoin).toHaveBeenCalledOnce();
    expect(screen.getByRole('button', { name: 'Joining Acme…' })).toBeDisabled();
  });

  it('shows the plan limit copy and disables Join on a 402', async () => {
    const user = userEvent.setup();
    const { InviteCard } = await import('@/components/InviteCard');
    render(
      <InviteCard
        teamName="Acme"
        inviter="Grace Hopper"
        joinAs="member"
        email="ada@acme.com"
        expires="Expires in 5 days"
        viewer={{ email: 'ada@acme.com' }}
        returnTo={`/invites/${TOKEN}`}
        onJoin={async () => ({ ok: false, code: 'plan_limit', message: 'x' })}
      />,
    );
    await user.click(screen.getByRole('button', { name: 'Join Acme' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Acme is on the Free plan, which allows 3 members, and it’s full. Ask Grace Hopper to upgrade Acme or remove someone, then open this link again.',
    );
    expect(screen.getByRole('button', { name: 'Join Acme' })).toBeDisabled();
  });

  it('shows another failure’s message and keeps Join available', async () => {
    const user = userEvent.setup();
    const { InviteCard } = await import('@/components/InviteCard');
    render(
      <InviteCard
        teamName="Acme"
        inviter={null}
        joinAs="owner"
        email="ada@acme.com"
        expires="Expires soon"
        viewer={{ email: 'ada@acme.com' }}
        returnTo="/invites/x"
        onJoin={async () => ({ ok: false, code: 'unavailable', message: 'Cloud isn’t answering.' })}
      />,
    );
    await user.click(screen.getByRole('button', { name: 'Join Acme' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Cloud isn’t answering.');
    expect(screen.getByRole('button', { name: 'Join Acme' })).toBeEnabled();
  });
});

describe('/invites/[token], dead and malformed', () => {
  it('answers 404 for an expired or used invite', async () => {
    routes({ [PREVIEW]: () => json(404, { error: 'not_found', message: 'Not found.' }) });
    await expect(Invite.default(params())).rejects.toThrow('NEXT_NOT_FOUND');
  });

  it.each(['cri_short', `crsl_${'B'.repeat(43)}`, `cri_${'B'.repeat(43)}?x`])(
    'answers 404 for %s without calling the API',
    async (bad) => {
      await expect(Invite.default(params(bad))).rejects.toThrow('NEXT_NOT_FOUND');
      expect(fetchMock).not.toHaveBeenCalled();
    },
  );

  it('renders the dead invite page', () => {
    render(<InviteNotFound />);
    expect(
      screen.getByRole('heading', { level: 1, name: 'This invite no longer works' }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Invites work for 7 days. Ask a team owner to send a new one/),
    ).toBeInTheDocument();
  });

  it('keeps the token out of the metadata', async () => {
    const meta = Invite.metadata;
    expect(JSON.stringify(meta)).not.toContain('cri_');
    expect(meta.robots).toEqual({ index: false, follow: false });
  });
});

describe('acceptInvite', () => {
  beforeEach(() => {
    jar.current = cookieJar({ cr_session: 'crs_ok' });
  });

  it('accepts, then goes to the team', async () => {
    routes({ [ACCEPT]: () => json(200, { slug: 'acme', name: 'Acme', role: 'member' }) });
    const err = await actions.acceptInvite(TOKEN).catch((e: unknown) => e);
    expect((err as RedirectSignal).url).toBe('/t/acme');
    const call = fetchMock.mock.calls[0];
    expect(call[1].method).toBe('POST');
    expect(call[1].headers.get('authorization')).toBe('Bearer crs_ok');
  });

  it('sends an existing member to the team on a 409', async () => {
    routes({
      [ACCEPT]: () =>
        json(409, { error: 'conflict', message: 'You are already a member of this team.' }),
      [PREVIEW]: () => json(200, preview),
    });
    const err = await actions.acceptInvite(TOKEN).catch((e: unknown) => e);
    expect((err as RedirectSignal).url).toBe('/t/acme');
  });

  it('returns the plan_limit code on a 402', async () => {
    routes({
      [ACCEPT]: () =>
        json(402, { error: 'plan_limit', message: 'The Free plan allows 3 members.' }),
    });
    expect(await actions.acceptInvite(TOKEN)).toMatchObject({ ok: false, code: 'plan_limit' });
  });

  it('sends a used or expired invite to its dead page', async () => {
    routes({ [ACCEPT]: () => json(404, { error: 'not_found', message: 'Not found.' }) });
    const err = await actions.acceptInvite(TOKEN).catch((e: unknown) => e);
    expect((err as RedirectSignal).url).toBe(`/invites/${TOKEN}`);
  });

  it('refuses a malformed token without calling the API', async () => {
    expect(await actions.acceptInvite('../x')).toMatchObject({ ok: false, code: 'not_found' });
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
