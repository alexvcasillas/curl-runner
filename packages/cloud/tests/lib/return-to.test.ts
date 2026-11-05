import { describe, expect, it } from 'vitest';
import { safeReturnTo } from '@/lib/return-to';

describe('safeReturnTo', () => {
  it.each(['/t/acme', '/s/x?y=1', '/t/acme/tokens#new', '/'])('accepts the local path %s', (v) => {
    expect(safeReturnTo(v)).toBe(v);
  });

  it.each([
    '//evil.example',
    'https://evil.example',
    '/\\evil',
    '\\\\evil',
    '%2F%2Fevil',
    '/%2F/evil.example',
    '/%5Cevil',
    '/\t/evil.example',
    '/%09/evil.example',
    'javascript:alert(1)',
    't/acme',
    '%E0%A4%A',
    '',
    null,
    undefined,
  ])('rejects %j and gives /', (v) => {
    expect(safeReturnTo(v)).toBe('/');
  });
});
