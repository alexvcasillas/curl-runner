import { describe, expect, it } from 'vitest';
import { parseCollectionsQuery } from '@/lib/admin-collections';
import { parseLogQuery } from '@/lib/admin-emails';
import {
  FIRST_PAGE,
  MAX_TRAIL,
  nextPage,
  pagingParams,
  parseCursor,
  parseSearch,
  parseTrail,
  previousPage,
} from '@/lib/admin-paging';
import { parseTeamsQuery } from '@/lib/admin-teams';
import { parseUsersQuery } from '@/lib/admin-users';

const long = Array.from({ length: 80 }, (_, i) => `c${i}`).join('.');

describe('parsing', () => {
  it('checks cursors and trims and cuts q', () => {
    expect(parseCursor('abc_1-')).toBe('abc_1-');
    expect(parseCursor('a b!')).toBeUndefined();
    expect(parseSearch('  hi  ')).toBe('hi');
    expect(parseSearch('x'.repeat(300))).toHaveLength(200);
  });

  it('drops bad trail entries', () => {
    expect(parseTrail('-.AAA.<script>.BBB')).toEqual(['-', 'AAA', 'BBB']);
  });
});

describe('trail cap', () => {
  it('keeps at most 50 entries', () => {
    expect(parseTrail(long)).toHaveLength(MAX_TRAIL);
  });

  it.each([
    ['users', () => parseUsersQuery({ trail: long }).trail],
    ['teams', () => parseTeamsQuery({ trail: long }).trail],
    ['collections', () => parseCollectionsQuery({ trail: long }).trail],
    ['emails', () => parseLogQuery({ trail: long }).trail],
  ])('applies to the %s list', (_name, read) => {
    expect(read()).toHaveLength(50);
  });

  it('never grows past the cap going forward', () => {
    const trail = Array.from({ length: 50 }, (_, i) => `c${i}`);
    const next = nextPage(trail, 'c50', 'c51');
    expect(next.trail).toHaveLength(50);
    expect(next.trail.at(-1)).toBe('c50');
    expect(next.cursor).toBe('c51');
  });
});

describe('previous and next', () => {
  it('goes back to the first page without a cursor', () => {
    expect(previousPage([FIRST_PAGE])).toEqual({ cursor: undefined, trail: [] });
    expect(previousPage([FIRST_PAGE, 'c1'])).toEqual({ cursor: 'c1', trail: [FIRST_PAGE] });
  });

  it('adds the page you leave to the trail', () => {
    expect(nextPage([], undefined, 'c2')).toEqual({ cursor: 'c2', trail: [FIRST_PAGE] });
    expect(nextPage([FIRST_PAGE], 'c2', 'c3')).toEqual({ cursor: 'c3', trail: [FIRST_PAGE, 'c2'] });
  });

  it('writes cursor and trail into params', () => {
    expect(pagingParams(new URLSearchParams(), { cursor: 'c', trail: ['-', 'a'] }).toString()).toBe(
      'cursor=c&trail=-.a',
    );
    expect(pagingParams(new URLSearchParams(), { trail: [] }).toString()).toBe('');
  });
});
