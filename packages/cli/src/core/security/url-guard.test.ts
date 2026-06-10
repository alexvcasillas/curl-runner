import { describe, expect, test } from 'bun:test';
import { DEFAULT_ALLOWED_PROTOCOLS, validateUrl } from './url-guard';

describe('DEFAULT_ALLOWED_PROTOCOLS', () => {
  test('contains http and https', () => {
    expect(DEFAULT_ALLOWED_PROTOCOLS).toContain('http');
    expect(DEFAULT_ALLOWED_PROTOCOLS).toContain('https');
    expect(DEFAULT_ALLOWED_PROTOCOLS).toHaveLength(2);
  });
});

describe('validateUrl', () => {
  describe('allowed protocols pass', () => {
    test('http URL is valid', () => {
      const result = validateUrl('http://example.com/api', DEFAULT_ALLOWED_PROTOCOLS);
      expect(result.valid).toBe(true);
      expect(result.error).toBeUndefined();
    });

    test('https URL is valid', () => {
      const result = validateUrl('https://api.example.com/users', DEFAULT_ALLOWED_PROTOCOLS);
      expect(result.valid).toBe(true);
      expect(result.error).toBeUndefined();
    });

    test('https with port is valid', () => {
      const result = validateUrl('https://localhost:3000/health', DEFAULT_ALLOWED_PROTOCOLS);
      expect(result.valid).toBe(true);
    });

    test('http with query params is valid', () => {
      const result = validateUrl(
        'http://example.com/search?q=test&limit=10',
        DEFAULT_ALLOWED_PROTOCOLS,
      );
      expect(result.valid).toBe(true);
    });
  });

  describe('blocked protocols are rejected', () => {
    test('file:// is blocked', () => {
      const result = validateUrl('file:///etc/passwd', DEFAULT_ALLOWED_PROTOCOLS);
      expect(result.valid).toBe(false);
      expect(result.error).toContain('Blocked protocol "file"');
      expect(result.error).toContain('allowed: http, https');
      expect(result.error).toContain('--allow-protocol');
    });

    test('gopher:// is blocked', () => {
      const result = validateUrl('gopher://evil.example.com/', DEFAULT_ALLOWED_PROTOCOLS);
      expect(result.valid).toBe(false);
      expect(result.error).toContain('Blocked protocol "gopher"');
    });

    test('ftp:// is blocked', () => {
      const result = validateUrl('ftp://files.example.com/data.txt', DEFAULT_ALLOWED_PROTOCOLS);
      expect(result.valid).toBe(false);
      expect(result.error).toContain('Blocked protocol "ftp"');
    });

    test('dict:// is blocked', () => {
      const result = validateUrl('dict://attacker.example.com/', DEFAULT_ALLOWED_PROTOCOLS);
      expect(result.valid).toBe(false);
      expect(result.error).toContain('Blocked protocol "dict"');
    });

    test('error message lists allowed protocols', () => {
      const result = validateUrl('file:///tmp/secret', ['http', 'https']);
      expect(result.error).toContain('allowed: http, https');
    });

    test('error message includes --allow-protocol hint', () => {
      const result = validateUrl('ftp://example.com', DEFAULT_ALLOWED_PROTOCOLS);
      expect(result.error).toContain('--allow-protocol');
    });
  });

  describe('custom allowedProtocols', () => {
    test('ftp passes when explicitly allowed', () => {
      const result = validateUrl('ftp://files.example.com/data.txt', ['http', 'https', 'ftp']);
      expect(result.valid).toBe(true);
      expect(result.error).toBeUndefined();
    });

    test('http blocked when not in custom list', () => {
      const result = validateUrl('http://example.com', ['https']);
      expect(result.valid).toBe(false);
      expect(result.error).toContain('Blocked protocol "http"');
      expect(result.error).toContain('allowed: https');
    });

    test('protocol from URL is lowercased before comparison', () => {
      // URL.protocol always returns lowercase; guard lowercases too — 'https' matches 'https'
      const result = validateUrl('https://example.com', ['https']);
      expect(result.valid).toBe(true);
    });

    test('scp blocked by default', () => {
      const result = validateUrl('scp://user@host:/path', DEFAULT_ALLOWED_PROTOCOLS);
      expect(result.valid).toBe(false);
    });
  });

  describe('malformed URLs are rejected', () => {
    test('empty string is invalid', () => {
      const result = validateUrl('', DEFAULT_ALLOWED_PROTOCOLS);
      expect(result.valid).toBe(false);
      expect(result.error).toContain('Invalid URL');
    });

    test('no protocol is invalid', () => {
      const result = validateUrl('example.com/api', DEFAULT_ALLOWED_PROTOCOLS);
      expect(result.valid).toBe(false);
      expect(result.error).toContain('Invalid URL');
    });

    test('malformed URL with spaces is invalid', () => {
      const result = validateUrl('http://exa mple.com', DEFAULT_ALLOWED_PROTOCOLS);
      expect(result.valid).toBe(false);
      expect(result.error).toContain('Invalid URL');
    });

    test('only protocol is invalid', () => {
      const result = validateUrl('https://', DEFAULT_ALLOWED_PROTOCOLS);
      expect(result.valid).toBe(false);
    });
  });
});
