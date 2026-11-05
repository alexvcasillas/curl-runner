'use client';

import { useEffect, useState } from 'react';
import type { ActionResult } from '@/lib/action-result';
import { dollars, type Interval, monthsFree, perMonth, total } from '@/lib/billing';
import type { Prices } from '@/lib/types';
import { Button } from './Button';
import { CopyField } from './CopyField';
import { Dialog } from './Dialog';
import { Notice } from './Notice';
import { RadioCard } from './RadioCard';
import type { UpgradeReason } from './UpgradeProvider';

const WORDS = ['', 'One', 'Two', 'Three', 'Four', 'Five', 'Six', 'Seven', 'Eight', 'Nine'];

const REASON: Record<UpgradeReason, (team: string) => string> = {
  sharing: () =>
    'Restricted sharing is on the Team plan. Share a collection only with the people you name.',
  members: () => 'The Free plan allows 3 members, counting pending invites. Team has no limit.',
  billing: (team) => `Restricted sharing and unlimited members, for everyone in ${team}.`,
};

const MEMBER_TITLE: Record<UpgradeReason, string> = {
  sharing: 'Restricted sharing needs the Team plan',
  members: 'More members need the Team plan',
  billing: 'Only team owners can upgrade',
};

/** "Ada", "Ada or Grace", "Ada, Grace or Linus". Four or more: "a team owner". */
function names(owners: string[]): string {
  if (owners.length === 0 || owners.length > 3) {
    return 'a team owner';
  }
  if (owners.length === 1) {
    return owners[0];
  }
  return `${owners.slice(0, -1).join(', ')} or ${owners[owners.length - 1]}`;
}

export type UpgradeDialogProps = {
  open: boolean;
  onClose: () => void;
  reason: UpgradeReason;
  initialInterval: Interval;
  team: string;
  teamName: string;
  owner: boolean;
  owners: string[];
  members: number;
  pending: number;
  prices: Prices | null;
  billingUrl: string;
  onCheckout: (interval: Interval) => Promise<ActionResult>;
};

/**
 * "Upgrade Acme to Team": pick Annual or Monthly, see what is due today, then
 * pay on Stripe. A member sees who to ask instead.
 */
export function UpgradeDialog(p: UpgradeDialogProps) {
  const [interval, setInterval] = useState<Interval>(p.initialInterval);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  // A fresh dialog each time it opens.
  useEffect(() => {
    if (p.open) {
      setInterval(p.initialInterval);
      setBusy(false);
      setProblem(null);
    }
  }, [p.open, p.initialInterval]);

  if (!p.owner) {
    return (
      <Dialog
        open={p.open}
        onClose={p.onClose}
        title={MEMBER_TITLE[p.reason]}
        description={`Only team owners can change ${p.teamName}’s plan.`}
        className="max-w-[520px]"
        footer={
          <Button variant="primary" onClick={p.onClose}>
            Got it
          </Button>
        }
      >
        {p.open ? (
          <>
            <Notice>Ask {names(p.owners)} to upgrade. Send them the billing page:</Notice>
            <CopyField value={p.billingUrl} label="billing page link" />
          </>
        ) : null}
      </Dialog>
    );
  }

  async function go() {
    setBusy(true);
    setProblem(null);
    const result = await p.onCheckout(interval);
    // A good answer redirects to Stripe and never gets here.
    if (!result.ok) {
      setProblem(result.message);
      setBusy(false);
    }
  }

  const { prices } = p;
  const free = prices ? monthsFree(prices) : 0;
  const freeNote =
    free > 0 ? ` ${WORDS[free] ?? free} ${free === 1 ? 'month' : 'months'} free.` : '';
  return (
    <Dialog
      open={p.open}
      onClose={p.onClose}
      title={`Upgrade ${p.teamName} to Team`}
      description={p.open ? REASON[p.reason](p.teamName) : undefined}
      className="max-w-[520px]"
      footer={
        <>
          <Button onClick={p.onClose} disabled={busy}>
            Not now
          </Button>
          <Button variant="primary" onClick={go} loading={busy} disabled={!prices}>
            {busy ? 'Opening Stripe…' : 'Continue to payment'}
          </Button>
        </>
      }
    >
      {p.open ? (
        <>
          {prices ? (
            <>
              <fieldset className="m-0 flex min-w-0 flex-col gap-2 border-0 p-0">
                <legend className="sr-only">Billing interval</legend>
                <RadioCard
                  name="upgrade-interval"
                  value="annual"
                  checked={interval === 'annual'}
                  onChange={() => setInterval('annual')}
                  title={`Annual · ${perMonth(prices, 'annual')} per member a month`}
                  description={`Billed ${dollars(prices.annual.unit_amount, prices.annual.currency)} per member a year.${freeNote}`}
                />
                <RadioCard
                  name="upgrade-interval"
                  value="monthly"
                  checked={interval === 'monthly'}
                  onChange={() => setInterval('monthly')}
                  title={`Monthly · ${perMonth(prices, 'monthly')} per member a month`}
                  description="Cancel any month."
                />
              </fieldset>
              <dl className="m-0 flex flex-col gap-1.5 rounded-[8px] border border-hairline bg-bg px-3.5 py-3">
                <Row label="Members today" value={String(p.members)} />
                <Row label="Pending invites" value={`${p.pending}, billed when accepted`} />
                <Row
                  label="Due today, before tax"
                  value={total(prices, interval, p.members)}
                  strong
                />
              </dl>
            </>
          ) : (
            <Notice tone="err" role="alert">
              Prices aren’t available right now. Try again in a minute.
            </Notice>
          )}
          {problem ? (
            <Notice tone="err" role="alert">
              {problem}
            </Notice>
          ) : null}
          <p className="m-0 text-[12px] text-fg-subtle">
            You pay on a secure Stripe page and come straight back here. Stripe adds tax for your
            country and accepts a business tax ID. Customers you share with are never seats.
          </p>
        </>
      ) : null}
    </Dialog>
  );
}

function Row({ label, value, strong }: { label: string; value: string; strong?: boolean }) {
  return (
    <div
      className={
        strong
          ? 'mt-0.5 flex justify-between gap-3 border-t border-hairline pt-1.5 text-fg'
          : 'flex justify-between gap-3 text-fg-muted'
      }
    >
      <dt>{label}</dt>
      <dd className="m-0 font-medium tabular-nums text-fg">{value}</dd>
    </div>
  );
}
