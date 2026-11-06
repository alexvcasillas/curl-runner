import type { Metadata } from 'next';
import { UserDetailView } from '@/components/admin/UserDetailView';
import { getAdminMe, getUser } from '@/lib/admin-data';
import {
  deleteUserAndReturn,
  emailUser,
  revokeSession,
  signOutEverywhere,
  suspendUser,
  unlinkIdentity,
  unsuspendUser,
} from '../actions';

export const metadata: Metadata = { title: 'User' };

export default async function AdminUserPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const admin = await getAdminMe();
  const detail = await getUser(id);
  return (
    <UserDetailView
      detail={detail}
      permissions={admin.permissions}
      now={new Date().toISOString()}
      actions={{
        suspend: suspendUser,
        unsuspend: unsuspendUser,
        signOut: signOutEverywhere,
        remove: deleteUserAndReturn,
        email: emailUser,
        revokeSession,
        unlink: unlinkIdentity,
      }}
    />
  );
}
