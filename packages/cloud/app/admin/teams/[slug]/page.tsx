import type { Metadata } from 'next';
import { TeamDetailView } from '@/components/admin/TeamDetailView';
import { getAdminMe, getTeamDetail } from '@/lib/admin-data';
import {
  deleteTeamAndReturn,
  emailOwners,
  grantOverride,
  removeOverride,
  resendInvoice,
  setMemberRole,
  stripeSync,
  suspendTeam,
  unsuspendTeam,
} from '../actions';

export const metadata: Metadata = { title: 'Team' };

export default async function AdminTeamPage({ params }: { params: Promise<{ slug: string }> }) {
  const { slug } = await params;
  const admin = await getAdminMe();
  const detail = await getTeamDetail(slug);
  return (
    <TeamDetailView
      detail={detail}
      permissions={admin.permissions}
      now={new Date().toISOString()}
      actions={{
        grant: grantOverride,
        removeOverride,
        suspend: suspendTeam,
        unsuspend: unsuspendTeam,
        remove: deleteTeamAndReturn,
        setRole: setMemberRole,
        stripeSync,
        resendInvoice,
        emailOwners,
      }}
    />
  );
}
