import Link from 'next/link';
import { routes } from '@/lib/routes';
import { ButtonLink } from './ButtonLink';

/**
 * Why Restricted is locked on a Free team. Billing off (self-hosted) has no
 * plan to buy, so no link. The upgrade dialog replaces this link later.
 */
export function RestrictedUpsell({
  team,
  teamName,
  billing,
}: {
  team: string;
  teamName: string;
  billing: boolean;
}) {
  if (!billing) {
    return <p className="m-0 text-[12px] text-fg-muted">Restricted sharing needs the Team plan.</p>;
  }
  return (
    <p className="m-0 text-[12px] text-fg-muted">
      Restricted sharing needs the Team plan.{' '}
      <Link href={routes.teamTab(team, 'billing')} className="text-blue hover:text-blue-hover">
        Upgrade {teamName}
      </Link>{' '}
      to share only with people you name.
    </p>
  );
}

/**
 * "Upgrade to Team", as a small button to Billing, for a limit that only Team
 * lifts. Billing off (self-hosted) has no plan to buy, so it renders nothing.
 * The upgrade dialog replaces this link later.
 */
export function UpgradeLink({ team, billing }: { team: string; billing: boolean }) {
  if (!billing) {
    return null;
  }
  return (
    <ButtonLink href={routes.teamTab(team, 'billing')} size="sm">
      Upgrade to Team
    </ButtonLink>
  );
}
