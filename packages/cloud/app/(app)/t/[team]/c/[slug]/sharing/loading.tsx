import { Skeleton } from '@/components/Skeleton';

/** Shown while the share settings load. */
export default function Loading() {
  return (
    <>
      <Skeleton width={160} />
      <Skeleton width={260} height={10} className="h-8" />
      <div className="flex flex-col gap-5" aria-hidden="true">
        <Skeleton width={560} height={10} className="h-40" />
        <Skeleton width={560} height={10} className="h-40" />
      </div>
    </>
  );
}
