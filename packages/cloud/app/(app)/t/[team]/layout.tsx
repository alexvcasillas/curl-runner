import { Banner } from '@/components/Banner';
import { PlanBanner } from '@/components/PlanBanner';
import { UpgradeProvider } from '@/components/UpgradeProvider';
import { getInfo } from '@/lib/app-data';
import { dueBy, planBanner } from '@/lib/billing';
import { getCollections } from '@/lib/collections-data';
import { env } from '@/lib/env';
import { ApiError } from '@/lib/errors';
import { routes } from '@/lib/routes';
import { getBillingSummary, getPrices, getTeam } from '@/lib/team-data';
import type { TeamDetail } from '@/lib/types';
import { openPortal, startCheckout } from './billing/actions';

/**
 * Wraps every team page. With billing on it adds the plan banner and the one
 * upgrade dialog that every Team-only limit opens. With billing off it is
 * transparent: no plan to talk about, so nothing to fetch.
 */
export default async function TeamLayout({
  children,
  params,
}: {
  children: React.ReactNode;
  params: Promise<{ team: string }>;
}) {
  const { team } = await params;
  const info = await getInfo();
  let detail: TeamDetail;
  try {
    detail = await getTeam(team);
  } catch (e) {
    // The page itself says what's wrong (a 404 for a team that isn't yours).
    if (e instanceof ApiError) {
      return children;
    }
    throw e;
  }
  // Members see that the team is suspended, never why: the reason stays with the admins.
  const suspended = detail.suspended ? (
    <Banner tone="err" title={`${detail.name} is suspended.`}>
      Publishing is off and share links show “no longer works”. Contact support.
    </Banner>
  ) : null;
  if (!info.billing) {
    return (
      <>
        {suspended}
        {children}
      </>
    );
  }

  try {
    const summary = await getBillingSummary(team);
    if (!summary) {
      return (
        <>
          {suspended}
          {children}
        </>
      );
    }
    const free = summary.plan === 'free';
    const [collections, prices] = free
      ? await Promise.all([getCollections(team), getPrices()])
      : [[], null];
    const kind = planBanner({
      billing: true,
      summary,
      restricted: collections.filter((c) => c.access === 'restricted').length,
      members: summary.seats,
    });
    const banner = kind ? (
      <PlanBanner
        kind={kind}
        teamName={detail.name}
        dueBy={summary.past_due_since ? dueBy(summary.past_due_since) : null}
        canManage={summary.can_manage}
        onPortal={openPortal.bind(null, team)}
      />
    ) : null;

    if (!free) {
      return (
        <>
          {suspended}
          {banner}
          {children}
        </>
      );
    }
    return (
      <UpgradeProvider
        team={team}
        teamName={detail.name}
        owner={detail.role === 'owner'}
        owners={detail.members.filter((m) => m.role === 'owner').map((m) => m.name || m.email)}
        members={summary.seats}
        pending={detail.pending_invites.length}
        prices={prices}
        billingUrl={`${env().PUBLIC_APP_URL}${routes.teamTab(team, 'billing')}`}
        onCheckout={startCheckout.bind(null, team)}
      >
        {suspended}
        {banner}
        {children}
      </UpgradeProvider>
    );
  } catch (e) {
    // The page itself says what's wrong (a 404 for a team that isn't yours).
    if (e instanceof ApiError) {
      return children;
    }
    throw e;
  }
}
