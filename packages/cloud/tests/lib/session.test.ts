import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';

const jar = { current: cookieJar() };
vi.mock('next/headers', () => ({
  cookies: async () => jar.current.store,
}));

const { getSession, setSession, clearSession } = await import('@/lib/session');
const { resetEnv } = await import('@/lib/env');

describe('session cookie', () => {
  beforeEach(() => {
    jar.current = cookieJar();
    resetEnv();
  });
  afterEach(() => {
    vi.unstubAllEnvs();
    resetEnv();
  });

  it('is HttpOnly, SameSite=Lax, Path=/ and lasts 90 days', async () => {
    vi.stubEnv('PUBLIC_APP_URL', 'https://cloud.example.com');
    await setSession('crs_abc');
    expect(jar.current.sets).toEqual([
      {
        name: 'cr_session',
        value: 'crs_abc',
        options: { httpOnly: true, secure: true, sameSite: 'lax', path: '/', maxAge: 7776000 },
      },
    ]);
  });

  it('drops Secure only for http://localhost', async () => {
    vi.stubEnv('PUBLIC_APP_URL', 'http://localhost:3000');
    await setSession('crs_abc');
    expect(jar.current.sets[0].options.secure).toBe(false);
  });

  it('keeps Secure for other http hosts', async () => {
    vi.stubEnv('PUBLIC_APP_URL', 'http://localhost.example.com');
    await setSession('crs_abc');
    expect(jar.current.sets[0].options.secure).toBe(true);
  });

  it('reads and clears the token', async () => {
    jar.current = cookieJar({ cr_session: 'crs_xyz' });
    expect(await getSession()).toBe('crs_xyz');
    await clearSession();
    expect(await getSession()).toBeNull();
    const last = jar.current.sets.at(-1);
    expect(last?.name).toBe('cr_session');
    expect(last?.options).toMatchObject({ maxAge: 0, path: '/', httpOnly: true });
  });

  it('gives null without a cookie', async () => {
    expect(await getSession()).toBeNull();
  });
});
