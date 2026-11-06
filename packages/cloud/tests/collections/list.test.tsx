import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';
import { info, me } from '../fixtures';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };
const incoming = { current: new Headers({ 'x-pathname': '/t/acme' }) };

vi.mock('next/headers', () => ({
  cookies: async () => jar.current.store,
  headers: async () => incoming.current,
}));
vi.mock('next/navigation', () => ({
  redirect: (url: string) => {
    throw new Error(`NEXT_REDIRECT ${url}`);
  },
  notFound: () => {
    throw new Error('NEXT_NOT_FOUND');
  },
  usePathname: () => '/t/acme',
}));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { default: TeamPage } = await import('@/app/(app)/t/[team]/page');
const { default: Loading } = await import('@/app/(app)/t/[team]/loading');
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
    const path = new URL(url).pathname;
    const make = routes[path];
    if (!make) {
      throw new Error(`unexpected ${path}`);
    }
    return make();
  });

// A function, so `updated_at` is computed after the clock is pinned in beforeEach.
const collections = () => [
  {
    slug: 'payments-api',
    name: 'Payments API',
    source: 'git',
    access: 'public',
    current_version: 14,
    updated_at: new Date(Date.now() - 2 * 3600_000).toISOString(),
    publisher: 'token github-actions',
  },
  {
    slug: 'webhooks',
    name: 'Webhooks',
    source: 'git',
    access: 'restricted',
    current_version: 3,
    updated_at: '2026-11-04T10:00:00Z',
    publisher: 'grace@acme.example',
  },
  {
    slug: 'sandbox-tour',
    name: 'Sandbox tour',
    source: 'cloud',
    access: 'off',
    current_version: null,
    updated_at: null,
    publisher: null,
  },
];

const infoBody =
  (billing = true) =>
  () =>
    json(200, { ...info, billing });

beforeEach(() => {
  // Pin the clock to midday UTC: "2 hours ago" must not cross a calendar day.
  vi.useFakeTimers({ toFake: ['Date'] });
  vi.setSystemTime(new Date('2026-11-04T12:00:00Z'));
  resetEnv();
  vi.stubEnv('API_INTERNAL_URL', 'http://api.internal:8080');
  vi.stubGlobal('fetch', fetchMock);
  fetchMock.mockReset();
  jar.current = cookieJar({ cr_session: 'crs_ok' });
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
  resetEnv();
});

const params = (team = 'acme') => ({ params: Promise.resolve({ team }) });

describe('collections list', () => {
  it('shows each collection from one request, with source, access, version and last publish', async () => {
    byPath({
      '/v1/me': () => json(200, me),
      '/v1/info': infoBody(),
      '/v1/teams/acme/collections': () => json(200, collections()),
    });
    render(await TeamPage(params()));
    expect(screen.getByRole('heading', { name: 'Collections' })).toBeInTheDocument();
    const rows = screen.getAllByRole('row').slice(1);
    expect(rows).toHaveLength(3);

    const first = within(rows[0]);
    expect(first.getByRole('link', { name: 'Payments API' })).toHaveAttribute(
      'href',
      '/t/acme/c/payments-api',
    );
    expect(first.getByText('payments-api')).toBeInTheDocument();
    expect(first.getByText('git')).toBeInTheDocument();
    expect(first.getByText('Public')).toBeInTheDocument();
    expect(first.getByText('v14')).toBeInTheDocument();
    const when = first.getByText('2 hours ago');
    expect(when).toHaveAttribute('title', expect.stringMatching(/UTC$/));
    expect(first.getByText('token github-actions')).toBeInTheDocument();

    const never = within(rows[2]);
    expect(never.getByText('Cloud')).toBeInTheDocument();
    expect(never.getByText('Off')).toBeInTheDocument();
    expect(never.getByText('Not published')).toBeInTheDocument();
    expect(screen.getByText('3 collections')).toBeInTheDocument();
    // The publisher comes with the list: no per-collection requests.
    expect(fetchMock.mock.calls.map((c) => new URL(c[0]).pathname).sort()).toEqual([
      '/v1/info',
      '/v1/me',
      '/v1/teams/acme/collections',
    ]);
  });

  it('filters by access and searches by name or slug', async () => {
    byPath({
      '/v1/me': () => json(200, me),
      '/v1/info': infoBody(),
      '/v1/teams/acme/collections': () => json(200, collections()),
    });
    const user = userEvent.setup();
    render(await TeamPage(params()));
    await user.click(screen.getByRole('radio', { name: 'Restricted' }));
    expect(screen.getAllByRole('row').slice(1)).toHaveLength(1);
    expect(screen.getByText('webhooks')).toBeInTheDocument();
    await user.click(screen.getByRole('radio', { name: 'All' }));
    await user.type(screen.getByRole('searchbox', { name: 'Search collections' }), 'sandbox');
    expect(screen.getAllByRole('row').slice(1)).toHaveLength(1);
    expect(screen.getByText('Sandbox tour')).toBeInTheDocument();
    await user.clear(screen.getByRole('searchbox', { name: 'Search collections' }));
    await user.type(screen.getByRole('searchbox', { name: 'Search collections' }), 'nothing');
    expect(screen.getByText('No collections match.')).toBeInTheDocument();
  });

  it('says what the plan covers, or that a self-hosted server has everything', async () => {
    const routes = (billing: boolean, plan: 'free' | 'team') => ({
      '/v1/me': () => json(200, { ...me, teams: [{ ...me.teams[0], plan }] }),
      '/v1/info': infoBody(billing),
      '/v1/teams/acme/collections': () => json(200, [collections()[2]]),
    });
    byPath(routes(true, 'free'));
    const free = render(await TeamPage(params()));
    expect(screen.getByText(/Free plan: public collections are unlimited/)).toBeInTheDocument();
    free.unmount();
    byPath(routes(false, 'team'));
    render(await TeamPage(params()));
    expect(screen.getByText('Self-hosted: every feature is on.')).toBeInTheDocument();
  });

  it('shows the empty state with the first steps, the docs and Tokens', async () => {
    byPath({
      '/v1/me': () => json(200, me),
      '/v1/info': infoBody(),
      '/v1/teams/acme/collections': () => json(200, []),
    });
    render(await TeamPage(params()));
    expect(
      screen.getByRole('heading', { name: 'Publish your first collection' }),
    ).toBeInTheDocument();
    expect(screen.getByText(/curl-runner publish/)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Create token' })).toHaveAttribute(
      'href',
      '/t/acme/tokens',
    );
    expect(screen.getByRole('link', { name: 'Read the guide' })).toHaveAttribute(
      'href',
      'https://www.curl-runner.com/docs/features/cloud',
    );
    expect(screen.getByText(/secrets\.CURL_RUNNER_TOKEN/)).toBeInTheDocument();
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
  });

  it('is not found for a team the person is not in', async () => {
    byPath({ '/v1/me': () => json(200, me), '/v1/info': infoBody() });
    await expect(TeamPage(params('other'))).rejects.toThrow('NEXT_NOT_FOUND');
  });

  it('has a loading state of skeleton rows', () => {
    render(<Loading />);
    expect(screen.getByRole('heading', { name: 'Collections' })).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent('Loading');
  });
});
