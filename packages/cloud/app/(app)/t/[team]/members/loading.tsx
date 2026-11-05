import { PageHeader } from '@/components/PageHeader';
import { Table, TableHead, TableHeader, TableLoading } from '@/components/Table';

export default function Loading() {
  return (
    <>
      <PageHeader title="Members">
        Members publish, share collections and make tokens. Owners also manage members and billing.
      </PageHeader>
      <Table label="Team members" minWidth={560}>
        <TableHeader>
          <TableHead>Member</TableHead>
          <TableHead width={150}>Role</TableHead>
          <TableHead width={112} />
        </TableHeader>
        <TableLoading rows={3} />
      </Table>
    </>
  );
}
