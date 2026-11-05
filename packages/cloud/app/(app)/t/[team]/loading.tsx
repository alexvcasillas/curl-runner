import { PageHeader } from '@/components/PageHeader';
import { Table, TableHead, TableHeader, TableLoading } from '@/components/Table';

/** Shown while the collections load: the header, then skeleton rows. */
export default function Loading() {
  return (
    <>
      <PageHeader title="Collections">
        Everything CI or the studio publishes. Each publish makes a new version.
      </PageHeader>
      <Table label="Collections" minWidth={820}>
        <TableHeader>
          <TableHead>Collection</TableHead>
          <TableHead width={96}>Source</TableHead>
          <TableHead width={120}>Access</TableHead>
          <TableHead width={72}>Version</TableHead>
          <TableHead width={220}>Last publish</TableHead>
          <TableHead width={16} />
        </TableHeader>
        <TableLoading />
      </Table>
    </>
  );
}
