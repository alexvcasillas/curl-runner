import { PageHeader } from '@/components/PageHeader';
import { Skeleton } from '@/components/Skeleton';

export default function Loading() {
  return (
    <>
      <PageHeader title="Stripe">Subscription revenue from Stripe webhooks.</PageHeader>
      <div role="status" className="flex flex-col gap-3">
        <span className="sr-only">Loading</span>
        <Skeleton width="40%" />
        <Skeleton width="70%" />
        <Skeleton width="55%" />
      </div>
    </>
  );
}
