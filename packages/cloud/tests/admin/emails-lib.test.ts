import { describe, expect, it } from 'vitest';
import {
  audienceOf,
  type Broadcast,
  broadcastMeta,
  logDetail,
  logHref,
  logWhen,
  parseLogQuery,
  peopleLabel,
  scheduleInstant,
  sendLabel,
} from '@/lib/admin-emails';
import { logRow } from './emails-fixtures';

const NOW = new Date('2026-11-06T12:00:00Z');

describe('audiences', () => {
  it('knows the five and falls back to everyone', () => {
    expect(audienceOf('team_owners').label).toBe('Team owners');
    expect(audienceOf('nonsense').value).toBe('everyone');
  });
});

describe('labels', () => {
  it('counts people', () => {
    expect(peopleLabel(1)).toBe('1 person');
    expect(peopleLabel(1248)).toBe('1,248 people');
    expect(sendLabel(214)).toBe('Send to 214 people');
    expect(sendLabel(null)).toBe('Send');
  });

  it('summarises a broadcast for the list', () => {
    const b = {
      audience: 'team_owners',
      status: 'draft',
      scheduled_at: null,
      sent_at: null,
      sent_count: 0,
      delivered: 0,
      failed: 0,
    } as Broadcast;
    expect(broadcastMeta(b, NOW)).toBe('Draft · Team owners');
    expect(
      broadcastMeta({ ...b, status: 'scheduled', scheduled_at: '2026-11-09T10:00:00Z' }, NOW),
    ).toBe('Scheduled Nov 9, 10:00 UTC · Team owners');
    expect(broadcastMeta({ ...b, status: 'sending', sent_count: 12 }, NOW)).toBe(
      'Sending · 12 emails queued',
    );
    expect(
      broadcastMeta(
        { ...b, status: 'sent', sent_at: '2026-11-05T09:00:00Z', delivered: 1148, failed: 3 },
        NOW,
      ),
    ).toBe('Sent Nov 5 · 1,148 delivered · 3 failed');
    expect(
      broadcastMeta({ ...b, status: 'sent', sent_at: '2026-11-05T09:00:00Z', delivered: 5 }, NOW),
    ).toBe('Sent Nov 5 · 5 delivered');
  });
});

describe('scheduleInstant', () => {
  it('turns a local time into the UTC instant the server stores', () => {
    const r = scheduleInstant('2026-11-09T10:00', NOW);
    expect(r).toEqual({ ok: true, iso: new Date('2026-11-09T10:00').toISOString() });
  });

  it('rejects the past, now and junk', () => {
    expect(scheduleInstant('2020-01-01T10:00', NOW)).toEqual({
      ok: false,
      message: 'Pick a time in the future.',
    });
    expect(scheduleInstant('', NOW)).toMatchObject({
      ok: false,
      message: 'Pick a date and a time.',
    });
    expect(scheduleInstant('2026-13-45T99:00', NOW)).toMatchObject({ ok: false });
  });
});

describe('the delivery log query', () => {
  it('checks status and kind', () => {
    expect(parseLogQuery({ status: 'failed', kind: 'broadcast', q: ' ada ' })).toMatchObject({
      status: 'failed',
      kind: 'broadcast',
      q: 'ada',
    });
    expect(parseLogQuery({ status: 'x', kind: 'y' })).toMatchObject({ status: 'all', kind: 'all' });
  });

  it('builds the tab url', () => {
    expect(logHref({ q: '', status: 'all', kind: 'all', trail: [] })).toBe('/admin/emails?tab=log');
    expect(
      logHref({ q: 'a', status: 'failed', kind: 'broadcast', cursor: 'c', trail: ['-'] }),
    ).toBe('/admin/emails?tab=log&q=a&status=failed&kind=broadcast&cursor=c&trail=-');
  });

  it('says what happened to a row', () => {
    expect(logDetail(logRow({ status: 'sent' }), NOW)).toBe('Sent');
    expect(logDetail(logRow({ status: 'sent', attempts: 3 }), NOW)).toBe('Sent on attempt 3');
    expect(
      logDetail(logRow({ status: 'failed', last_error: '550 Mailbox does not exist' }), NOW),
    ).toBe('550 Mailbox does not exist');
    expect(logDetail(logRow({ status: 'failed', last_error: null }), NOW)).toBe(
      'Gave up after 8 attempts',
    );
    expect(logDetail(logRow({ status: 'queued', attempts: 0 }), NOW)).toBe('Waiting to send');
    expect(
      logDetail(
        logRow({ status: 'queued', attempts: 2, next_attempt_at: '2026-11-06T12:00:40Z' }),
        NOW,
      ),
    ).toBe('Attempt 2 of 8 failed · next try in 40 s');
  });

  it('prints times in UTC', () => {
    expect(logWhen('2026-11-06T13:04:12Z', NOW)).toBe('13:04:12');
    expect(logWhen('2026-11-05T13:04:12Z', NOW)).toBe('Nov 5, 13:04');
    expect(logWhen(null, NOW)).toBe('–');
  });
});
