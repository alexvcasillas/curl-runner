import { PageHeader } from '@/components/PageHeader';
import { Table, TableHead, TableHeader, TableLoading } from '@/components/Table';

export default function Loading() {
  return (
    <>
      <PageHeader title="Activity log">
        Every change an admin makes, kept for 2 years. Customers’ own actions are on each user’s
        page.
      </PageHeader>
      <Table label="Admin activity" minWidth={760}>
        <TableHeader>
          <TableHead width={104}>Time (UTC)</TableHead>
          <TableHead width={150}>Admin</TableHead>
          <TableHead>Action</TableHead>
          <TableHead width={120}>IP</TableHead>
          <TableHead width={32} />
        </TableHeader>
        <TableLoading rows={6} />
      </Table>
    </>
  );
}
