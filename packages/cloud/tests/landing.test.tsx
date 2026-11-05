import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from './cookies';

const jar = { current: cookieJar() };
const incoming = { current: new Headers({ 'x-pathname': '/s/x' }) };

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
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const Landing = await import('@/app/s/[token]/page');
const LandingNotFound = (await import('@/app/s/[token]/not-found')).default;
const { resetEnv } = await import('@/lib/env');

const TOKEN = `crsl_${'A'.repeat(43)}`;
const json = (status: number, body: unknown) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

const fetchMock = vi.fn();
const shared = (access: 'public' | 'restricted' = 'public') => ({
  team: 'Acme',
  team_slug: 'acme',
  collection: 'payments-api',
  name: 'Payments API',
  description: 'Smoke tests for the payments service.',
  version: 14,
  updated_at: new Date(Date.now() - 2 * 3_600_000).toISOString(),
  request_count: 12,
  access,
});
const me = (email = 'sam@gmail.com') =>
  json(200, { user: { id: 'u1', email, name: null }, teams: [], admin: false });

function routes(map: { shared: () => Response; me?: () => Response }) {
  fetchMock.mockImplementation(async (url: string) => {
    const path = new URL(url).pathname;
    if (path.startsWith('/v1/shared/')) {
      return map.shared();
    }
    if (path === '/v1/me' && map.me) {
      return map.me();
    }
    throw new Error(`unexpected ${path}`);
  });
}
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

describe('/s/[token]', () => {
  it('shows a public collection with its facts', async () => {
    routes({ shared: () => json(200, shared()) });
    render(await Landing.default(params()));
    expect(screen.getByRole('heading', { level: 1, name: 'Payments API' })).toBeInTheDocument();
    expect(screen.getByText('Acme shared a collection with you')).toBeInTheDocument();
    expect(screen.getByText('Smoke tests for the payments service.')).toBeInTheDocument();
    expect(screen.getByText('12')).toBeInTheDocument();
    expect(screen.getByText('v14')).toBeInTheDocument();
    expect(screen.getByText('2 hours ago')).toBeInTheDocument();
    expect(screen.getByText('Public link')).toBeInTheDocument();
  });

  it('calls the API with the session when there is one, and without when there is not', async () => {
    routes({ shared: () => json(200, shared()) });
    await Landing.default(params());
    expect(fetchMock.mock.calls[0][0]).toBe(`http://api.internal:8080/v1/shared/${TOKEN}`);
    expect(fetchMock.mock.calls[0][1].headers.get('authorization')).toBeNull();

    fetchMock.mockClear();
    jar.current = cookieJar({ cr_session: 'crs_ok' });
    routes({ shared: () => json(200, shared()), me: () => me() });
    await Landing.default(params());
    expect(fetchMock.mock.calls[0][1].headers.get('authorization')).toBe('Bearer crs_ok');
  });

  it('builds the studio deep link from the team slug and the API URL', async () => {
    routes({ shared: () => json(200, shared()) });
    render(await Landing.default(params()));
    expect(screen.getByRole('link', { name: /Open in curl-runner studio/ })).toHaveAttribute(
      'href',
      `curl-runner://open/acme/payments-api?share=${TOKEN}&server=https%3A%2F%2Fapi.example.com`,
    );
  });

  it('links the download and builds the pull command from the app URL', async () => {
    routes({ shared: () => json(200, shared()) });
    render(await Landing.default(params()));
    expect(screen.getByRole('link', { name: 'Download the studio for macOS' })).toHaveAttribute(
      'href',
      'https://www.curl-runner.com/downloads',
    );
    expect(screen.getByText('Not on a Mac?')).toBeInTheDocument();
    expect(
      screen.getByText(`curl-runner pull https://cloud.example.com/s/${TOKEN}`),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Copy the pull command' })).toBeInTheDocument();
  });

  it('opens the "Studio didn’t open?" help when Open is clicked', async () => {
    const user = userEvent.setup();
    routes({ shared: () => json(200, shared()) });
    render(await Landing.default(params()));
    const summary = screen.getByText('Studio didn’t open?');
    const details = summary.closest('details') as HTMLDetailsElement;
    expect(details.open).toBe(false);
    const open = screen.getByRole('link', { name: /Open in curl-runner studio/ });
    open.addEventListener('click', (e) => e.preventDefault());
    await user.click(open);
    expect(details.open).toBe(true);
    expect(details).toHaveTextContent('The studio needs to be installed first');
    expect(details.textContent).not.toMatch(/\d+\.\d+/);
  });

  it('asks a signed-out visitor to sign in on a restricted link', async () => {
    routes({
      shared: () =>
        json(401, { error: 'unauthorized', message: 'Sign in to open this collection.' }),
    });
    render(await Landing.default(params()));
    expect(screen.getByText('Sign in to open this collection.')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Sign in to open' })).toHaveAttribute(
      'href',
      `/sign-in?return_to=${encodeURIComponent(`/s/${TOKEN}`)}`,
    );
    expect(screen.queryByRole('link', { name: /Open in curl-runner studio/ })).toBeNull();
  });

  it('treats a rejected session like signed out', async () => {
    jar.current = cookieJar({ cr_session: 'crs_stale' });
    routes({
      shared: () => json(401, { error: 'unauthorized', message: 'Sign in.' }),
      me: () => json(401, { error: 'unauthorized', message: 'Sign in.' }),
    });
    render(await Landing.default(params()));
    expect(screen.getByRole('link', { name: 'Sign in to open' })).toBeInTheDocument();
  });

  it('names the team and the signed-in email on a 403, with a way to switch', async () => {
    jar.current = cookieJar({ cr_session: 'crs_ok' });
    routes({
      shared: () =>
        json(403, {
          error: 'forbidden',
          message: 'Your account isn’t on the list for this collection. Ask Acme for access.',
        }),
      me: () => me('sam@gmail.com'),
    });
    render(await Landing.default(params()));
    expect(screen.getByRole('alert')).toHaveTextContent('Ask Acme for access.');
    expect(screen.getByText(/Signed in as sam@gmail.com/)).toBeInTheDocument();
    const form = screen.getByRole('button', { name: 'Use another account' }).closest('form');
    expect(form).toHaveAttribute('method', 'post');
    expect(form).toHaveAttribute(
      'action',
      `/sign-out?return_to=${encodeURIComponent(`/s/${TOKEN}`)}`,
    );
  });

  it('answers 404 for a dead link', async () => {
    routes({ shared: () => json(404, { error: 'not_found', message: 'Not found.' }) });
    await expect(Landing.default(params())).rejects.toThrow('NEXT_NOT_FOUND');
  });

  it.each([
    'crsl_short',
    `crs_${'A'.repeat(43)}`,
    `crsl_${'A'.repeat(42)}!`,
    `crsl_${'A'.repeat(44)}`,
    '../etc',
  ])('answers 404 for the malformed token %s without calling the API', async (bad) => {
    await expect(Landing.default(params(bad))).rejects.toThrow('NEXT_NOT_FOUND');
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('lets an unavailable API reach the error page', async () => {
    routes({ shared: () => json(503, { error: 'unavailable', message: 'Down.' }) });
    await expect(Landing.default(params())).rejects.toMatchObject({ code: 'unavailable' });
  });

  it('never puts the token in the title or the metadata', async () => {
    routes({ shared: () => json(200, shared()) });
    const meta = await Landing.generateMetadata(params());
    expect(JSON.stringify(meta)).not.toContain(TOKEN);
    expect(JSON.stringify(meta)).not.toContain('crsl_');
    expect(meta.robots).toEqual({ index: false, follow: false });

    fetchMock.mockClear();
    routes({ shared: () => json(404, { error: 'not_found', message: 'x' }) });
    const dead = await Landing.generateMetadata(params());
    expect(JSON.stringify(dead)).not.toContain('crsl_');
  });

  it('renders the dead-link page with the same layout', () => {
    render(<LandingNotFound />);
    expect(
      screen.getByRole('heading', { level: 1, name: 'This link no longer works' }),
    ).toBeInTheDocument();
    expect(screen.getByText(/Ask the team that shared it for a new one/)).toBeInTheDocument();
    expect(screen.getByText('404 Not Found')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'What’s curl-runner?' })).toHaveAttribute(
      'href',
      'https://www.curl-runner.com',
    );
  });

  it('labels a restricted collection and shows the signed-in email in the header', async () => {
    jar.current = cookieJar({ cr_session: 'crs_ok' });
    routes({ shared: () => json(200, shared('restricted')), me: () => me('jane@example.com') });
    render(await Landing.default(params()));
    expect(screen.getByText('Restricted')).toBeInTheDocument();
    expect(screen.getAllByText('jane@example.com').length).toBeGreaterThan(0);
    expect(screen.getByRole('link', { name: /Open in curl-runner studio/ })).toBeInTheDocument();
  });
});
