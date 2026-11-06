import { describe, expect, it } from 'vitest';
import { initials, teamLetter } from '@/lib/initials';

describe('initials', () => {
  it.each([
    ['Ada Lovelace', 'AL'],
    ['Linus Benedict Torvalds', 'LT'],
    ['Ada', 'A'],
    ['jane@globex.example', 'JG'],
    ['grace.hopper@acme.example', 'GH'],
    ['  ', '?'],
  ])('%j → %s', (input, out) => {
    expect(initials(input)).toBe(out);
  });

  it('gives a team one letter', () => {
    expect(teamLetter('acme')).toBe('A');
    expect(teamLetter('')).toBe('?');
  });
});
