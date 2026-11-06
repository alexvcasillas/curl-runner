import type { Metadata } from 'next';
import { HealthView } from '@/components/admin/HealthView';
import { getAdminMe, getHealth } from '@/lib/admin-data';

export const metadata: Metadata = { title: 'Health' };

// The numbers are live: never cache the page.
export const dynamic = 'force-dynamic';

export default async function AdminHealthPage() {
  await getAdminMe();
  const health = await getHealth();
  return <HealthView health={health} now={new Date().toISOString()} />;
}
