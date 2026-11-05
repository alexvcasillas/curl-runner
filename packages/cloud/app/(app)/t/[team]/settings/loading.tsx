import { PageHeader } from '@/components/PageHeader';
import { Skeleton } from '@/components/Skeleton';

export default function Loading() {
  return (
    <>
      <PageHeader title="Settings" />
      <span role="status" className="sr-only">
        Loading
      </span>
      <div
        aria-hidden="true"
        className="flex flex-col gap-4 rounded-[8px] border border-hairline p-6"
      >
        <Skeleton width="30%" />
        <Skeleton width="60%" height={8} />
        <Skeleton height={10} />
      </div>
    </>
  );
}
