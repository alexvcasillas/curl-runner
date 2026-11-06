import { Skeleton } from '@/components/Skeleton';

export default function Loading() {
  return (
    <>
      <span role="status" className="sr-only">
        Loading
      </span>
      <div aria-hidden="true" className="flex flex-col gap-2">
        <Skeleton width="30%" />
        <Skeleton width="50%" height={8} />
      </div>
    </>
  );
}
