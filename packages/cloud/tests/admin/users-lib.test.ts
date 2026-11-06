import { describe, expect, it } from 'vitest';
import {
  displayName,
  eventLine,
  firstName,
  methodsLabel,
  parseUsersQuery,
  teamsLabel,
  USER_FILTERS,
  usersExportHref,
  usersHref,
} from '@/lib/admin-users';
import { rows } from './users-fixtures';

describe('parseUsersQuery', () => {
  it('defaults to every user, first page', () => {
    expect(parseUsersQuery({})).toEqual({ q: '', filter: 'all', cursor: undefined, trail: [] });
  });

  it('keeps a known filter and falls back to all for anything else', () => {
    expect(parseUsersQuery({ filter: 'suspended' }).filter).toBe('suspended');
    expect(parseUsersQuery({ filter: 'bogus' }).filter).toBe('all');
    expect(parseUsersQuery({ filter: ['members', 'admins'] }).filter).toBe('all');
  });

  it('trims q and caps it at the server limit of 200', () => {
    expect(parseUsersQuery({ q: '  ada  ' }).q).toBe('ada');
    expect(parseUsersQuery({ q: 'x'.repeat(300) }).q).toHaveLength(200);
  });

  it('drops a cursor that is not URL-safe base64', () => {
    expect(parseUsersQuery({ cursor: 'MTc2:abc' }).cursor).toBeUndefined();
    expect(parseUsersQuery({ cursor: 'MTc2OTQ_-x' }).cursor).toBe('MTc2OTQ_-x');
  });

  it('reads the trail of earlier pages and drops bad entries', () => {
    expect(parseUsersQuery({ trail: '-.AAA.BBB' }).trail).toEqual(['-', 'AAA', 'BBB']);
    expect(parseUsersQuery({ trail: '-.<script>' }).trail).toEqual(['-']);
  });
});

describe('usersHref', () => {
  it('is the bare path for the defaults', () => {
    expect(usersHref({ q: '', filter: 'all', trail: [] })).toBe('/admin/users');
  });

  it('carries q, filter, cursor and trail', () => {
    expect(usersHref({ q: 'a b', filter: 'members', cursor: 'C2', trail: ['-'] })).toBe(
      '/admin/users?q=a+b&filter=members&cursor=C2&trail=-',
    );
  });

  it('builds the export link without a cursor', () => {
    expect(usersExportHref({ q: 'ada', filter: 'suspended' })).toBe(
      '/admin/users/export?q=ada&filter=suspended',
    );
    expect(usersExportHref({ q: '', filter: 'all' })).toBe('/admin/users/export');
  });
});

describe('labels', () => {
  it('offers the board filters in order', () => {
    expect(USER_FILTERS.map((f) => f.label)).toEqual([
      'All',
      'Team members',
      'Customers only',
      'Admins',
      'Suspended',
    ]);
  });

  it('names sign-in methods', () => {
    expect(methodsLabel(['email', 'github', 'google'])).toBe('Email · GitHub · Google');
    expect(methodsLabel(['email', 'gitlab'])).toBe('Email · Gitlab');
  });

  it('lists teams with the owner role, or None', () => {
    expect(teamsLabel(rows[0].teams)).toBe('Acme (owner), Globex Partners');
    expect(teamsLabel([])).toBe('None');
  });

  it('falls back from name to email', () => {
    expect(displayName(rows[0])).toBe('Ada Lovelace');
    expect(displayName(rows[2])).toBe('jane@globex.example');
  });

  it('says a first name, else the start of the email', () => {
    expect(firstName(rows[0])).toBe('Ada');
    expect(firstName(rows[2])).toBe('jane');
    expect(firstName({ name: '  ', email: 'x@y.co' })).toBe('x');
  });
});

describe('eventLine', () => {
  it('names the admin, the verb and the reason', () => {
    expect(
      eventLine(
        {
          id: '1',
          at: '2026-11-05T13:02:00Z',
          admin: 'alex@example.com',
          action: 'user.suspend',
          detail: { reason: 'Chargeback dispute' },
          ip: null,
        },
        'Ada Lovelace',
      ),
    ).toBe('alex@example.com suspended Ada Lovelace: “Chargeback dispute”');
  });

  it('quotes an email subject and names an unlinked provider', () => {
    const base = { id: '1', at: '2026-11-05T13:02:00Z', admin: 'a@b.co', ip: null };
    expect(eventLine({ ...base, action: 'user.email', detail: { subject: 'Hi' } }, 'Ada')).toBe(
      'a@b.co emailed Ada: “Hi”',
    );
    expect(
      eventLine({ ...base, action: 'user.identity.unlink', detail: { provider: 'github' } }, 'Ada'),
    ).toBe('a@b.co unlinked a sign-in method of Ada: GitHub');
  });

  it('says a bulk action included them', () => {
    expect(
      eventLine(
        {
          id: '1',
          at: '2026-11-05T13:02:00Z',
          admin: 'a@b.co',
          action: 'users.bulk.sign_out',
          detail: { ids: ['x'] },
          ip: null,
        },
        'Ada',
      ),
    ).toBe('a@b.co signed out several users: including Ada');
  });

  it('survives a detail it does not know', () => {
    expect(
      eventLine(
        {
          id: '1',
          at: '2026-11-05T13:02:00Z',
          admin: 'a@b.co',
          action: 'user.delete',
          detail: null,
          ip: null,
        },
        'Ada',
      ),
    ).toBe('a@b.co deleted Ada');
  });
});
