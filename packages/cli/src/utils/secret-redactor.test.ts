import { afterEach, describe, expect, test } from 'bun:test';
import {
  getGlobalRedactor,
  registerRequestSecrets,
  resetGlobalRedactor,
  SecretRedactor,
} from './secret-redactor';

// Helper to build test keys at runtime to bypass GitHub secret scanning
const testKey = (prefix: string, suffix: string) => prefix + suffix;

describe('SecretRedactor', () => {
  describe('explicit secret redaction', () => {
    test('redacts registered secret value', () => {
      const redactor = new SecretRedactor();
      redactor.addSecret('API_KEY', 'supersecret123');

      const result = redactor.redact('Authorization: Bearer supersecret123');
      expect(result).toBe('Authorization: Bearer [REDACTED]');
    });

    test('redacts multiple secrets', () => {
      const redactor = new SecretRedactor();
      redactor.addSecrets({
        API_KEY: 'key123',
        TOKEN: 'token456',
      });

      const result = redactor.redact('key: key123, token: token456');
      expect(result).toBe('key: [REDACTED], token: [REDACTED]');
    });

    test('redacts longer secrets first to handle overlapping', () => {
      const redactor = new SecretRedactor();
      redactor.addSecrets({
        SHORT: 'abc',
        LONG: 'abcdef',
      });

      const result = redactor.redact('secret: abcdef');
      expect(result).toBe('secret: [REDACTED]');
    });

    test('redacts all occurrences of a secret', () => {
      const redactor = new SecretRedactor();
      redactor.addSecret('KEY', 'secret');

      const result = redactor.redact('first: secret, second: secret, third: secret');
      expect(result).toBe('first: [REDACTED], second: [REDACTED], third: [REDACTED]');
    });

    test('ignores empty secret values', () => {
      const redactor = new SecretRedactor();
      redactor.addSecret('EMPTY', '');
      redactor.addSecrets({ ALSO_EMPTY: '' });

      expect(redactor.secretCount).toBe(0);
    });

    test('returns unchanged string when no secrets match', () => {
      const redactor = new SecretRedactor();
      redactor.addSecret('KEY', 'mysecret');

      const result = redactor.redact('no secrets here');
      expect(result).toBe('no secrets here');
    });

    test('clear removes all secrets', () => {
      const redactor = new SecretRedactor();
      redactor.addSecrets({ A: 'secret1', B: 'secret2' });
      expect(redactor.hasSecrets()).toBe(true);

      redactor.clear();
      expect(redactor.hasSecrets()).toBe(false);
      expect(redactor.secretCount).toBe(0);
    });
  });

  describe('pattern-based redaction', () => {
    test('redacts Stripe live keys', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact(`key: ${testKey('sk_live_', 'abcdefghijklmnopqrstuvwx')}`);
      expect(result).toBe('key: [REDACTED]');
    });

    test('redacts Stripe test keys', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact(`key: ${testKey('sk_test_', 'abcdefghijklmnopqrstuvwx')}`);
      expect(result).toBe('key: [REDACTED]');
    });

    test('redacts Stripe publishable keys', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact(testKey('pk_live_', 'abcdefghijklmnopqrstuvwx'));
      expect(result).toBe('[REDACTED]');
    });

    test('redacts Stripe restricted keys', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact(testKey('rk_live_', 'abcdefghijklmnopqrstuvwx'));
      expect(result).toBe('[REDACTED]');
    });

    test('redacts AWS access key IDs', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact('AWS_KEY=AKIAIOSFODNN7EXAMPLE');
      expect(result).toBe('AWS_KEY=[REDACTED]');
    });

    test('redacts GitHub personal access tokens', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact('token: ghp_abcdefghijklmnopqrstuvwxyz0123456789');
      expect(result).toBe('token: [REDACTED]');
    });

    test('redacts GitHub OAuth tokens', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact('gho_abcdefghijklmnopqrstuvwxyz0123456789');
      expect(result).toBe('[REDACTED]');
    });

    test('redacts GitHub user tokens', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact('ghu_abcdefghijklmnopqrstuvwxyz0123456789');
      expect(result).toBe('[REDACTED]');
    });

    test('redacts GitHub server tokens', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact('ghs_abcdefghijklmnopqrstuvwxyz0123456789');
      expect(result).toBe('[REDACTED]');
    });

    test('redacts GitHub refresh tokens', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact('ghr_abcdefghijklmnopqrstuvwxyz0123456789');
      expect(result).toBe('[REDACTED]');
    });

    test('redacts NPM tokens', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact('npm_abcdefghijklmnopqrstuvwxyz0123456789');
      expect(result).toBe('[REDACTED]');
    });

    test('redacts Slack tokens', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact('xoxb-123456789-abcdefghij');
      expect(result).toBe('[REDACTED]');
    });

    test('redacts Slack app tokens', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact('xoxp-123456789-abcdefghij');
      expect(result).toBe('[REDACTED]');
    });

    test('redacts Paddle keys', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact('pdl_abcdefghijklmnopqrst');
      expect(result).toBe('[REDACTED]');
    });

    test('redacts OpenAI keys', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact('sk-abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWX');
      expect(result).toBe('[REDACTED]');
    });

    test('redacts Anthropic keys', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact('sk-ant-api03-abcdefghijklmnopqrstuvwxyzABCDEFGHIJKL');
      expect(result).toBe('[REDACTED]');
    });

    test('redacts Bearer tokens', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact(
        'Authorization: Bearer abcdefghijklmnopqrstuvwxyz0123456789ABCD',
      );
      expect(result).toBe('Authorization: [REDACTED]');
    });

    test('can disable pattern redaction', () => {
      const redactor = new SecretRedactor();
      redactor.setPatternRedaction(false);
      const stripeKey = testKey('sk_live_', 'abcdefghijklmnopqrstuvwx');

      const result = redactor.redact(stripeKey);
      expect(result).toBe(stripeKey);
    });

    test('can re-enable pattern redaction', () => {
      const redactor = new SecretRedactor();
      redactor.setPatternRedaction(false);
      redactor.setPatternRedaction(true);

      const result = redactor.redact(testKey('sk_live_', 'abcdefghijklmnopqrstuvwx'));
      expect(result).toBe('[REDACTED]');
    });

    test('redacts multiple patterns in same string', () => {
      const redactor = new SecretRedactor();
      const stripeKey = testKey('sk_live_', 'abcdefghijklmnopqrstuvwx');
      const ghKey = testKey('ghp_', 'abcdefghijklmnopqrstuvwxyz0123456789');
      const input = `stripe: ${stripeKey}, github: ${ghKey}`;
      const result = redactor.redact(input);
      expect(result).toBe('stripe: [REDACTED], github: [REDACTED]');
    });
  });

  describe('combined redaction', () => {
    test('redacts both explicit secrets and patterns', () => {
      const redactor = new SecretRedactor();
      redactor.addSecret('MY_SECRET', 'mysuperpassword');
      const stripeKey = testKey('sk_live_', 'abcdefghijklmnopqrstuvwx');

      const input = `password: mysuperpassword, stripe: ${stripeKey}`;
      const result = redactor.redact(input);
      expect(result).toBe('password: [REDACTED], stripe: [REDACTED]');
    });

    test('explicit secrets take precedence over patterns', () => {
      const redactor = new SecretRedactor();
      const stripeKey = testKey('sk_live_', 'abcdefghijklmnopqrstuvwx');
      // Register the full stripe key as explicit secret
      redactor.addSecret('STRIPE_KEY', stripeKey);

      const result = redactor.redact(`key: ${stripeKey}`);
      expect(result).toBe('key: [REDACTED]');
    });
  });

  describe('hasSecrets and secretCount', () => {
    test('hasSecrets returns false initially', () => {
      const redactor = new SecretRedactor();
      expect(redactor.hasSecrets()).toBe(false);
    });

    test('hasSecrets returns true after adding secrets', () => {
      const redactor = new SecretRedactor();
      redactor.addSecret('KEY', 'value');
      expect(redactor.hasSecrets()).toBe(true);
    });

    test('secretCount returns correct count', () => {
      const redactor = new SecretRedactor();
      expect(redactor.secretCount).toBe(0);

      redactor.addSecret('A', 'value1');
      expect(redactor.secretCount).toBe(1);

      redactor.addSecrets({ B: 'value2', C: 'value3' });
      expect(redactor.secretCount).toBe(3);
    });
  });

  describe('global singleton', () => {
    afterEach(() => {
      resetGlobalRedactor();
    });

    test('getGlobalRedactor returns same instance', () => {
      const r1 = getGlobalRedactor();
      const r2 = getGlobalRedactor();
      expect(r1).toBe(r2);
    });

    test('resetGlobalRedactor creates new instance', () => {
      const r1 = getGlobalRedactor();
      r1.addSecret('KEY', 'value');

      resetGlobalRedactor();
      const r2 = getGlobalRedactor();

      expect(r2.hasSecrets()).toBe(false);
    });
  });

  describe('new patterns', () => {
    test('redacts JWT tokens', () => {
      const redactor = new SecretRedactor();
      const jwt =
        'eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c';
      const result = redactor.redact(`token: ${jwt}`);
      expect(result).toBe('token: [REDACTED]');
    });

    test('redacts Google API keys', () => {
      const redactor = new SecretRedactor();
      const key = 'AIzaSyD-9tSrke72I6e0dvos8Fy7T5EeHzNs9MA';
      const result = redactor.redact(`key=${key}`);
      expect(result).toBe('key=[REDACTED]');
    });

    test('redacts api_key query param value', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact('https://api.example.com?api_key=supersecret123&other=ok');
      expect(result).toBe('https://api.example.com?api_key=[REDACTED]&other=ok');
    });

    test('redacts token query param value', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact('url?token=abc123xyz');
      expect(result).toBe('url?token=[REDACTED]');
    });

    test('redacts secret query param value', () => {
      const redactor = new SecretRedactor();
      const result = redactor.redact('body: secret=mysecretvalue');
      expect(result).toBe('body: secret=[REDACTED]');
    });

    test('does not over-redact benign text', () => {
      const redactor = new SecretRedactor();
      const benign = 'Hello world, the count is 42, status=ok';
      expect(redactor.redact(benign)).toBe(benign);
    });
  });

  describe('registerRequestSecrets', () => {
    afterEach(() => {
      resetGlobalRedactor();
    });

    test('registers bearer token and redacts it', () => {
      registerRequestSecrets([
        {
          url: 'https://example.com',
          auth: { type: 'bearer', token: 'mysupersecretbearertoken123' },
        },
      ]);
      const result = getGlobalRedactor().redact(
        'Authorization: Bearer mysupersecretbearertoken123',
      );
      expect(result).toBe('Authorization: Bearer [REDACTED]');
    });

    test('registers basic password and redacts it', () => {
      registerRequestSecrets([
        {
          url: 'https://example.com',
          auth: { type: 'basic', username: 'admin', password: 'p@ssw0rd!' },
        },
      ]);
      const result = getGlobalRedactor().redact('password is p@ssw0rd!');
      expect(result).toBe('password is [REDACTED]');
    });

    test('registers basic user:pass combined and redacts it', () => {
      registerRequestSecrets([
        {
          url: 'https://example.com',
          auth: { type: 'basic', username: 'admin', password: 'p@ssw0rd!' },
        },
      ]);
      const result = getGlobalRedactor().redact('-u admin:p@ssw0rd!');
      expect(result).toBe('-u [REDACTED]');
    });

    test('registers sensitive header value and redacts it', () => {
      registerRequestSecrets([
        { url: 'https://example.com', headers: { 'X-Api-Key': 'header-secret-value-xyz' } },
      ]);
      const result = getGlobalRedactor().redact('x-api-key: header-secret-value-xyz');
      expect(result).toBe('x-api-key: [REDACTED]');
    });

    test('registers bare token from Authorization Bearer header', () => {
      registerRequestSecrets([
        { url: 'https://example.com', headers: { Authorization: 'Bearer rawtoken987654' } },
      ]);
      const redactor = getGlobalRedactor();
      expect(redactor.redact('token rawtoken987654 found')).toBe('token [REDACTED] found');
    });

    test('does not register non-sensitive headers', () => {
      registerRequestSecrets([
        { url: 'https://example.com', headers: { 'Content-Type': 'application/json' } },
      ]);
      const result = getGlobalRedactor().redact('Content-Type: application/json');
      expect(result).toBe('Content-Type: application/json');
    });

    test('handles multiple requests', () => {
      registerRequestSecrets([
        { url: 'https://a.example.com', auth: { type: 'bearer', token: 'tokenAAA' } },
        { url: 'https://b.example.com', auth: { type: 'bearer', token: 'tokenBBB' } },
      ]);
      const redactor = getGlobalRedactor();
      expect(redactor.redact('a=tokenAAA b=tokenBBB')).toBe('a=[REDACTED] b=[REDACTED]');
    });

    test('summary JSON string is redacted', () => {
      registerRequestSecrets([
        { url: 'https://example.com', auth: { type: 'bearer', token: 'json-secret-token' } },
      ]);
      const summaryJson = JSON.stringify(
        { request: { auth: { token: 'json-secret-token' } } },
        null,
        2,
      );
      const redacted = getGlobalRedactor().redact(summaryJson);
      expect(redacted).not.toContain('json-secret-token');
      expect(redacted).toContain('[REDACTED]');
    });
  });

  describe('edge cases', () => {
    test('handles empty string', () => {
      const redactor = new SecretRedactor();
      redactor.addSecret('KEY', 'secret');

      expect(redactor.redact('')).toBe('');
    });

    test('handles string with only whitespace', () => {
      const redactor = new SecretRedactor();
      redactor.addSecret('KEY', 'secret');

      expect(redactor.redact('   \n\t  ')).toBe('   \n\t  ');
    });

    test('handles special regex characters in secrets', () => {
      const redactor = new SecretRedactor();
      redactor.addSecret('REGEX', 'secret.*+?^${}()|[]\\');

      const result = redactor.redact('value: secret.*+?^${}()|[]\\');
      expect(result).toBe('value: [REDACTED]');
    });

    test('handles unicode in secrets', () => {
      const redactor = new SecretRedactor();
      redactor.addSecret('UNICODE', 'пароль123');

      const result = redactor.redact('password: пароль123');
      expect(result).toBe('password: [REDACTED]');
    });

    test('handles very long strings', () => {
      const redactor = new SecretRedactor();
      redactor.addSecret('KEY', 'secret');

      const longString = 'prefix'.repeat(10000) + 'secret' + 'suffix'.repeat(10000);
      const result = redactor.redact(longString);

      expect(result).toContain('[REDACTED]');
      expect(result).not.toContain('secret');
    });
  });
});
