import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { env, resetEnv } from '@/lib/env';

describe('env', () => {
  beforeEach(() => resetEnv());
  afterEach(() => {
    vi.unstubAllEnvs();
    resetEnv();
  });

  it('defaults to localhost outside production', () => {
    vi.stubEnv('NODE_ENV', 'development');
    vi.stubEnv('PUBLIC_APP_URL', '');
    vi.stubEnv('PUBLIC_API_URL', '');
    expect(env()).toMatchObject({
      PUBLIC_APP_URL: 'http://localhost:3000',
      PUBLIC_API_URL: 'http://localhost:8080',
      API_INTERNAL_URL: 'http://localhost:8080',
    });
  });

  it('throws in production without the public URLs', () => {
    vi.stubEnv('NODE_ENV', 'production');
    vi.stubEnv('PUBLIC_APP_URL', ' ');
    vi.stubEnv('PUBLIC_API_URL', '');
    expect(() => env()).toThrow('Set PUBLIC_APP_URL and PUBLIC_API_URL');
  });

  it('throws in production when one is missing', () => {
    vi.stubEnv('NODE_ENV', 'production');
    vi.stubEnv('PUBLIC_APP_URL', 'https://cloud.example.com');
    vi.stubEnv('PUBLIC_API_URL', '');
    expect(() => env()).toThrow('Set PUBLIC_API_URL:');
  });

  it('accepts production with both set', () => {
    vi.stubEnv('NODE_ENV', 'production');
    vi.stubEnv('PUBLIC_APP_URL', 'https://cloud.example.com/');
    vi.stubEnv('PUBLIC_API_URL', 'https://api.example.com');
    expect(env().PUBLIC_APP_URL).toBe('https://cloud.example.com');
  });

  it('rejects a non-http URL', () => {
    vi.stubEnv('PUBLIC_API_URL', 'ftp://api.example.com');
    expect(() => env()).toThrow('Invalid environment');
  });
});
