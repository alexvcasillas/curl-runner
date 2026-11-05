import { PageHeader } from '@/components/PageHeader';
import { Skeleton } from '@/components/Skeleton';

export default function Loading() {
  return (
    <>
      <PageHeader title="Billing">
        The plan belongs to your team. You pay per member. Customers you share with are always free.
      </PageHeader>
      <Skeleton className="h-[180px] w-full rounded-[8px]" />
    </>
  );
}
