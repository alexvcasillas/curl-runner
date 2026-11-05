import { PageHeader } from '@/components/PageHeader';
import { Table, TableHead, TableHeader, TableLoading } from '@/components/Table';

export default function Loading() {
  return (
    <>
      <PageHeader title="Publish tokens">
        CI uses these to publish to your team. A token can publish to any of its collections and
        nothing else.
      </PageHeader>
      <Table label="Tokens" minWidth={820}>
        <TableHeader>
          <TableHead>Name</TableHead>
          <TableHead width={180}>Created by</TableHead>
          <TableHead width={110}>Created</TableHead>
          <TableHead width={140}>Expires</TableHead>
          <TableHead width={120}>Last used</TableHead>
          <TableHead width={72} />
        </TableHeader>
        <TableLoading />
      </Table>
    </>
  );
}
