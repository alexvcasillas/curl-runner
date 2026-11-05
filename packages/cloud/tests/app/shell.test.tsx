import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';
import { info, me } from '../fixtures';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };
const incoming = { current: new Headers({ 'x-pathname': '/' }) };
const pathname = { current: '/' };

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
  usePathname: () => pathname.current,
}));
vi.mock('next/cache', () => ({ revalidatePath: vi.fn() }));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { default: Home } = await import('@/app/(app)/page');
const { default: NewTeamPage } = await import('@/app/(app)/new-team/page');
const { NewTeamForm } = await import('@/app/(app)/new-team/NewTeamForm');
const { AppShell } = await import('@/components/AppShell');
const { slugFromName } = await import('@/lib/slug');
const { isSlug: isValidSlug } = await import('@/lib/params');
const { parseLocation } = await import('@/lib/routes');
const { resetEnv } = await import('@/lib/env');

function json(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}
const fetchMock = vi.fn();
const byPath = (routes: Record<string, () => Response>) =>
  fetchMock.mockImplementation(async (url: string) => {
    const make = routes[new URL(url).pathname];
    if (!make) {
      throw new Error(`unexpected ${new URL(url).pathname}`);
    }
    return make();
  });
const meBody =
  (teams: unknown[], admin = false) =>
  () =>
    json(200, { user: { id: 'u1', email: 'ada@example.com', name: null }, teams, admin });
const infoBody =
  (billing = true) =>
  () =>
    json(200, { version: '2', sign_in: ['email'], signup: 'open', billing, app_url: null });

beforeEach(() => {
  resetEnv();
  vi.stubEnv('API_INTERNAL_URL', 'http://api.internal:8080');
  vi.stubEnv('PUBLIC_API_URL', 'https://api.example.com');
  vi.stubEnv('PUBLIC_APP_URL', 'https://cloud.example.com');
  vi.stubGlobal('fetch', fetchMock);
  fetchMock.mockReset();
  jar.current = cookieJar({ cr_session: 'crs_ok' });
  pathname.current = '/';
});
afterEach(() => {
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
  resetEnv();
});

const redirectOf = async (p: Promise<unknown>) =>
  ((await p.catch((e: unknown) => e)) as RedirectSignal).url;

describe('/ redirect', () => {
  it('goes to the first team', async () => {
    byPath({
      '/v1/me': meBody([
        { slug: 'acme', name: 'Acme', role: 'owner', plan: 'free' },
        { slug: 'globex', name: 'Globex', role: 'member', plan: 'free' },
      ]),
      '/v1/info': infoBody(),
    });
    expect(await redirectOf(Home())).toBe('/t/acme');
  });

  it('goes to shared with me when there are no teams but shared collections', async () => {
    byPath({
      '/v1/me': meBody([]),
      '/v1/info': infoBody(),
      '/v1/me/shared': () => json(200, [{ team: 'x', collection: 'api', name: 'API' }]),
    });
    expect(await redirectOf(Home())).toBe('/shared');
  });

  it('goes to new team when there is nothing', async () => {
    byPath({
      '/v1/me': meBody([]),
      '/v1/info': infoBody(),
      '/v1/me/shared': () => json(200, []),
    });
    expect(await redirectOf(Home())).toBe('/new-team');
  });
});

describe('slug', () => {
  it('derives a slug from the name', () => {
    expect(slugFromName('Acme')).toBe('acme');
    expect(slugFromName('  Acme Labs! ')).toBe('acme-labs');
    expect(slugFromName('--Acme   &  Co--')).toBe('acme-co');
    expect(slugFromName('a'.repeat(80))).toHaveLength(63);
    expect(slugFromName('???')).toBe('');
  });

  it('matches the server rule', () => {
    expect(isValidSlug('acme-1')).toBe(true);
    expect(isValidSlug('1acme')).toBe(true);
    expect(isValidSlug('-acme')).toBe(false);
    expect(isValidSlug('Acme')).toBe(false);
    expect(isValidSlug('a'.repeat(64))).toBe(false);
    expect(isValidSlug('')).toBe(false);
  });
});

describe('parseLocation', () => {
  it('finds the team and tab in a path', () => {
    expect(parseLocation('/t/acme')).toEqual({ team: 'acme', tab: 'collections' });
    expect(parseLocation('/t/acme/members?x=1')).toEqual({ team: 'acme', tab: 'members' });
    expect(parseLocation('/t/acme/c/api')).toEqual({ team: 'acme', tab: 'collections' });
    expect(parseLocation('/shared')).toEqual({ team: null, tab: null });
  });
});

describe('/new-team', () => {
  it('first team: title, steps, and no admin notice', async () => {
    byPath({ '/v1/me': meBody([]), '/v1/info': infoBody() });
    render(await NewTeamPage());
    expect(screen.getByRole('heading', { name: 'Create your first team' })).toBeInTheDocument();
    expect(screen.getByText('Make a publish token')).toBeInTheDocument();
    expect(screen.queryByText(/first account on this server/)).toBeNull();
    expect(screen.getByText(/Starts on the Free plan/)).toBeInTheDocument();
  });

  it('another team: plain title and no steps', async () => {
    byPath({
      '/v1/me': meBody([{ slug: 'acme', name: 'Acme', role: 'owner', plan: 'free' }]),
      '/v1/info': infoBody(),
    });
    render(await NewTeamPage());
    expect(screen.getByRole('heading', { name: 'Create a team' })).toBeInTheDocument();
    expect(screen.queryByText('Make a publish token')).toBeNull();
  });

  it('shows the slug prefix from PUBLIC_APP_URL', async () => {
    vi.stubEnv('PUBLIC_APP_URL', 'https://studio.acme.test:8443');
    resetEnv();
    byPath({ '/v1/me': meBody([]), '/v1/info': infoBody() });
    render(await NewTeamPage());
    expect(screen.getByText('studio.acme.test:8443/t/')).toBeInTheDocument();
    expect(screen.queryByText(/curl-runner\.com\/t\//)).toBeNull();
  });

  it('shows the admin notice and the self-hosted plan note', async () => {
    byPath({ '/v1/me': meBody([], true), '/v1/info': infoBody(false) });
    render(await NewTeamPage());
    expect(screen.getByRole('status')).toHaveTextContent(
      'You’re the first account on this server, so you’re its admin.',
    );
    expect(screen.getByText(/every feature is on, no plans or limits/)).toBeInTheDocument();
  });
});

describe('/ with a failing shared lookup', () => {
  it('falls through to new team instead of crashing', async () => {
    byPath({
      '/v1/me': meBody([]),
      '/v1/info': infoBody(),
      '/v1/me/shared': () => json(500, { error: 'unavailable' }),
    });
    expect(await redirectOf(Home())).toBe('/new-team');
  });

  it('still lets the sign-in redirect through', async () => {
    byPath({
      '/v1/me': meBody([]),
      '/v1/me/shared': () => json(401, { error: 'unauthorized' }),
    });
    expect(await redirectOf(Home())).toContain('/sign-in/expired');
  });
});

describe('NewTeamForm', () => {
  const run = (action: (f: FormData) => Promise<unknown>) =>
    render(
      <NewTeamForm
        email="ada@example.com"
        billing
        slugHost="cloud.example.com"
        action={action as never}
      />,
    );

  it('derives the slug from the name until the slug is edited', async () => {
    const user = userEvent.setup();
    run(vi.fn());
    const slug = screen.getByLabelText('Slug');
    await user.type(screen.getByLabelText('Team name'), 'Acme Labs');
    expect(slug).toHaveValue('acme-labs');
    await user.clear(slug);
    await user.type(slug, 'mine');
    await user.type(screen.getByLabelText('Team name'), ' Two');
    expect(slug).toHaveValue('mine');
  });

  it('shows the signup_closed copy and a link to shared', async () => {
    const user = userEvent.setup();
    run(vi.fn(async () => ({ ok: false, kind: 'closed' })));
    await user.type(screen.getByLabelText('Team name'), 'Acme');
    await user.click(screen.getByRole('button', { name: 'Create team' }));
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent(
      'Creating teams is closed on this server. You can still open collections shared with you, and join a team when someone invites you.',
    );
    expect(screen.getByRole('link', { name: 'Go to shared with me' })).toHaveAttribute(
      'href',
      '/shared',
    );
    expect(screen.getByText('Signed in as ada@example.com')).toBeInTheDocument();
    expect(screen.queryByLabelText('Slug')).toBeNull();
  });

  it('shows a taken slug on the slug field', async () => {
    const user = userEvent.setup();
    run(vi.fn(async () => ({ ok: false, kind: 'taken', message: 'That team slug is taken.' })));
    await user.type(screen.getByLabelText('Team name'), 'Acme');
    await user.click(screen.getByRole('button', { name: 'Create team' }));
    const slug = await screen.findByLabelText('Slug');
    expect(slug).toHaveAttribute('aria-invalid', 'true');
    expect(screen.getByRole('alert')).toHaveTextContent(
      'That team slug is already taken. Try acme-api or acme-eng.',
    );
  });

  it('refuses an invalid slug without calling the server', async () => {
    const user = userEvent.setup();
    const action = vi.fn();
    run(action);
    await user.type(screen.getByLabelText('Team name'), 'Acme');
    await user.clear(screen.getByLabelText('Slug'));
    await user.type(screen.getByLabelText('Slug'), '-Bad');
    await user.click(screen.getByRole('button', { name: 'Create team' }));
    expect(action).not.toHaveBeenCalled();
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });
});

describe('AppShell', () => {
  it('hides Billing when the server has no billing', () => {
    pathname.current = '/t/acme/members';
    render(
      <AppShell me={me} info={{ ...info, billing: false }}>
        <p>page</p>
      </AppShell>,
    );
    const nav = within(screen.getByRole('navigation', { name: 'Team' }));
    expect(nav.queryByRole('link', { name: 'Billing' })).toBeNull();
    expect(nav.getByRole('link', { name: 'Members' })).toHaveAttribute('aria-current', 'page');
  });

  it('shows Billing when it is on, and no tabs off team pages', () => {
    pathname.current = '/t/acme';
    const { unmount } = render(
      <AppShell me={me} info={info}>
        <p>page</p>
      </AppShell>,
    );
    expect(screen.getByRole('link', { name: 'Billing' })).toBeInTheDocument();
    unmount();
    pathname.current = '/shared';
    render(
      <AppShell me={me} info={info}>
        <p>page</p>
      </AppShell>,
    );
    expect(screen.queryByRole('navigation', { name: 'Team' })).toBeNull();
  });
});
