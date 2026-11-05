'use client';

import { createContext, useCallback, useContext, useMemo, useState } from 'react';
import type { ActionResult } from '@/lib/action-result';
import type { Interval } from '@/lib/billing';
import type { Prices } from '@/lib/types';
import { UpgradeDialog } from './UpgradeDialog';

/** What a Team-only limit says when it opens the dialog. */
export type UpgradeReason = 'sharing' | 'members' | 'billing';

type Open = (reason: UpgradeReason, interval?: Interval) => void;

const UpgradeContext = createContext<Open | null>(null);

/** Opens the upgrade dialog, or null outside a team page (no dialog to open). */
export function useUpgrade(): Open | null {
  return useContext(UpgradeContext);
}

export type UpgradeProviderProps = {
  team: string;
  teamName: string;
  /** Owners can buy; everyone else sees who to ask. */
  owner: boolean;
  /** Names (or emails) of the team's owners, for "Ask … to upgrade". */
  owners: string[];
  /** Accepted members, which is what Stripe bills. */
  members: number;
  pending: number;
  /** From `/v1/billing/prices`. Null when it couldn't be read. */
  prices: Prices | null;
  /** The absolute URL of this team's Billing page, to send to an owner. */
  billingUrl: string;
  onCheckout: (interval: Interval) => Promise<ActionResult>;
  children: React.ReactNode;
};

/**
 * One upgrade dialog for the whole team layout. Every Team-only limit (Sharing's
 * Restricted, the 4th member, Billing, the banner) opens it through `useUpgrade`.
 */
export function UpgradeProvider({ children, ...dialog }: UpgradeProviderProps) {
  const [state, setState] = useState<{ reason: UpgradeReason; interval: Interval } | null>(null);
  const open = useCallback<Open>(
    (reason, interval = 'annual') => setState({ reason, interval }),
    [],
  );
  const value = useMemo(() => open, [open]);
  return (
    <UpgradeContext.Provider value={value}>
      {children}
      <UpgradeDialog
        {...dialog}
        open={state !== null}
        reason={state?.reason ?? 'billing'}
        initialInterval={state?.interval ?? 'annual'}
        onClose={() => setState(null)}
      />
    </UpgradeContext.Provider>
  );
}
