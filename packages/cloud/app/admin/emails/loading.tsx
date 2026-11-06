import { EmailsTabs } from '@/components/admin/EmailsTabs';
import { PageHeader } from '@/components/PageHeader';
import { Skeleton } from '@/components/Skeleton';

export default function Loading() {
  return (
    <>
      <PageHeader title="Emails" actions={<EmailsTabs tab="broadcasts" />}>
        Write to the people on this server, and see every email the server sent. Product emails skip
        anyone who turned them off. Sign-in codes and invites always go out.
      </PageHeader>
      <div role="status" className="flex flex-col gap-3">
        <span className="sr-only">Loading</span>
        <Skeleton width="40%" />
        <Skeleton width="70%" />
        <Skeleton width="55%" />
      </div>
    </>
  );
}
