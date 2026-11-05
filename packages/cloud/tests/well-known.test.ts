import { afterEach, describe, expect, it, vi } from 'vitest';
import { GET } from '@/app/.well-known/curl-runner.json/route';
import { GET as healthz } from '@/app/healthz/route';
import { resetEnv } from '@/lib/env';

describe('/.well-known/curl-runner.json', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    resetEnv();
  });

  it('advertises PUBLIC_API_URL, cached for 5 minutes', async () => {
    resetEnv();
    vi.stubEnv('PUBLIC_API_URL', 'https://api.example.com/');
    const res = await GET();
    expect(res.status).toBe(200);
    expect(res.headers.get('cache-control')).toBe('public, max-age=300');
    expect(await res.json()).toEqual({ api: 'https://api.example.com' });
  });

  it('defaults to the local API', async () => {
    resetEnv();
    vi.stubEnv('PUBLIC_API_URL', '');
    const res = await GET();
    expect(await res.json()).toEqual({ api: 'http://localhost:8080' });
  });
});

describe('/healthz', () => {
  it('answers ok', async () => {
    const res = await healthz();
    expect(res.status).toBe(200);
    expect(await res.text()).toBe('ok');
  });
});
