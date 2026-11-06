import type { Metadata } from 'next';
import { OverviewView } from '@/components/admin/OverviewView';
import { getAdminMe, getOverview } from '@/lib/admin-data';

export const metadata: Metadata = { title: 'Admin overview' };

export default async function AdminOverviewPage() {
  const [admin, overview] = await Promise.all([getAdminMe(), getOverview()]);
  return <OverviewView overview={overview} permissions={admin.permissions} />;
}
