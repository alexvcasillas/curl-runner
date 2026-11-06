import type { Broadcast, LogRow, Preview } from '@/lib/admin-emails';

export const NOW = '2026-11-06T12:00:00Z';

export const ID = 'b0000000-0000-4000-8000-000000000001';
export const ID2 = 'b0000000-0000-4000-8000-000000000002';
export const ID3 = 'b0000000-0000-4000-8000-000000000003';

export const draft: Broadcast = {
  id: ID,
  subject: 'The curl-runner Cloud dashboard is live',
  body_md: 'Hi {{first_name}},\n\nYou can now manage {{team_name}}.',
  audience: 'team_owners',
  status: 'draft',
  scheduled_at: null,
  sent_at: null,
  sent_count: 0,
  created_by: { user_id: 'a0000000-0000-4000-8000-000000000001', email: 'ada@example.com' },
  created_at: '2026-11-06T09:00:00Z',
  updated_at: '2026-11-06T09:30:00Z',
  delivered: 0,
  failed: 0,
};

export const sent: Broadcast = {
  ...draft,
  id: ID2,
  subject: 'Welcome to the private preview',
  status: 'sent',
  sent_at: '2026-11-05T09:00:00Z',
  sent_count: 1151,
  delivered: 1148,
  failed: 3,
};

export const scheduled: Broadcast = {
  ...draft,
  id: ID3,
  subject: 'Scheduled maintenance',
  status: 'scheduled',
  scheduled_at: '2026-11-09T10:00:00Z',
};

export const preview: Preview = {
  subject: 'The curl-runner Cloud dashboard is live',
  html: '<html><body><p>Hi Ada,</p><p>You can now manage Acme.</p></body></html>',
  text: 'Hi Ada,\n\nYou can now manage Acme.',
  as_user: {
    id: 'a0000000-0000-4000-8000-000000000001',
    email: 'ada@example.com',
    name: 'Ada Lovelace',
  },
  in_audience: true,
  opted_out: false,
};

export function logRow(over: Partial<LogRow> = {}): LogRow {
  return {
    id: 'c0000000-0000-4000-8000-000000000001',
    to: 'jane@globex.com',
    subject: 'You’re invited',
    kind: 'customer_invite',
    status: 'sent',
    broadcast_id: null,
    created_at: '2026-11-06T13:04:12Z',
    sent_at: '2026-11-06T13:04:14Z',
    failed_at: null,
    next_attempt_at: null,
    attempts: 1,
    last_error: null,
    ...over,
  };
}

export const logRows: LogRow[] = [
  logRow(),
  logRow({
    id: 'c0000000-0000-4000-8000-000000000002',
    to: 'ada@acme.com',
    kind: 'sign_in_code',
    subject: 'Your curl-runner sign-in code',
  }),
  logRow({
    id: 'c0000000-0000-4000-8000-000000000003',
    to: 'old@initech.example',
    kind: 'payment_failed',
    status: 'failed',
    sent_at: null,
    failed_at: '2026-11-06T12:40:03Z',
    attempts: 8,
    last_error: '550 Mailbox does not exist',
  }),
  logRow({
    id: 'c0000000-0000-4000-8000-000000000004',
    to: 'sam@umbrella.dev',
    kind: 'broadcast',
    status: 'queued',
    sent_at: null,
    attempts: 2,
    next_attempt_at: '2026-11-06T12:00:40Z',
    last_error: '421 Try again',
  }),
];
