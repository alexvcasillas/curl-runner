import { PageHeader } from '@/components/PageHeader';
import { Table, TableHead, TableHeader, TableLoading } from '@/components/Table';

export default function Loading() {
  return (
    <>
      <PageHeader title="Collections">
        Every collection on the server. Disable a share link when it’s abused: the team keeps the
        collection, and customers see “This link no longer works”.
      </PageHeader>
      <Table label="Collections" minWidth={980}>
        <TableHeader>
          <TableHead>Collection</TableHead>
          <TableHead width={120}>Team</TableHead>
          <TableHead width={90}>Access</TableHead>
          <TableHead width={64} className="text-right">
            Versions
          </TableHead>
          <TableHead width={64} className="text-right">
            Size
          </TableHead>
          <TableHead width={135} className="text-right">
            Signed-in opens (30d)
          </TableHead>
          <TableHead width={100}>Last publish</TableHead>
          <TableHead width={104}>
            <span className="sr-only">Actions</span>
          </TableHead>
        </TableHeader>
        <TableLoading rows={6} />
      </Table>
    </>
  );
}
