import { PageHeader } from '@/components/PageHeader';
import { Table, TableHead, TableHeader, TableLoading } from '@/components/Table';

export default function Loading() {
  return (
    <>
      <PageHeader title="Teams">
        Every team, where its plan comes from, and what it pays.
      </PageHeader>
      <Table label="Teams" minWidth={1060}>
        <TableHeader>
          <TableHead>Team</TableHead>
          <TableHead width={230}>Owner</TableHead>
          <TableHead width={70} className="text-right">
            Members
          </TableHead>
          <TableHead width={90} className="text-right">
            Collections
          </TableHead>
          <TableHead width={70}>Plan</TableHead>
          <TableHead width={210}>Source</TableHead>
          <TableHead width={80} className="text-right">
            MRR
          </TableHead>
        </TableHeader>
        <TableLoading rows={6} />
      </Table>
    </>
  );
}
