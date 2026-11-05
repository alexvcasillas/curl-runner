import { describe, expect, it } from 'vitest';
import { shareToken, studioLink } from '@/lib/deep-link';

describe('studioLink', () => {
  it('builds the path form with the share token and the encoded server', () => {
    expect(studioLink('https://api.example.com', 'acme', 'payments-api', 'abc123')).toBe(
      'curl-runner://open/acme/payments-api?share=abc123&server=https%3A%2F%2Fapi.example.com',
    );
  });

  it('encodes a server with a port and a path', () => {
    expect(studioLink('http://localhost:8080/api', 'acme', 'x', 't')).toContain(
      'server=http%3A%2F%2Flocalhost%3A8080%2Fapi',
    );
  });

  it('keeps an underscore in the token', () => {
    expect(studioLink('https://api.example.com', 'acme', 'x', 'crsl_V8kq2LmT9')).toContain(
      'share=crsl_V8kq2LmT9&',
    );
  });

  it('encodes each path segment', () => {
    const link = studioLink('https://api.example.com', 'a/b', 'c d?', 't');
    expect(link.startsWith('curl-runner://open/a%2Fb/c%20d%3F?share=t')).toBe(true);
  });
});

describe('shareToken', () => {
  it('takes the last path segment of a share link', () => {
    expect(shareToken('https://app.example.com/s/crsl_V8kq2LmT9')).toBe('crsl_V8kq2LmT9');
    expect(shareToken('https://app.example.com/s/crsl_V8kq2LmT9/')).toBe('crsl_V8kq2LmT9');
  });
});
