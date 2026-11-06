import type { Metadata } from 'next';
import { OverviewView } from '@/components/admin/OverviewView';
import { getAdminMe, getOverview } from '@/lib/admin-data';

export const metadata: Metadata = { title: 'Admin overview' };

export default async function AdminOverviewPage() {
  // The guard first: a non-admin gets a 404 before any overview request goes out.
  const admin = await getAdminMe();
  const overview = await getOverview();
  return <OverviewView overview={overview} permissions={admin.permissions} />;
}
