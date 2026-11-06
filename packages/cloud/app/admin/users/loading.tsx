import { PageHeader } from '@/components/PageHeader';
import { Table, TableHead, TableHeader, TableLoading } from '@/components/Table';

export default function Loading() {
  return (
    <>
      <PageHeader title="Users">
        Everyone with an account: team members and customers who signed in to open a restricted
        link. Every action here is logged.
      </PageHeader>
      <Table label="Users" minWidth={900}>
        <TableHeader>
          <TableHead>User</TableHead>
          <TableHead width={120}>Sign-in</TableHead>
          <TableHead width={150}>Teams</TableHead>
          <TableHead width={90} className="text-right">
            Shared in
          </TableHead>
          <TableHead width={110}>Last active</TableHead>
          <TableHead width={90}>Status</TableHead>
          <TableHead width={28} />
        </TableHeader>
        <TableLoading rows={6} />
      </Table>
    </>
  );
}
