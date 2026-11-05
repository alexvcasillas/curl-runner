import { NextRequest } from 'next/server';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { isPublicPath, proxy } from '@/proxy';

function req(path: string, cookie?: string) {
  return new NextRequest(`http://localhost:3000${path}`, {
    headers: cookie ? { cookie } : {},
  });
}

describe('proxy', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it('sends signed-out requests for app routes to sign in, with return_to', () => {
    const res = proxy(req('/t/acme/tokens?x=1'));
    expect(res.status).toBe(307);
    expect(res.headers.get('location')).toBe(
      `http://localhost:3000/sign-in?return_to=${encodeURIComponent('/t/acme/tokens?x=1')}`,
    );
  });

  it('sends a signed-out / to sign in without return_to', () => {
    expect(proxy(req('/')).headers.get('location')).toBe('http://localhost:3000/sign-in');
  });

  it('passes signed-in requests through with x-pathname', () => {
    const res = proxy(req('/t/acme?y=2', 'cr_session=crs_abc'));
    expect(res.headers.get('location')).toBeNull();
    expect(res.headers.get('x-middleware-request-x-pathname')).toBe('/t/acme?y=2');
  });

  it.each([
    '/sign-in',
    '/sign-in/code',
    '/s/crsl_abc',
    '/invites/abc',
    '/.well-known/curl-runner.json',
    '/healthz',
    '/robots.txt',
    '/favicon.ico',
  ])('leaves public %s alone', (path) => {
    expect(isPublicPath(path)).toBe(true);
    const res = proxy(req(path));
    expect(res.headers.get('location')).toBeNull();
    expect(res.headers.get('x-middleware-request-x-pathname')).toBe(path);
  });

  it.each(['/sign-inx', '/shared', '/s', '/invitesx', '/t/acme', '/account'])(
    'guards %s',
    (path) => {
      expect(isPublicPath(path)).toBe(false);
    },
  );

  it('opens the dev kit outside production only', () => {
    vi.stubEnv('NODE_ENV', 'development');
    expect(isPublicPath('/dev/kit')).toBe(true);
    vi.stubEnv('NODE_ENV', 'production');
    expect(isPublicPath('/dev/kit')).toBe(false);
  });
});
