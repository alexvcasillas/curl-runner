import type { Metadata } from 'next';
import { UsersView } from '@/components/admin/UsersView';
import { getAdminMe, getUsers, USERS_PAGE_SIZE } from '@/lib/admin-data';
import { parseUsersQuery } from '@/lib/admin-users';
import {
  bulkUsers,
  deleteUser,
  emailUser,
  signOutEverywhere,
  suspendUser,
  unsuspendUser,
} from './actions';

export const metadata: Metadata = { title: 'Users' };

export default async function AdminUsersPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | string[] | undefined>>;
}) {
  const query = parseUsersQuery(await searchParams);
  // The guard first: a non-admin gets a 404 before any list request goes out.
  const admin = await getAdminMe();
  const page = await getUsers(query);
  return (
    <UsersView
      page={page}
      q={query.q}
      filter={query.filter}
      cursor={query.cursor}
      trail={query.trail}
      pageSize={USERS_PAGE_SIZE}
      permissions={admin.permissions}
      now={new Date().toISOString()}
      actions={{
        suspend: suspendUser,
        unsuspend: unsuspendUser,
        signOut: signOutEverywhere,
        remove: deleteUser,
        email: emailUser,
        bulk: bulkUsers,
      }}
    />
  );
}
