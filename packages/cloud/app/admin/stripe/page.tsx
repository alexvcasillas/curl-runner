import type { Metadata } from 'next';
import { StripeView } from '@/components/admin/StripeView';
import { getAdminMe, getOverview, getStripeStats } from '@/lib/admin-data';

export const metadata: Metadata = { title: 'Stripe' };

export default async function AdminStripePage() {
  // The guard first: a non-admin gets a 404 before any request goes out.
  await getAdminMe();
  const [stats, overview] = await Promise.all([getStripeStats(), getOverview()]);
  return (
    <StripeView
      stats={stats}
      granted={{ teams: overview.counts.granted_teams, seats: overview.counts.granted_seats }}
    />
  );
}
