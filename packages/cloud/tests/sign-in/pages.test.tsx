import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';

const jar = { current: cookieJar() };
const incoming = { current: new Headers() };

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

const SignInPage = (await import('@/app/sign-in/page')).default;
const DevicePage = (await import('@/app/sign-in/device/page')).default;
const StudioPage = (await import('@/app/sign-in/studio/page')).default;
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

const info = (sign_in: string[]) => () =>
  json(200, { version: '2', sign_in, signup: 'open', billing: false });
const me = () =>
  json(200, { user: { id: 'u1', email: 'ada@example.com', name: null }, teams: [], admin: true });

beforeEach(() => {
  resetEnv();
  vi.stubEnv('API_INTERNAL_URL', 'http://api.internal:8080');
  vi.stubEnv('PUBLIC_API_URL', 'https://api.example.com');
  vi.stubEnv('PUBLIC_APP_URL', 'https://cloud.example.com');
  vi.stubGlobal('fetch', fetchMock);
  fetchMock.mockReset();
  jar.current = cookieJar();
  incoming.current = new Headers({ 'x-pathname': '/sign-in/device' });
});
afterEach(() => {
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
  resetEnv();
});

const sp = (o: Record<string, string>) => ({ searchParams: Promise.resolve(o) });

describe('/sign-in', () => {
  it('renders providers only when the server lists them', async () => {
    byPath({ '/v1/info': info(['email', 'github']) });
    render(await SignInPage(sp({})));
    expect(screen.getByRole('button', { name: /Continue with GitHub/ })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Continue with Google/ })).toBeNull();
  });

  it('shows the CLI notice for client=device', async () => {
    byPath({ '/v1/info': info(['email']) });
    render(await SignInPage(sp({ client: 'device' })));
    expect(screen.getByText(/Signing in for the curl-runner CLI/)).toBeInTheDocument();
  });

  it('does not treat a stale cookie as signed in', async () => {
    jar.current = cookieJar({ cr_session: 'crs_stale' });
    byPath({
      '/v1/me': () => json(401, { error: 'unauthorized', message: 'Sign in.' }),
      '/v1/info': info(['email']),
    });
    render(await SignInPage(sp({ return_to: '/t/acme' })));
    expect(screen.getByRole('button', { name: 'Email me a code' })).toBeInTheDocument();
  });

  it('sends a confirmed session on to return_to, or to the device page', async () => {
    jar.current = cookieJar({ cr_session: 'crs_ok' });
    byPath({ '/v1/me': me, '/v1/info': info(['email']) });
    const err = await SignInPage(sp({ return_to: '/t/acme' })).catch((e: unknown) => e);
    expect((err as RedirectSignal).url).toBe('/t/acme');
    const dev = await SignInPage(sp({ client: 'device' })).catch((e: unknown) => e);
    expect((dev as RedirectSignal).url).toBe('/sign-in/device');
    const bad = await SignInPage(sp({ return_to: '//evil.example' })).catch((e: unknown) => e);
    expect((bad as RedirectSignal).url).toBe('/');
  });
});

const handoffCalls = () =>
  fetchMock.mock.calls.filter(([u]) => String(u).endsWith('/v1/auth/handoff'));

describe('/sign-in/device', () => {
  it('mints nothing on load, then shows the code when asked', async () => {
    const user = userEvent.setup();
    jar.current = cookieJar({ cr_session: 'crs_ok' });
    byPath({
      '/v1/me': me,
      '/v1/auth/handoff': () => json(200, { code: 'crh_abc123' }),
    });
    render(await DevicePage());
    expect(handoffCalls()).toHaveLength(0);
    expect(screen.queryByText('crh_abc123')).toBeNull();
    await user.click(screen.getByRole('button', { name: 'Show code' }));
    expect(await screen.findByText('crh_abc123')).toBeInTheDocument();
    expect(screen.getByText('Paste this code in your terminal')).toBeInTheDocument();
    expect(screen.getByText(/works once and expires in 2 minutes/)).toBeInTheDocument();
    expect(handoffCalls()).toHaveLength(1);
    expect(JSON.parse(handoffCalls()[0][1].body)).toEqual({ session_label: 'CLI' });
    expect(handoffCalls()[0][1].method).toBe('POST');
  });

  it('builds the terminal line from the app URL', async () => {
    const user = userEvent.setup();
    jar.current = cookieJar({ cr_session: 'crs_ok' });
    byPath({ '/v1/me': me, '/v1/auth/handoff': () => json(200, { code: 'crh_x' }) });
    render(await DevicePage());
    await user.click(screen.getByRole('button', { name: 'Show code' }));
    expect(
      await screen.findByText('Opened https://cloud.example.com/sign-in?client=device'),
    ).toBeInTheDocument();
  });

  it('needs a session: without one it goes to sign in and back', async () => {
    const err = await DevicePage().catch((e: unknown) => e);
    expect((err as RedirectSignal).url).toBe(
      `/sign-in?return_to=${encodeURIComponent('/sign-in/device')}`,
    );
  });
});

describe('/sign-in/studio', () => {
  it('mints nothing on load; the click mints a Studio code and opens the app', async () => {
    const user = userEvent.setup();
    jar.current = cookieJar({ cr_session: 'crs_ok' });
    incoming.current = new Headers({ 'x-pathname': '/sign-in/studio' });
    byPath({
      '/v1/me': me,
      '/v1/auth/handoff': () => json(200, { code: 'crh_studio1' }),
    });
    const hrefSet = vi.fn();
    const loc = { ...window.location } as Record<string, unknown>;
    Object.defineProperty(window, 'location', {
      configurable: true,
      value: Object.defineProperty(loc, 'href', { set: hrefSet, get: () => 'http://localhost/' }),
    });
    render(await StudioPage());
    expect(handoffCalls()).toHaveLength(0);
    expect(screen.queryByRole('link', { name: 'Open curl-runner studio' })).toBeNull();
    await user.click(screen.getByRole('button', { name: 'Open curl-runner studio' }));
    const expected = `curl-runner://auth/callback?code=crh_studio1&server=${encodeURIComponent('https://api.example.com')}`;
    const open = await screen.findByRole('link', { name: 'Open curl-runner studio' });
    expect(open).toHaveAttribute('href', expected);
    expect(hrefSet).toHaveBeenCalledWith(expected);
    expect(screen.getByText('crh_studio1')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Download it' })).toHaveAttribute(
      'href',
      'https://www.curl-runner.com/downloads',
    );
    expect(JSON.parse(handoffCalls()[0][1].body)).toEqual({ session_label: 'Studio' });
  });
});
