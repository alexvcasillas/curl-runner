import { describe, expect, it } from 'vitest';
import {
  type ActivityFilters,
  activityExportHref,
  activityHref,
  detailPairs,
  parseActivityQuery,
  sinceIso,
  utcStamp,
} from '@/lib/admin-activity';

const ADMIN = 'a0000000-0000-4000-8000-000000000001';

describe('parseActivityQuery', () => {
  it('defaults to every admin, every action, the last 7 days', () => {
    expect(parseActivityQuery({})).toEqual({
      admin: undefined,
      group: 'all',
      since: '7d',
      cursor: undefined,
      trail: [],
    });
  });

  it('keeps what is valid and drops what is not', () => {
    const q = parseActivityQuery({
      admin: ADMIN,
      group: 'suspension',
      since: 'all',
      cursor: 'abc_-1',
      trail: '-.abc',
    });
    expect(q).toEqual({
      admin: ADMIN,
      group: 'suspension',
      since: 'all',
      cursor: 'abc_-1',
      trail: ['-', 'abc'],
    });
    const bad = parseActivityQuery({ admin: 'nope', group: 'x', since: '1y', cursor: '../' });
    expect(bad).toMatchObject({ admin: undefined, group: 'all', since: '7d', cursor: undefined });
  });
});

describe('trail cap', () => {
  it('keeps at most 50 entries', () => {
    const trail = Array.from({ length: 80 }, (_, i) => `c${i}`).join('.');
    expect(parseActivityQuery({ trail }).trail).toHaveLength(50);
  });
});

describe('hrefs', () => {
  it('writes only what differs from the default', () => {
    const q: ActivityFilters & { trail: string[] } = {
      admin: undefined,
      group: 'all',
      since: '7d',
      trail: [],
    };
    expect(activityHref(q)).toBe('/admin/activity');
    expect(activityHref({ ...q, group: 'email', since: '30d', admin: ADMIN })).toBe(
      `/admin/activity?admin=${ADMIN}&group=email&since=30d`,
    );
    expect(activityHref({ ...q, cursor: 'c1', trail: ['-'] })).toBe(
      '/admin/activity?cursor=c1&trail=-',
    );
  });

  it('the export keeps the filters and never a cursor', () => {
    expect(activityExportHref({ admin: ADMIN, group: 'settings', since: 'all' })).toBe(
      `/admin/activity/export?admin=${ADMIN}&group=settings&since=all`,
    );
    expect(activityExportHref({ admin: undefined, group: 'all', since: '7d' })).toBe(
      '/admin/activity/export',
    );
  });
});

describe('sinceIso', () => {
  const now = new Date('2026-11-06T12:00:00Z');
  it('counts back from now, and all time has no bound', () => {
    expect(sinceIso('7d', now)).toBe('2026-10-30T12:00:00.000Z');
    expect(sinceIso('30d', now)).toBe('2026-10-07T12:00:00.000Z');
    expect(sinceIso('all', now)).toBeUndefined();
  });
});

describe('detailPairs', () => {
  it('reads flat details as labelled values', () => {
    expect(
      detailPairs({ reason: 'Pilot ended', emailed_owners: true, recipients: 1151, note: null }),
    ).toEqual([
      ['Reason', 'Pilot ended'],
      ['Emailed owners', 'Yes'],
      ['Recipients', '1151'],
      ['Note', 'None'],
    ]);
  });

  it('shows from and to as one change per field', () => {
    expect(
      detailPairs({
        section: 'signup',
        from: { mode: 'closed', domains: [] },
        to: { mode: 'domains', domains: ['acme.com', 'globex.com'] },
      }),
    ).toEqual([
      ['Section', 'signup'],
      ['Mode', 'closed → domains'],
      ['Domains', 'None → acme.com, globex.com'],
    ]);
  });

  it('lists arrays, flattens objects, and survives anything', () => {
    expect(detailPairs({ permissions: ['users', 'emails'], extra: { a: 1 } })).toEqual([
      ['Permissions', 'users, emails'],
      ['Extra a', '1'],
    ]);
    expect(detailPairs(null)).toEqual([]);
    expect(detailPairs({})).toEqual([]);
    expect(detailPairs('plain')).toEqual([['Detail', 'plain']]);
  });

  it('returns text, never markup', () => {
    const [[, v]] = detailPairs({ reason: '<img src=x onerror=alert(1)>' });
    expect(v).toBe('<img src=x onerror=alert(1)>');
  });
});

describe('utcStamp', () => {
  it('is the board’s "Nov 5 14:08"', () => {
    expect(utcStamp('2026-11-05T14:08:33Z')).toBe('Nov 5 14:08');
  });
});
