import { Skeleton } from '@/components/Skeleton';
import { Table, TableHead, TableHeader, TableLoading } from '@/components/Table';

/** Shown while a collection or one of its versions loads. */
export default function Loading() {
  return (
    <>
      <Skeleton width={160} />
      <div className="flex flex-col gap-2.5" aria-hidden="true">
        <Skeleton width={260} height={10} className="h-8" />
        <Skeleton width={340} />
      </div>
      <Table label="Loading" minWidth={820}>
        <TableHeader>
          <TableHead>Loading</TableHead>
        </TableHeader>
        <TableLoading />
      </Table>
    </>
  );
}
