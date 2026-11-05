import type { Metadata } from 'next';
import { notFound } from 'next/navigation';
import { BillingView } from '@/components/BillingView';
import { Notice } from '@/components/Notice';
import { PageHeader } from '@/components/PageHeader';
import { getInfo } from '@/lib/app-data';
import { getCollections, requireTeam } from '@/lib/collections-data';
import { getBillingSummary, getPrices } from '@/lib/team-data';
import { openPortal } from './actions';

export const metadata: Metadata = { title: 'Billing' };

export default async function BillingPage({
  params,
  searchParams,
}: {
  params: Promise<{ team: string }>;
  searchParams: Promise<{ checkout?: string | string[] }>;
}) {
  const { team } = await params;
  const { checkout } = await searchParams;
  // Billing off (self-hosted) has no plan to manage: there is no such page.
  const info = await getInfo();
  if (!info.billing) {
    notFound();
  }
  const current = await requireTeam(team);
  const [summary, prices] = await Promise.all([getBillingSummary(team), getPrices()]);

  if (!summary) {
    return (
      <>
        <PageHeader title="Billing" />
        <Notice tone="err" role="alert">
          Billing isn’t reachable right now. Try again in a minute.
        </Notice>
      </>
    );
  }
  const restricted =
    summary.plan === 'team'
      ? (await getCollections(team)).filter((c) => c.access === 'restricted').length
      : 0;

  return (
    <BillingView
      teamName={current.name}
      owner={current.role === 'owner'}
      summary={summary}
      prices={prices}
      restricted={restricted}
      checkout={checkout === 'done' || checkout === 'cancelled' ? checkout : null}
      onPortal={openPortal.bind(null, team)}
    />
  );
}
