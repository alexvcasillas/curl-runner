import { describe, expect, it } from 'vitest';
import { isInviteToken, isShareToken } from '@/lib/tokens';

const body = `${'aZ09-_'.repeat(7)}a`;

describe('token shapes', () => {
  it('accepts what the server mints: a prefix and 43 url-safe characters', () => {
    expect(body).toHaveLength(43);
    expect(isShareToken(`crsl_${body}`)).toBe(true);
    expect(isInviteToken(`cri_${body}`)).toBe(true);
  });

  it.each([
    '',
    'crsl_',
    `crsl_${body}x`,
    `crsl_${body.slice(1)}`,
    `cri_${body}`,
    `crsl_${body.slice(1)}/`,
  ])('rejects %s as a share token', (v) => expect(isShareToken(v)).toBe(false));

  it.each(['', `crsl_${body}`, `cri_${body}\n`, `cri_${body.slice(1)}.`])(
    'rejects %s as an invite token',
    (v) => expect(isInviteToken(v)).toBe(false),
  );
});
