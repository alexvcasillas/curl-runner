import type { Metadata } from 'next';
import { BroadcastsView } from '@/components/admin/BroadcastsView';
import { DeliveryLogView } from '@/components/admin/DeliveryLogView';
import { EmailsTabs } from '@/components/admin/EmailsTabs';
import { PageHeader } from '@/components/PageHeader';
import {
  getAdminMe,
  getAudienceCount,
  getBroadcast,
  getBroadcasts,
  getEmailLog,
} from '@/lib/admin-data';
import { parseLogQuery } from '@/lib/admin-emails';
import {
  audienceCount,
  createBroadcast,
  previewBroadcast,
  saveBroadcast,
  sendBroadcast,
  sendTest,
} from './actions';

export const metadata: Metadata = { title: 'Emails' };

type Params = Record<string, string | string[] | undefined>;

export default async function AdminEmailsPage({ searchParams }: { searchParams: Promise<Params> }) {
  const sp = await searchParams;
  // The guard first: a non-admin gets a 404 before any request goes out.
  const admin = await getAdminMe();
  const now = new Date().toISOString();
  const log = sp.tab === 'log';

  return (
    <>
      <PageHeader title="Emails" actions={<EmailsTabs tab={log ? 'log' : 'broadcasts'} />}>
        Write to the people on this server, and see every email the server sent. Product emails skip
        anyone who turned them off. Sign-in codes and invites always go out.
      </PageHeader>
      {log ? (
        <Log sp={sp} now={now} />
      ) : (
        <Broadcasts sp={sp} now={now} permissions={admin.permissions} />
      )}
    </>
  );
}

async function Log({ sp, now }: { sp: Params; now: string }) {
  const query = parseLogQuery(sp);
  const page = await getEmailLog(query);
  return (
    <DeliveryLogView
      page={page}
      q={query.q}
      status={query.status}
      kind={query.kind}
      cursor={query.cursor}
      trail={query.trail}
      now={now}
    />
  );
}

async function Broadcasts({
  sp,
  now,
  permissions,
}: {
  sp: Params;
  now: string;
  permissions: Awaited<ReturnType<typeof getAdminMe>>['permissions'];
}) {
  const items = await getBroadcasts();
  const wanted = typeof sp.b === 'string' ? sp.b : undefined;
  // Older than the list's latest 100: read it on its own.
  const selected =
    (wanted ? (items.find((b) => b.id === wanted) ?? (await getBroadcast(wanted))) : null) ??
    items[0] ??
    null;
  const initialCount = selected ? await getAudienceCount(selected.audience) : null;
  return (
    <BroadcastsView
      items={items}
      selected={selected}
      initialCount={initialCount}
      now={now}
      permissions={permissions}
      actions={{
        create: createBroadcast,
        save: saveBroadcast,
        test: sendTest,
        send: sendBroadcast,
        preview: previewBroadcast,
        count: audienceCount,
      }}
    />
  );
}
