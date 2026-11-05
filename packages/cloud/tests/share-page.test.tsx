import { render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from './cookies';

const jar = { current: cookieJar() };

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
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));
// The server-side `cache` is a per-request memo; tests want a fresh call each time.
vi.mock('react', async (orig) => ({
  ...(await orig<typeof import('react')>()),
  cache: <T,>(fn: T) => fn,
}));

const SharePage = (await import('@/app/s/[token]/page')).default;
const { resetEnv } = await import('@/lib/env');

const TOKEN = `crsl_${'a'.repeat(43)}`;
const info = {
  name: 'Payments API',
  team: 'Acme',
  team_slug: 'acme',
  collection: 'payments-api',
  description: null,
  access: 'public',
  request_count: 4,
  version: 3,
  updated_at: '2026-11-01T10:00:00Z',
};

const json = (status: number, body: unknown, headers: Record<string, string> = {}) =>
  new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json', ...headers },
  });

const fetchMock = vi.fn();

/** Answers `/v1/shared/*` by whether the request carried a bearer. */
function shared(withBearer: () => Response, anonymous: () => Response) {
  fetchMock.mockImplementation(async (url: string, init: RequestInit) => {
    const path = new URL(url).pathname;
    const bearer = new Headers(init.headers).has('authorization');
    if (path.startsWith('/v1/shared/')) {
      return bearer ? withBearer() : anonymous();
    }
    if (path === '/v1/me') {
      return json(401, { error: 'unauthorized', message: 'Invalid session.' });
    }
    throw new Error(`unexpected ${path}`);
  });
}

const stale = () => json(401, { error: 'unauthorized', message: 'Invalid session.' });
const render_ = async () => render(await SharePage({ params: Promise.resolve({ token: TOKEN }) }));

describe('share page with a stale session cookie', () => {
  beforeEach(() => {
    resetEnv();
    vi.stubEnv('API_INTERNAL_URL', 'http://api.internal:8080');
    vi.stubEnv('PUBLIC_APP_URL', 'https://cloud.example.com');
    vi.stubEnv('PUBLIC_API_URL', 'https://api.example.com');
    vi.stubGlobal('fetch', fetchMock);
    fetchMock.mockReset();
    jar.current = cookieJar({ cr_session: 'crs_dead' });
  });
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.unstubAllGlobals();
    resetEnv();
  });

  it('shows a Public link after retrying without the cookie', async () => {
    shared(stale, () => json(200, info));
    await render_();
    expect(screen.getByText('Payments API')).toBeInTheDocument();
    expect(screen.queryByText(/Sign in to open/)).toBeNull();
    const calls = fetchMock.mock.calls.filter(([u]) => String(u).includes('/v1/shared/'));
    expect(calls).toHaveLength(2);
    expect(new Headers(calls[1][1].headers).has('authorization')).toBe(false);
  });

  it('is a 404 for a dead link', async () => {
    shared(stale, () => json(404, { error: 'not_found', message: 'No such link.' }));
    await expect(render_()).rejects.toThrow('NEXT_NOT_FOUND');
  });

  it('still asks a restricted link to sign in when the anonymous call is a 401 too', async () => {
    shared(stale, stale);
    await render_();
    expect(screen.getByText('Sign in to open this collection.')).toBeInTheDocument();
  });

  it('shows the API message and the wait on a rate limit', async () => {
    shared(
      () => json(429, { error: 'rate_limited', message: 'Slow down.' }, { 'retry-after': '120' }),
      () => json(200, info),
    );
    await render_();
    expect(screen.getByText('Too many tries')).toBeInTheDocument();
    expect(screen.getByText('Too many tries. Try again in 2 minutes.')).toBeInTheDocument();
  });
});
