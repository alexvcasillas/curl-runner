'use client';

import Link from 'next/link';
import { routes } from '@/lib/routes';
import { Button } from './Button';
import { ButtonLink } from './ButtonLink';
import { type UpgradeReason, useUpgrade } from './UpgradeProvider';

/**
 * Why Restricted is locked on a Free team. Billing off (self-hosted) has no
 * plan to buy, so no link. Billing on opens the upgrade dialog.
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
  const upgrade = useUpgrade();
  if (!billing) {
    return <p className="m-0 text-[12px] text-fg-muted">Restricted sharing needs the Team plan.</p>;
  }
  return (
    <p className="m-0 text-[12px] text-fg-muted">
      Restricted sharing needs the Team plan.{' '}
      {upgrade ? (
        <button
          type="button"
          onClick={() => upgrade('sharing')}
          className="cursor-pointer border-0 bg-transparent p-0 text-[12px] text-blue hover:text-blue-hover"
        >
          Upgrade {teamName}
        </button>
      ) : (
        <Link href={routes.teamTab(team, 'billing')} className="text-blue hover:text-blue-hover">
          Upgrade {teamName}
        </Link>
      )}{' '}
      to share only with people you name.
    </p>
  );
}

/**
 * "Upgrade to Team", as a small button, for a limit that only Team lifts. It
 * opens the upgrade dialog. Billing off (self-hosted) has no plan to buy, so it
 * renders nothing.
 */
export function UpgradeLink({
  team,
  billing,
  reason = 'members',
}: {
  team: string;
  billing: boolean;
  reason?: UpgradeReason;
}) {
  const upgrade = useUpgrade();
  if (!billing) {
    return null;
  }
  if (upgrade) {
    return (
      <Button size="sm" onClick={() => upgrade(reason)}>
        Upgrade to Team
      </Button>
    );
  }
  return (
    <ButtonLink href={routes.teamTab(team, 'billing')} size="sm">
      Upgrade to Team
    </ButtonLink>
  );
}
