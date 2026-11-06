import { PageHeader } from '@/components/PageHeader';
import { Skeleton } from '@/components/Skeleton';

export default function Loading() {
  return (
    <>
      <PageHeader title="Server settings">Changes apply right away and are logged.</PageHeader>
      <div role="status" className="flex flex-col gap-3">
        <span className="sr-only">Loading</span>
        <Skeleton width="40%" />
        <Skeleton width="70%" />
        <Skeleton width="55%" />
      </div>
    </>
  );
}
