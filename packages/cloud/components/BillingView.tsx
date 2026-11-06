'use client';

import { useState } from 'react';
import type { ActionResult } from '@/lib/action-result';
import { billedNote, dueBy as dueDate, type Interval, perMonth, total } from '@/lib/billing';
import { formatDate } from '@/lib/time';
import type { BillingSummary, Prices } from '@/lib/types';
import { Banner } from './Banner';
import { Button } from './Button';
import { Card, CardBody, CardFooter, CardTitle } from './Card';
import { CheckoutStatus } from './CheckoutStatus';
import { CheckIcon } from './icons';
import { Notice } from './Notice';
import { PageHeader } from './PageHeader';
import { Pill } from './Pill';
import { Segmented } from './Segmented';
import { useUpgrade } from './UpgradeProvider';

export type BillingViewProps = {
  teamName: string;
  /** Owners buy and manage; everyone else reads. */
  owner: boolean;
  summary: BillingSummary;
  /** From `/v1/billing/prices`. Null when it couldn't be read. */
  prices: Prices | null;
  /** Restricted collections the team has. */
  restricted: number;
  /** `?checkout=` on the way back from Stripe. */
  checkout: 'done' | 'cancelled' | null;
  onPortal: () => Promise<ActionResult>;
};

const FREE_FEATURES = [
  'Up to 3 members',
  'Unlimited public collections',
  'Publishing from CI and the studio',
];
const TEAM_FEATURES = [
  'Everything in Free',
  'Restricted collections, shared only with people you name',
  'Unlimited members. Customers are never seats.',
];

type Mode = 'free' | 'active' | 'scheduled' | 'past_due' | 'granted';

function modeOf(s: BillingSummary): Mode {
  if (s.plan === 'free') {
    return 'free';
  }
  if (s.plan_source === 'override') {
    return 'granted';
  }
  if (s.past_due_since) {
    return 'past_due';
  }
  return s.cancel_at ? 'scheduled' : 'active';
}

/** The Billing page: the plan, what it costs, and the way to change it. */
export function BillingView(p: BillingViewProps) {
  const { summary: s, prices, teamName } = p;
  const upgrade = useUpgrade();
  const [interval, setPeriod] = useState<Interval>('annual');
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const mode = modeOf(s);
  const sub: Interval = s.interval === 'monthly' ? 'monthly' : 'annual';
  const canPortal = p.owner && s.plan_source === 'stripe' && mode !== 'free' && mode !== 'granted';

  async function portal() {
    setBusy(true);
    setProblem(null);
    const r = await p.onPortal();
    // A good answer redirects to Stripe and never gets here.
    if (!r.ok) {
      setProblem(r.message);
      setBusy(false);
    }
  }

  const ends = s.cancel_at ? formatDate(s.cancel_at) : null;
  // A grant's end is its own date, never Stripe's cancel date.
  const grantEnd = s.override_until ? `until ${formatDate(s.override_until)}` : 'with no end date';
  const status: { word: string; tone: 'neutral' | 'ok' | 'err' } = {
    free: { word: 'Free', tone: 'neutral' as const },
    active: { word: 'Active', tone: 'ok' as const },
    scheduled: { word: `Ends ${ends}`, tone: 'neutral' as const },
    past_due: { word: 'Past due', tone: 'err' as const },
    granted: { word: 'Granted', tone: 'neutral' as const },
  }[mode];

  const renews = s.current_period_end ? ` Renews on ${formatDate(s.current_period_end)}.` : '';
  const line = {
    free: '3 members, counting pending invites. Public collections and publishing are unlimited.',
    granted: `curl-runner gave ${teamName} the Team plan ${grantEnd}. There’s nothing to pay. Questions? Reply to your welcome email.`,
    active: `Billed ${sub === 'annual' ? 'annually' : 'monthly'}.${renews} Adding or removing a member changes the next invoice, prorated.`,
    scheduled: `Billed ${sub === 'annual' ? 'annually' : 'monthly'}. Adding or removing a member changes the next invoice, prorated.`,
    past_due: `Billed ${sub === 'annual' ? 'annually' : 'monthly'}.${renews} Adding or removing a member changes the next invoice, prorated.`,
  }[mode];

  const cost =
    mode === 'free' || mode === 'granted'
      ? '$0'
      : mode === 'scheduled'
        ? 'None'
        : prices
          ? total(prices, sub, s.seats)
          : 'Unavailable';

  const portalNote = `Cards, invoices, switching to ${sub === 'annual' ? 'monthly' : 'annual'} and cancelling are in the Stripe portal.`;

  const foot = {
    free: `Upgrading starts today. The first invoice covers the members ${teamName} has now.`,
    granted: s.override_until
      ? `When the grant ends, ${teamName} moves to Free unless an owner upgrades. Owners get an email 14 days before.`
      : `The grant has no end date. If it ever ends, ${teamName} moves to Free unless an owner upgrades.`,
    active: portalNote,
    scheduled: portalNote,
    past_due: portalNote,
  }[mode];

  return (
    <>
      <PageHeader title="Billing">
        The plan belongs to {teamName}. You pay per member. Customers you share with are always
        free.
      </PageHeader>

      <CheckoutStatus plan={s.plan} checkout={p.checkout} teamName={teamName} />

      {mode === 'scheduled' ? (
        <Notice tone="info" role="status" className="px-4 py-3 text-[13px] leading-5">
          <span className="flex flex-wrap items-center gap-3">
            <span className="min-w-[200px] flex-1">
              The Team plan is cancelled and ends on {ends}. After that {teamName} moves to Free,
              and nothing is deleted.
            </span>
            {canPortal ? (
              <Button size="sm" onClick={portal} loading={busy}>
                Renew plan
              </Button>
            ) : null}
          </span>
        </Notice>
      ) : null}
      {mode === 'past_due' && s.past_due_since ? (
        <Banner
          tone="err"
          title="We couldn’t take the last payment."
          action={
            canPortal ? (
              <Button size="sm" onClick={portal} loading={busy}>
                Update card
              </Button>
            ) : null
          }
        >
          Stripe retries the card. Update it by {dueDate(s.past_due_since)} to keep the Team plan.
          After that {teamName} moves to Free, and nothing is deleted.
        </Banner>
      ) : null}
      {problem ? (
        <Notice tone="err" role="alert">
          {problem}
        </Notice>
      ) : null}

      <Card aria-labelledby="cur">
        <CardBody className="flex-row flex-wrap items-center gap-3">
          <div className="flex min-w-[240px] flex-1 flex-col gap-1">
            <div className="flex items-center gap-2">
              <CardTitle id="cur">{mode === 'free' ? 'Free plan' : 'Team plan'}</CardTitle>
              <Pill tone={status.tone}>{status.word}</Pill>
            </div>
            <p className="m-0 leading-normal text-fg-muted">{line}</p>
          </div>
          {canPortal ? (
            <Button onClick={portal} loading={busy}>
              Manage billing
            </Button>
          ) : null}
        </CardBody>
        <div className="flex flex-wrap border-t border-hairline">
          <Stat label="Members" value={mode === 'free' ? `${s.seats} of 3` : String(s.seats)} />
          <Stat
            label="Restricted collections"
            value={mode === 'free' ? 'Team only' : String(p.restricted)}
          />
          <Stat
            label={mode === 'free' || mode === 'granted' ? 'Cost' : 'Next invoice'}
            value={cost}
            last
          />
        </div>
        <CardFooter>{foot}</CardFooter>
      </Card>

      {mode === 'free' ? (
        <section aria-labelledby="plans" className="flex flex-col gap-4">
          <div className="flex flex-wrap items-center gap-3">
            <h2 id="plans" className="m-0 mr-auto text-[16px] leading-6 font-semibold">
              Plans
            </h2>
            <Segmented
              label="Billing interval"
              value={interval}
              onChange={setPeriod}
              options={[
                { value: 'annual', label: 'Annual' },
                { value: 'monthly', label: 'Monthly' },
              ]}
            />
          </div>
          <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,300px),1fr))] gap-4">
            <Card className="flex flex-col">
              <CardBody className="flex-1 gap-4">
                <div className="flex items-center gap-2">
                  <h3 className="m-0 text-[14px] font-semibold">Free</h3>
                  <Pill>Current plan</Pill>
                </div>
                <span className="text-[32px] leading-10 font-semibold">$0</span>
                <Features items={FREE_FEATURES} />
              </CardBody>
            </Card>
            <Card className="flex flex-col border-fg">
              <CardBody className="flex-1 gap-4">
                <h3 className="m-0 text-[14px] font-semibold">Team</h3>
                {prices ? (
                  <div className="flex flex-wrap items-baseline gap-x-1.5">
                    <span className="text-[32px] leading-10 font-semibold">
                      {perMonth(prices, interval)}
                    </span>
                    <span className="text-fg-muted">per member a month</span>
                    <span className="text-[12px] text-fg-subtle">
                      {billedNote(prices, interval)}
                    </span>
                  </div>
                ) : (
                  <span className="text-fg-muted">
                    Prices aren’t available right now. Try again in a minute.
                  </span>
                )}
                <Features items={TEAM_FEATURES} />
              </CardBody>
              <CardFooter className="justify-between">
                <span>
                  {prices
                    ? `For ${teamName} today: ${s.seats} ${s.seats === 1 ? 'member' : 'members'}, ${total(prices, interval, s.seats)}.`
                    : ''}
                </span>
                {p.owner ? (
                  <Button
                    variant="primary"
                    disabled={!prices}
                    onClick={() => upgrade?.('billing', interval)}
                  >
                    Upgrade to Team
                  </Button>
                ) : (
                  <span className="text-fg">Ask a team owner to upgrade.</span>
                )}
              </CardFooter>
            </Card>
          </div>
          {prices ? (
            <p className="m-0 text-[12px] text-fg-subtle">
              Prices in {prices.annual.currency.toUpperCase()}, before tax. Checkout runs on Stripe
              and accepts business tax IDs.
            </p>
          ) : null}
        </section>
      ) : null}
    </>
  );
}

function Stat({ label, value, last }: { label: string; value: string; last?: boolean }) {
  return (
    <div
      className={`flex min-w-[160px] flex-1 flex-col gap-0.5 px-6 py-4 max-sm:px-4 ${last ? '' : 'border-r border-hairline'}`}
    >
      <span className="text-[12px] text-fg-subtle">{label}</span>
      <span className="text-[20px] leading-7 font-semibold tabular-nums">{value}</span>
    </div>
  );
}

function Features({ items }: { items: string[] }) {
  return (
    <ul className="m-0 flex list-none flex-col gap-2 p-0">
      {items.map((t) => (
        <li key={t} className="flex items-start gap-2 text-fg-muted">
          <CheckIcon size={13} className="mt-[3px] shrink-0 text-fg" />
          {t}
        </li>
      ))}
    </ul>
  );
}
