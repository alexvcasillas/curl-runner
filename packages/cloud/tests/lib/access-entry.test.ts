import { describe, expect, it } from 'vitest';
import { isAccessEntry, normalizeEntry } from '@/lib/access-entry';

describe('access entry', () => {
  it.each(['ada@customer.com', '*@Customer.com', 'a@b', 'ada+cust@example.com', '  ada@x.io '])(
    'accepts %s',
    (v) => expect(isAccessEntry(v)).toBe(true),
  );
  it.each([
    'foo',
    '*@',
    '@x.com',
    '',
    'a@b@c.com',
    'a b@x.com',
    '*@localhost',
    '*@a..com',
    '*@.com',
    'a*@x.com',
    '..',
  ])('rejects %j', (v) => expect(isAccessEntry(v)).toBe(false));

  it('trims and lower-cases', () => {
    expect(normalizeEntry('  *@Customer.COM ')).toBe('*@customer.com');
  });
});
