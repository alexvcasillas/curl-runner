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

const actions = await import('@/app/sign-in/actions');
const { resetEnv } = await import('@/lib/env');

function json(status: number, body: unknown, headers: Record<string, string> = {}) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json', ...headers },
  });
}

function form(values: Record<string, string>) {
  const f = new FormData();
  for (const [k, v] of Object.entries(values)) {
    f.set(k, v);
  }
  return f;
}

async function redirected(p: Promise<unknown>): Promise<string> {
  const err = await p.catch((e: unknown) => e);
  expect(err).toBeInstanceOf(RedirectSignal);
  return (err as RedirectSignal).url;
}

const fetchMock = vi.fn();
const calls = () => fetchMock.mock.calls.map(([url, init]) => [url, init.method, init.body]);

beforeEach(() => {
  resetEnv();
  vi.stubEnv('API_INTERNAL_URL', 'http://api.internal:8080');
  vi.stubEnv('PUBLIC_APP_URL', 'https://cloud.example.com');
  vi.stubGlobal('fetch', fetchMock);
  fetchMock.mockReset();
  jar.current = cookieJar();
  incoming.current = new Headers({ 'x-forwarded-for': '203.0.113.9' });
});
afterEach(() => {
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
  resetEnv();
});

describe('email code', () => {
  it('starts with client web, then verifies and sets the session', async () => {
    fetchMock.mockResolvedValueOnce(new Response(null, { status: 202 }));
    const started = await actions.startEmail(
      form({ email: ' Ada@Example.com ', return_to: '/t/acme' }),
    );
    expect(started).toEqual({ ok: true, email: 'ada@example.com' });

    fetchMock.mockResolvedValueOnce(
      json(200, { session: 'crs_new', user: { id: 'u1', email: 'ada@example.com', name: null } }),
    );
    const to = await redirected(
      actions.verifyCode(
        form({ email: 'ada@example.com', code: '481903', return_to: '/t/acme', client: 'web' }),
      ),
    );
    expect(to).toBe('/t/acme');
    expect(calls()).toEqual([
      [
        'http://api.internal:8080/v1/auth/email/start',
        'POST',
        JSON.stringify({ email: 'ada@example.com', client: 'web', label: 'Browser' }),
      ],
      [
        'http://api.internal:8080/v1/auth/email/verify',
        'POST',
        JSON.stringify({ email: 'ada@example.com', code: '481903' }),
      ],
    ]);
    const session = jar.current.sets.find((s) => s.name === 'cr_session');
    expect(session?.value).toBe('crs_new');
    expect(session?.options).toMatchObject({ httpOnly: true, secure: true, sameSite: 'lax' });
  });

  it('sends client=device to the device page, and studio to the studio page', async () => {
    fetchMock.mockImplementation(async () =>
      json(200, { session: 'crs_new', user: { id: 'u1', email: 'a@b.co', name: null } }),
    );
    const device = form({ email: 'a@b.co', code: '123456', client: 'device', return_to: '/' });
    expect(await redirected(actions.verifyCode(device))).toBe('/sign-in/device');
    const studio = form({ email: 'a@b.co', code: '123456', client: 'studio' });
    expect(await redirected(actions.verifyCode(studio))).toBe('/sign-in/studio');
    // The API call itself stays client web.
    expect(fetchMock.mock.calls.every(([url]) => !String(url).includes('device'))).toBe(true);
  });

  it('ignores a hostile return_to', async () => {
    fetchMock.mockImplementation(async () =>
      json(200, { session: 'crs_new', user: { id: 'u1', email: 'a@b.co', name: null } }),
    );
    for (const bad of ['//evil.example', 'https://evil.example', '/\\evil', '%2F%2Fevil']) {
      const to = await redirected(
        actions.verifyCode(form({ email: 'a@b.co', code: '123456', return_to: bad })),
      );
      expect(to).toBe('/');
    }
  });

  it('sets the code outcome for a wrong, expired and throttled code', async () => {
    const f = form({ email: 'a@b.co', code: '000000' });
    fetchMock.mockResolvedValueOnce(json(401, { error: 'code_invalid', message: 'Nope.' }));
    expect(await actions.verifyCode(f)).toMatchObject({ ok: false, kind: 'wrong' });
    fetchMock.mockResolvedValueOnce(json(401, { error: 'code_expired', message: 'Old.' }));
    expect(await actions.verifyCode(f)).toMatchObject({ ok: false, kind: 'expired' });
    expect(jar.current.sets.find((s) => s.name === 'cr_session')).toBeUndefined();
  });

  it('does not call the API for a code that is not 6 digits', async () => {
    const r = await actions.verifyCode(form({ email: 'a@b.co', code: '12' }));
    expect(r).toMatchObject({ ok: false, kind: 'wrong' });
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('shows the API 429 message with the wait', async () => {
    fetchMock.mockResolvedValueOnce(
      json(
        429,
        { error: 'rate_limited', message: 'Too many requests. Try again in a few minutes.' },
        { 'retry-after': '720' },
      ),
    );
    const r = await actions.verifyCode(form({ email: 'a@b.co', code: '123456' }));
    expect(r).toMatchObject({ ok: false, kind: 'throttled' });
    expect((r as { message: string }).message).toContain('Try again in 12 minutes');
  });

  it('start: a 429 names the wait, and keeps the code step closed', async () => {
    fetchMock.mockResolvedValueOnce(
      json(429, { error: 'rate_limited', message: 'Slow.' }, { 'retry-after': '720' }),
    );
    const r = await actions.startEmail(form({ email: 'a@b.co' }));
    expect(r).toEqual({
      ok: false,
      kind: 'throttled',
      retryAfter: 720,
      message: expect.any(String),
    });
  });

  it('start: a rejected address shows the API message', async () => {
    fetchMock.mockResolvedValueOnce(
      json(400, { error: 'invalid_request', message: 'Enter a valid email address.' }),
    );
    const r = await actions.startEmail(form({ email: 'nope' }));
    expect(r).toEqual({ ok: false, kind: 'error', message: 'Enter a valid email address.' });
  });

  it('remembers a local destination for the email link, and forgets a hostile one', async () => {
    fetchMock.mockResolvedValue(new Response(null, { status: 202 }));
    await actions.startEmail(form({ email: 'a@b.co', return_to: '/t/acme' }));
    const kept = jar.current.sets.find((s) => s.name === 'cr_return_to');
    expect(kept?.value).toBe('/t/acme');
    expect(kept?.options).toMatchObject({ httpOnly: true, path: '/sign-in', maxAge: 600 });
    await actions.startEmail(form({ email: 'a@b.co', return_to: '//evil.example' }));
    expect(jar.current.values.get('cr_return_to')).toBeUndefined();
  });
});

describe('email link', () => {
  const ok = () =>
    json(200, { session: 'crs_link', user: { id: 'u1', email: 'a@b.co', name: null } });

  it('sets the session and honours a local return_to', async () => {
    fetchMock.mockResolvedValueOnce(ok());
    const to = await redirected(actions.verifyLink('crl_abc', '/t/acme/tokens'));
    expect(to).toBe('/t/acme/tokens');
    expect(calls()[0]).toEqual([
      'http://api.internal:8080/v1/auth/email/verify',
      'POST',
      JSON.stringify({ token: 'crl_abc' }),
    ]);
    expect(jar.current.sets.find((s) => s.name === 'cr_session')?.value).toBe('crs_link');
  });

  it('falls back to the destination remembered at start, then to /', async () => {
    jar.current = cookieJar({ cr_return_to: '/sign-in/device' });
    fetchMock.mockResolvedValueOnce(ok());
    expect(await redirected(actions.verifyLink('crl_abc', undefined))).toBe('/sign-in/device');
    jar.current = cookieJar();
    fetchMock.mockResolvedValueOnce(ok());
    expect(await redirected(actions.verifyLink('crl_abc', undefined))).toBe('/');
  });

  it('ignores //evil.example and the other hostile forms', async () => {
    for (const bad of ['//evil.example', 'https://evil.example', '/\\evil', '%2F%2Fevil']) {
      fetchMock.mockResolvedValueOnce(ok());
      expect(await redirected(actions.verifyLink('crl_abc', bad))).toBe('/');
    }
  });

  it('says the link expired, with a way back', async () => {
    fetchMock.mockResolvedValueOnce(json(401, { error: 'code_expired', message: 'Old.' }));
    const r = await actions.verifyLink('crl_abc', undefined);
    expect(r).toMatchObject({
      title: 'This link has expired',
      actionLabel: 'Send a new code',
      actionHref: '/sign-in',
    });
    fetchMock.mockResolvedValueOnce(json(401, { error: 'code_invalid', message: 'No.' }));
    expect(await actions.verifyLink('crl_abc', undefined)).toMatchObject({
      title: 'This link has expired',
    });
  });

  it('points a device link at the terminal', async () => {
    fetchMock.mockResolvedValueOnce(
      json(400, { error: 'invalid_request', message: 'Enter this code in your terminal.' }),
    );
    const r = await actions.verifyLink('crl_abc', undefined);
    expect(r).toMatchObject({
      title: 'Enter this code in your terminal',
      actionLabel: 'Sign in on the web instead',
    });
    expect((r as { hint?: string }).hint).toContain('The CLI asked for this code');
  });
});

describe('oauth', () => {
  it('refuses a provider we do not offer, without calling the API', async () => {
    for (const bad of ['../admin', 'github/../x', 'gitlab', '']) {
      expect(await actions.startOAuth(bad, '/', 'web')).toMatchObject({ ok: false });
      jar.current = cookieJar({ cr_oauth: 'b' });
      expect(await actions.finishOAuth(bad, 'c', 's')).toMatchObject({
        title: 'That sign-in method isn’t available',
        actionHref: '/sign-in',
      });
    }
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('names the provider in the expired copy', async () => {
    const r = await actions.finishOAuth('google', 'c', 's');
    expect((r as { body: string }).body).toContain('opened the Google page twice');
  });

  it('stores the binding in cr_oauth and redirects to the provider', async () => {
    fetchMock.mockResolvedValueOnce(
      json(200, { url: 'https://github.com/login/oauth/authorize?x=1', binding: 'bind_1' }),
    );
    const to = await redirected(actions.startOAuth('github', '/t/acme', 'web'));
    expect(to).toBe('https://github.com/login/oauth/authorize?x=1');
    expect(calls()[0]).toEqual([
      'http://api.internal:8080/v1/auth/oauth/github/start',
      'POST',
      JSON.stringify({ client: 'web', return_to: '/t/acme' }),
    ]);
    const c = jar.current.sets.find((s) => s.name === 'cr_oauth');
    expect(c?.value).toBe('bind_1');
    expect(c?.options).toEqual({
      httpOnly: true,
      secure: true,
      sameSite: 'lax',
      path: '/sign-in',
      maxAge: 600,
    });
  });

  it('refuses a provider URL that is not https', async () => {
    fetchMock.mockResolvedValueOnce(
      json(200, { url: 'http://github.com/login/oauth/authorize', binding: 'b' }),
    );
    const r = await actions.startOAuth('github', '/', 'web');
    expect(r.ok).toBe(false);
    expect(jar.current.sets.find((s) => s.name === 'cr_oauth')).toBeUndefined();
  });

  it('sends a hostile return_to as /', async () => {
    fetchMock.mockResolvedValueOnce(json(200, { url: 'https://x.example/a', binding: 'b' }));
    await redirected(actions.startOAuth('google', '//evil.example', 'web'));
    expect(JSON.parse(calls()[0][2])).toEqual({ client: 'web', return_to: '/' });
  });

  it('with client=device the destination is the device page', async () => {
    fetchMock.mockResolvedValueOnce(json(200, { url: 'https://x.example/a', binding: 'b' }));
    await redirected(actions.startOAuth('github', '/', 'device'));
    expect(JSON.parse(calls()[0][2])).toEqual({ client: 'web', return_to: '/sign-in/device' });
  });

  it('finishes: posts the callback, deletes the cookie, sets the session, redirects', async () => {
    jar.current = cookieJar({ cr_oauth: 'bind_1' });
    fetchMock.mockResolvedValueOnce(
      json(200, {
        session: 'crs_oauth',
        user: { id: 'u1', email: 'a@b.co', name: 'Ada' },
        return_to: '/t/acme',
        client: 'web',
      }),
    );
    const to = await redirected(actions.finishOAuth('github', 'code_1', 'state_1'));
    expect(to).toBe('/t/acme');
    expect(calls()[0]).toEqual([
      'http://api.internal:8080/v1/auth/oauth/github/callback',
      'POST',
      JSON.stringify({ code: 'code_1', state: 'state_1', binding: 'bind_1' }),
    ]);
    expect(jar.current.sets.find((s) => s.name === 'cr_session')?.value).toBe('crs_oauth');
    const gone = jar.current.sets.find((s) => s.name === 'cr_oauth');
    expect(gone).toMatchObject({ value: '', options: { maxAge: 0, path: '/sign-in' } });
  });

  it('ignores a hostile return_to from the callback', async () => {
    jar.current = cookieJar({ cr_oauth: 'b' });
    fetchMock.mockResolvedValueOnce(
      json(200, { session: 'crs_o', user: {}, return_to: '//evil.example', client: 'web' }),
    );
    expect(await redirected(actions.finishOAuth('github', 'c', 's'))).toBe('/');
  });

  it('without the binding cookie shows the expired message and never calls the API', async () => {
    const r = await actions.finishOAuth('github', 'code_1', 'state_1');
    expect(r).toMatchObject({
      title: 'That sign-in has expired',
      actionLabel: 'Start again',
      actionHref: '/sign-in',
    });
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('maps invalid_request to expired and unauthorized to the verified-email message', async () => {
    jar.current = cookieJar({ cr_oauth: 'b' });
    fetchMock.mockResolvedValueOnce(
      json(400, { error: 'invalid_request', message: 'That sign-in has expired. Start again.' }),
    );
    expect(await actions.finishOAuth('github', 'c', 's')).toMatchObject({
      title: 'That sign-in has expired',
    });
    jar.current.values.set('cr_oauth', 'b');
    fetchMock.mockResolvedValueOnce(json(401, { error: 'unauthorized', message: 'No.' }));
    const r = await actions.finishOAuth('github', 'c', 's');
    expect(r).toMatchObject({
      title: 'We couldn’t verify your GitHub email',
      actionLabel: 'Back to sign in',
    });
    // Both outcomes drop the binding cookie.
    expect(
      jar.current.sets.filter((s) => s.name === 'cr_oauth' && s.options.maxAge === 0),
    ).toHaveLength(2);
  });
});

describe('a suspended account', () => {
  const SUSPENDED = 'This account is suspended. Contact support.';
  const refuse = () =>
    json(403, {
      error: 'account_suspended',
      message: 'This account is suspended. Contact support.',
    });

  it('code verify says so and sets no session', async () => {
    fetchMock.mockResolvedValueOnce(refuse());
    const r = await actions.verifyCode(form({ email: 'a@b.co', code: '123456' }));
    expect(r).toEqual({ ok: false, kind: 'error', message: SUSPENDED });
    expect(jar.current.sets.find((s) => s.name === 'cr_session')).toBeUndefined();
  });

  it('the emailed link says so with a way back', async () => {
    fetchMock.mockResolvedValueOnce(refuse());
    const r = await actions.verifyLink('tok', undefined);
    expect(r).toMatchObject({
      title: 'This account is suspended',
      body: SUSPENDED,
      actionHref: '/sign-in',
    });
  });

  it('the OAuth callback says so with a way back', async () => {
    jar.current = cookieJar({ cr_oauth: 'binding' });
    fetchMock.mockResolvedValueOnce(refuse());
    const r = await actions.finishOAuth('github', 'code', 'state');
    expect(r).toMatchObject({
      title: 'This account is suspended',
      body: SUSPENDED,
      actionHref: '/sign-in',
    });
  });
});
