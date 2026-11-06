import { describe, expect, it } from 'vitest';
import { can, describeEvent, formatMoney, NAV_GROUPS, navPerm } from '@/lib/admin';

describe('can', () => {
  it('is true only for a held permission', () => {
    expect(can(['billing'], 'billing')).toBe(true);
    expect(can(['billing'], 'users')).toBe(false);
    expect(can([], 'users')).toBe(false);
  });
});

describe('nav', () => {
  it('lists the board groups and items in order', () => {
    expect(NAV_GROUPS.map((g) => [g.label, g.items.map((i) => i.label)])).toEqual([
      ['General', ['Overview']],
      ['Customers', ['Users', 'Teams', 'Collections']],
      ['Operations', ['Emails']],
      ['Billing', ['Stripe']],
      ['Server', ['Settings', 'Health']],
      ['Access', ['Admins', 'Activity log']],
    ]);
  });
  it('maps pages to the permission that edits them', () => {
    const by = Object.fromEntries(
      NAV_GROUPS.flatMap((g) => g.items).map((i) => [i.label, navPerm(i)]),
    );
    expect(by).toMatchObject({
      Users: 'users',
      Teams: 'teams',
      Emails: 'emails',
      Stripe: 'billing',
      Settings: 'settings',
      Admins: 'admins',
      Overview: null,
      Health: null,
      'Activity log': null,
    });
  });
});

describe('describeEvent', () => {
  it('words known actions', () => {
    expect(describeEvent('team.override.grant')).toBe('granted a plan to');
    expect(describeEvent('collection.link.disable')).toBe('disabled the share link of');
  });
  it('falls back to the action in words', () => {
    expect(describeEvent('thing.do_stuff')).toBe('thing do stuff');
  });
});

describe('formatMoney', () => {
  it('shows whole dollars from cents', () => {
    expect(formatMoney(241600)).toBe('$2,416');
    expect(formatMoney(0)).toBe('$0');
  });
});
