import type { Metadata } from 'next';
import { AdminsView } from '@/components/admin/AdminsView';
import { getAdminMe, getAdmins } from '@/lib/admin-data';
import { addAdmin, removeAdmin, updateAdmin } from './actions';

export const metadata: Metadata = { title: 'Admins' };

export default async function AdminAdminsPage() {
  // The guard first: a non-admin gets a 404 before any request goes out.
  const me = await getAdminMe();
  const admins = await getAdmins();
  return (
    <AdminsView
      admins={admins}
      permissions={me.permissions}
      now={new Date().toISOString()}
      actions={{ add: addAdmin, update: updateAdmin, remove: removeAdmin }}
    />
  );
}
