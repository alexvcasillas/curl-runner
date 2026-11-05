'use client';

import { usePathname } from 'next/navigation';
import { useState } from 'react';
import type { ActionResult } from '@/lib/action-result';
import type { BannerKind } from '@/lib/billing';
import { Banner } from './Banner';
import { Button } from './Button';
import { useUpgrade } from './UpgradeProvider';

/**
 * Sits at the top of every team page until the cause is gone: a failed payment,
 * or a Free team still holding Team-only things.
 */
export function PlanBanner({
  kind,
  teamName,
  dueBy,
  canManage,
  onPortal,
}: {
  kind: BannerKind;
  teamName: string;
  /** "Nov 19": the last day to fix the card. */
  dueBy: string | null;
  /** Owners can open the portal. */
  canManage: boolean;
  onPortal: () => Promise<ActionResult>;
}) {
  const upgrade = useUpgrade();
  const pathname = usePathname();
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  // The Billing page says all of this itself, with the same buttons.
  if (/\/billing\/?$/.test(pathname)) {
    return null;
  }
  if (kind === 'past_due') {
    return (
      <Banner
        tone="err"
        title={`We couldn’t take ${teamName}’s last payment.`}
        action={
          canManage ? (
            <Button
              size="sm"
              loading={busy}
              onClick={async () => {
                setBusy(true);
                setProblem(null);
                const r = await onPortal();
                if (!r.ok) {
                  setProblem(r.message);
                  setBusy(false);
                }
              }}
            >
              {busy ? 'Opening Stripe…' : 'Update card'}
            </Button>
          ) : null
        }
      >
        Update the card{dueBy ? ` by ${dueBy}` : ''} to keep the Team plan. After that, {teamName}{' '}
        moves to Free and nothing is deleted.
        {problem ? <span className="block text-danger">{problem}</span> : null}
      </Banner>
    );
  }
  return (
    <Banner
      tone="warn"
      title={`${teamName} is on the Free plan now. Nothing was deleted.`}
      action={
        upgrade ? (
          <Button size="sm" onClick={() => upgrade('billing')}>
            Upgrade to Team
          </Button>
        ) : null
      }
    >
      Restricted collections keep serving the people already on their lists. To publish to them, add
      people, or add members past 3, upgrade or make those collections public or off.
    </Banner>
  );
}
