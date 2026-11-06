'use client';

import Link from 'next/link';
import { useState } from 'react';
import type { ActionResult } from '@/lib/action-result';
import { adminRoutes, can, formatInt, type Perm } from '@/lib/admin';
import {
  headerSource,
  historyLine,
  limitsLabel,
  type OverrideInput,
  planName,
  stripeUrl,
  type TeamDetail,
  type TeamRole,
} from '@/lib/admin-teams';
import { dollars } from '@/lib/billing';
import { initials, teamLetter } from '@/lib/initials';
import { formatDate, shortDate } from '@/lib/time';
import { Banner } from '../Banner';
import { Breadcrumb } from '../Breadcrumb';
import { Button } from '../Button';
import { Card } from '../Card';
import { ConfirmDialog } from '../ConfirmDialog';
import { Notice } from '../Notice';
import { Pill } from '../Pill';
import { Select } from '../Select';
import { EmailUserDialog } from './EmailUserDialog';
import { OverrideForm } from './OverrideForm';
import { SuspendDialog } from './SuspendDialog';
import type { TeamDetailActions } from './team-actions';

const READ_ONLY_BILLING = 'Read only: you need the billing permission.';
const READ_ONLY_TEAMS = 'Read only: you need the teams permission.';
const READ_ONLY_EMAIL = 'Read only: you need the emails permission.';
const ROW =
  'flex min-h-[52px] items-center gap-3 border-b border-hairline px-4 py-2 last:border-b-0';
const HISTORY_SHOWN = 10;

type Dialog = { kind: 'suspend' | 'unsuspend' | 'delete' | 'email' | 'remove-override' };

const plural = (n: number, one: string) => `${n} ${n === 1 ? one : `${one}s`}`;
const article = (role: TeamRole) => (role === 'owner' ? 'an owner' : 'a member');

function CardHeader({
  id,
  title,
  children,
}: {
  id: string;
  title: string;
  children?: React.ReactNode;
}) {
  return (
    <div className="flex min-h-11 flex-wrap items-center gap-2 border-b border-hairline px-4">
      <h2 id={id} className="m-0 mr-auto text-[14px] font-semibold">
        {title}
      </h2>
      {children}
    </div>
  );
}

function Empty({ children }: { children: React.ReactNode }) {
  return <p className="m-0 px-4 py-3 text-fg-subtle">{children}</p>;
}

/** "Nov 5 13:02", UTC, for the history. */
function eventTime(iso: string): string {
  const d = new Date(iso);
  const hh = String(d.getUTCHours()).padStart(2, '0');
  const mm = String(d.getUTCMinutes()).padStart(2, '0');
  return `${shortDate(iso)} ${hh}:${mm}`;
}

/** One team: where its plan comes from, who is in it, and what an admin can do to it. */
export function TeamDetailView({
  detail,
  permissions,
  now,
  actions,
}: {
  detail: TeamDetail;
  permissions: Perm[];
  now: string;
  actions: TeamDetailActions;
}) {
  const { team, stripe, override } = detail;
  const clock = new Date(now);
  const canBilling = can(permissions, 'billing');
  const canTeams = can(permissions, 'teams');
  const canEmail = can(permissions, 'emails');
  const { slug, name } = team;
  const source = headerSource(detail, clock);
  const su = detail.suspension;
  // A subscription that is still in force, whatever the plan says. Canceled ones don't bill.
  const billing = stripe !== null && stripe.status !== 'canceled';

  const [dialog, setDialog] = useState<Dialog | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [granting, setGranting] = useState(false);
  const [roles, setRoles] = useState(false);
  const [picked, setPicked] = useState<Record<string, TeamRole>>({});
  const [allHistory, setAllHistory] = useState(false);

  const close = () => setDialog(null);

  /** For a confirm dialog: the API's message stays in the dialog. */
  async function confirm(result: Promise<ActionResult>, done: string) {
    const r = await result;
    if (!r.ok) {
      throw new Error(r.message);
    }
    setProblem(null);
    setNote(done);
    setDialog(null);
  }

  async function stripeCall(
    key: 'sync' | 'invoice',
    call: () => Promise<ActionResult>,
    done: string,
  ) {
    setBusy(key);
    setProblem(null);
    setNote(null);
    try {
      const r = await call();
      if (r.ok) {
        setNote(done);
      } else {
        setProblem(r.message);
      }
    } catch {
      setProblem('That didn’t work. Try again.');
    } finally {
      setBusy(null);
    }
  }

  async function changeRole(userId: string, who: string, role: TeamRole) {
    setPicked((p) => ({ ...p, [userId]: role }));
    setProblem(null);
    setNote(null);
    try {
      const r = await actions.setRole(slug, userId, role);
      if (r.ok) {
        setNote(`${who} is now ${article(role)}.`);
      } else {
        setProblem(r.message);
      }
    } catch {
      setProblem('That didn’t work. Try again.');
    } finally {
      setPicked((p) => {
        const { [userId]: _gone, ...rest } = p;
        return rest;
      });
    }
  }

  function granted(result: { stripeConflict: boolean }, input: OverrideInput) {
    setGranting(false);
    setProblem(null);
    const plan = planName(input.plan);
    const done = override
      ? `Updated the ${plan} override for ${name}.`
      : `Granted ${plan} to ${name}.`;
    setNote(
      result.stripeConflict
        ? `${done} Stripe still bills them until the subscription is cancelled there.`
        : done,
    );
  }

  const history = allHistory ? detail.plan_history : detail.plan_history.slice(0, HISTORY_SHOWN);
  const acceptedLabel = `${detail.members.length} accepted · ${detail.pending.length} pending`;
  const overrideEnded = override !== null && !override.active;

  return (
    <>
      <Breadcrumb items={[{ label: 'Teams', href: adminRoutes.teams }, { label: name }]} />

      {su ? (
        <Banner
          tone="err"
          title={`Suspended${su.by ? ` by ${su.by}` : ''} on ${shortDate(su.at)}${su.reason ? `: “${su.reason}”` : ''}.`}
          action={
            <Button
              size="sm"
              disabled={!canTeams}
              title={canTeams ? undefined : READ_ONLY_TEAMS}
              onClick={() => setDialog({ kind: 'unsuspend' })}
            >
              Lift suspension
            </Button>
          }
        >
          Publishing is refused, share links show “no longer works”, and members see a notice to
          contact support.
        </Banner>
      ) : null}

      {note ? (
        <Notice tone="ok" role="status">
          {note}
        </Notice>
      ) : null}
      {problem ? (
        <Notice tone="err" role="alert">
          {problem}
        </Notice>
      ) : null}

      <div className="flex flex-wrap items-center gap-4">
        <span
          aria-hidden="true"
          className="inline-grid size-12 shrink-0 place-items-center rounded-[12px] bg-surface-300 text-[18px] font-semibold"
        >
          {teamLetter(name)}
        </span>
        <div className="flex min-w-[240px] flex-1 flex-col gap-0.5">
          <div className="flex flex-wrap items-center gap-2">
            <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em] [overflow-wrap:anywhere]">
              {name}
            </h1>
            <Pill>{planName(team.plan)}</Pill>
            {su && team.plan_source === 'none' ? null : (
              <Pill tone={source.tone}>{source.label}</Pill>
            )}
            {su ? <Pill tone="err">Suspended</Pill> : null}
          </div>
          <span className="font-mono text-[12px] text-fg-muted [overflow-wrap:anywhere]">
            {slug} · created {formatDate(team.created_at)}
          </span>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            disabled={!canEmail}
            title={canEmail ? undefined : READ_ONLY_EMAIL}
            onClick={() => setDialog({ kind: 'email' })}
          >
            Email owners
          </Button>
        </div>
      </div>

      <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,440px),1fr))] items-start gap-4">
        <Card aria-labelledby="t-plan" className="col-span-full overflow-hidden">
          <CardHeader id="t-plan" title="Plan">
            <span className="text-[12px] text-fg-muted">
              An override beats Stripe. Stripe keeps billing until the subscription is cancelled
              there.
            </span>
          </CardHeader>
          <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,320px),1fr))]">
            <div className="flex flex-col gap-3 border-hairline p-4 md:border-r">
              <span className="text-[12px] text-fg-muted">Stripe subscription</span>
              {stripe ? (
                <>
                  <div className="flex flex-col gap-1">
                    <span className="font-medium">
                      {[
                        'Team',
                        stripe.interval === 'monthly'
                          ? 'Monthly'
                          : stripe.interval
                            ? 'Annual'
                            : null,
                        plural(stripe.seats, 'seat'),
                      ]
                        .filter(Boolean)
                        .join(' · ')}
                    </span>
                    <span className="text-fg-muted">
                      {[
                        stripe.amount_cents !== null && stripe.currency
                          ? `${dollars(stripe.amount_cents, stripe.currency)} a ${stripe.interval === 'monthly' ? 'month' : 'year'}`
                          : null,
                        stripe.cancel_at
                          ? `cancels ${formatDate(stripe.cancel_at)}`
                          : stripe.renews
                            ? `renews ${formatDate(stripe.renews)}`
                            : null,
                        stripe.past_due_since
                          ? `past due since ${shortDate(stripe.past_due_since)}`
                          : null,
                      ]
                        .filter(Boolean)
                        .join(' · ')}
                    </span>
                    <span className="font-mono text-[12px]">
                      <StripeLink kind="subscriptions" id={stripe.subscription_id} />
                      {stripe.customer_id ? (
                        <>
                          {' · '}
                          <StripeLink kind="customers" id={stripe.customer_id} />
                        </>
                      ) : null}
                    </span>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <Button
                      size="sm"
                      disabled={!canBilling}
                      loading={busy === 'sync'}
                      title={canBilling ? undefined : READ_ONLY_BILLING}
                      onClick={() =>
                        stripeCall('sync', () => actions.stripeSync(slug), 'Synced from Stripe.')
                      }
                    >
                      Sync from Stripe
                    </Button>
                    <Button
                      size="sm"
                      disabled={!canBilling}
                      loading={busy === 'invoice'}
                      title={canBilling ? undefined : READ_ONLY_BILLING}
                      onClick={() =>
                        stripeCall(
                          'invoice',
                          () => actions.resendInvoice(slug),
                          'Invoice sent to the owners.',
                        )
                      }
                    >
                      Resend last invoice
                    </Button>
                  </div>
                </>
              ) : (
                <span className="text-fg-muted">
                  None. This team has never paid through Stripe.
                </span>
              )}
            </div>
            <div className="flex flex-col gap-3 p-4">
              <span className="text-[12px] text-fg-muted">Plan override</span>
              {granting ? (
                <OverrideForm
                  key={override ? 'edit' : 'new'}
                  teamName={name}
                  initial={override}
                  stripe={billing}
                  onSubmit={(input) => actions.grant(slug, input)}
                  onCancel={() => setGranting(false)}
                  onDone={granted}
                />
              ) : override ? (
                <>
                  <div className="flex flex-col gap-1">
                    <span className="font-medium">
                      {planName(override.plan)}
                      {override.until
                        ? ` until ${formatDate(override.until)}${overrideEnded ? ' (ended)' : ''}`
                        : ', no end date'}
                    </span>
                    {override.reason || override.by ? (
                      <span className="text-fg-muted [overflow-wrap:anywhere]">
                        {[
                          override.reason ? `“${override.reason}”` : null,
                          override.by || override.at
                            ? `granted${override.by ? ` by ${override.by}` : ''}${override.at ? ` on ${shortDate(override.at)}` : ''}`
                            : null,
                        ]
                          .filter(Boolean)
                          .join(' · ')}
                      </span>
                    ) : null}
                    {override.until && !overrideEnded ? (
                      <span className="text-[12px] leading-[18px] text-fg-subtle">
                        Owners get an email 14 days before it ends. Then the team goes back to
                        Stripe or Free.
                      </span>
                    ) : null}
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <Button
                      size="sm"
                      disabled={!canBilling}
                      title={canBilling ? undefined : READ_ONLY_BILLING}
                      onClick={() => setGranting(true)}
                    >
                      Edit
                    </Button>
                    <Button
                      size="sm"
                      variant="danger"
                      disabled={!canBilling}
                      title={canBilling ? undefined : READ_ONLY_BILLING}
                      onClick={() => setDialog({ kind: 'remove-override' })}
                    >
                      Remove override
                    </Button>
                  </div>
                </>
              ) : (
                <>
                  <span className="text-fg-muted">
                    None. The plan comes from{' '}
                    {team.plan_source === 'stripe'
                      ? 'the Stripe subscription'
                      : 'the default, Free'}
                    .
                  </span>
                  <Button
                    variant="primary"
                    className="self-start"
                    disabled={!canBilling}
                    title={canBilling ? undefined : READ_ONLY_BILLING}
                    onClick={() => setGranting(true)}
                  >
                    Grant a plan
                  </Button>
                </>
              )}
            </div>
          </div>
          <p className="m-0 flex flex-wrap items-center justify-between gap-3 border-t border-hairline bg-raised px-4 py-2 text-[12px] text-fg-muted">
            <span>
              Effective plan: <span className="text-fg">{planName(team.plan)}</span> · limits:{' '}
              {limitsLabel(team.limits)}
            </span>
            <a href="#t-history">Plan history</a>
          </p>
        </Card>

        <Card aria-labelledby="t-mem" className="overflow-hidden">
          <CardHeader id="t-mem" title="Members">
            <span className="text-[12px] text-fg-muted">{acceptedLabel}</span>
          </CardHeader>
          <ul className="m-0 list-none p-0">
            {detail.members.map((m) => {
              const who = m.name || m.email;
              return (
                <li key={m.user_id} className={ROW}>
                  <span
                    aria-hidden="true"
                    className="inline-grid size-7 shrink-0 place-items-center rounded-full bg-surface-300 text-[11px] font-medium text-fg-muted"
                  >
                    {initials(who)}
                  </span>
                  <Link
                    href={adminRoutes.user(m.user_id)}
                    className="flex min-w-0 flex-1 flex-col text-fg hover:text-fg"
                  >
                    <span className="[overflow-wrap:anywhere]">{who}</span>
                    {m.name ? (
                      <span className="text-[12px] text-fg-muted [overflow-wrap:anywhere]">
                        {m.email}
                      </span>
                    ) : null}
                  </Link>
                  {roles ? (
                    <Select
                      aria-label={`Role of ${who}`}
                      value={picked[m.user_id] ?? m.role}
                      disabled={picked[m.user_id] !== undefined}
                      onChange={(e) => changeRole(m.user_id, who, e.target.value as TeamRole)}
                      options={[
                        { value: 'owner', label: 'Owner' },
                        { value: 'member', label: 'Member' },
                      ]}
                      className="w-[120px]"
                    />
                  ) : (
                    <span className="text-fg-muted">{m.role === 'owner' ? 'Owner' : 'Member'}</span>
                  )}
                </li>
              );
            })}
            {detail.pending.map((p) => (
              <li key={p.id} className={ROW}>
                <span
                  aria-hidden="true"
                  className="inline-grid size-7 shrink-0 place-items-center rounded-full bg-surface-300 text-[11px] font-medium text-fg-muted"
                />
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="[overflow-wrap:anywhere]">{p.email}</span>
                  <span className="text-[12px] text-fg-muted">
                    Pending invite · expires {shortDate(p.expires_at)}
                  </span>
                </span>
                <span className="text-fg-muted">{p.role === 'owner' ? 'Owner' : 'Member'}</span>
              </li>
            ))}
          </ul>
          <p className="m-0 flex flex-wrap items-center justify-between gap-3 border-t border-hairline bg-raised px-4 py-1.5 text-[12px] text-fg-muted">
            <span>Need to hand the team over? Make someone an owner from here.</span>
            <Button
              size="sm"
              disabled={!canTeams}
              title={canTeams ? undefined : READ_ONLY_TEAMS}
              onClick={() => setRoles((r) => !r)}
            >
              {roles ? 'Done' : 'Change roles'}
            </Button>
          </p>
        </Card>

        <Card aria-labelledby="t-col" className="overflow-hidden">
          <CardHeader id="t-col" title="Collections">
            <Link href={adminRoutes.collections} className="text-[12px]">
              All collections
            </Link>
          </CardHeader>
          {detail.collections.length === 0 ? (
            <Empty>No collections yet.</Empty>
          ) : (
            <ul className="m-0 list-none p-0">
              {detail.collections.map((c) => (
                <li key={c.slug} className={ROW}>
                  <span className="flex min-w-0 flex-1 flex-col">
                    <span className="[overflow-wrap:anywhere]">{c.name}</span>
                    <span className="font-mono text-[12px] text-fg-subtle [overflow-wrap:anywhere]">
                      {[
                        c.slug,
                        c.current_version === null ? null : `v${c.current_version}`,
                        c.source === 'git' ? 'git' : 'cloud',
                        c.link_blocked ? 'link blocked' : null,
                      ]
                        .filter(Boolean)
                        .join(' · ')}
                    </span>
                  </span>
                  <Pill>
                    {c.access === 'public'
                      ? 'Public'
                      : c.access === 'restricted'
                        ? 'Restricted'
                        : 'Off'}
                  </Pill>
                </li>
              ))}
            </ul>
          )}
          <p className="m-0 border-t border-hairline bg-raised px-4 py-2 text-[12px] text-fg-muted">
            {plural(detail.usage.tokens_used_this_month, 'token')} used this month ·{' '}
            {formatInt(detail.usage.publishes_this_month)}{' '}
            {detail.usage.publishes_this_month === 1 ? 'publish' : 'publishes'}
          </p>
        </Card>

        <Card aria-labelledby="t-history" className="col-span-full overflow-hidden">
          <CardHeader id="t-history" title="Plan history">
            <span className="text-[12px] text-fg-muted">Newest first</span>
          </CardHeader>
          {history.length === 0 ? (
            <Empty>No plan changes yet.</Empty>
          ) : (
            <ul className="m-0 list-none p-0">
              {history.map((h, i) => (
                <li
                  key={`${i}-${h.at}-${h.source}-${h.plan}`}
                  className="flex min-h-10 flex-wrap items-center gap-x-3 gap-y-0.5 border-b border-hairline px-4 py-1.5 last:border-b-0"
                >
                  <span className="w-[110px] shrink-0 font-mono text-[12px] text-fg-subtle">
                    {eventTime(h.at)}
                  </span>
                  <span className="min-w-[200px] flex-1 text-fg-muted [overflow-wrap:anywhere]">
                    {historyLine(h)}
                    {h.reason ? ` · “${h.reason}”` : ''}
                  </span>
                </li>
              ))}
            </ul>
          )}
          {detail.plan_history.length > HISTORY_SHOWN ? (
            <p className="m-0 border-t border-hairline bg-raised px-4 py-1.5 text-[12px]">
              <Button size="sm" variant="ghost" onClick={() => setAllHistory((v) => !v)}>
                {allHistory ? 'Show fewer' : `Show all ${detail.plan_history.length}`}
              </Button>
            </p>
          ) : null}
        </Card>

        <Card aria-labelledby="t-danger" className="col-span-full border-danger-border">
          <div className="flex flex-wrap items-center gap-4 p-4">
            <div className="flex min-w-[260px] flex-1 flex-col gap-1">
              <h2 id="t-danger" className="m-0 text-[14px] font-semibold">
                Suspend or delete
              </h2>
              <span className="text-fg-muted">
                Suspending stops publishing and share links but keeps everything. Deleting removes
                the team, its collections and links, and cancels its Stripe subscription.
              </span>
            </div>
            {su ? null : (
              <Button
                variant="danger"
                disabled={!canTeams}
                title={canTeams ? undefined : READ_ONLY_TEAMS}
                onClick={() => setDialog({ kind: 'suspend' })}
              >
                Suspend team…
              </Button>
            )}
            <Button
              variant="danger"
              disabled={!canTeams}
              title={canTeams ? undefined : READ_ONLY_TEAMS}
              onClick={() => setDialog({ kind: 'delete' })}
            >
              Delete team…
            </Button>
          </div>
        </Card>
      </div>

      <SuspendDialog
        open={dialog?.kind === 'suspend'}
        onClose={close}
        title={`Suspend ${name}?`}
        description="Publishing stops and share links show “no longer works”. Members still read their own collections, and nothing is deleted."
        confirmLabel="Suspend team"
        notifyLabel={`Email the owners that ${name} is suspended`}
        onSubmit={(reason, notify) => actions.suspend(slug, reason, notify)}
        onDone={() => {
          setProblem(null);
          setNote(`Suspended ${name}.`);
        }}
      />

      <ConfirmDialog
        open={dialog?.kind === 'unsuspend'}
        onClose={close}
        title={`Lift the suspension on ${name}?`}
        description="Publishing and share links work again. Nothing was deleted."
        confirmLabel="Lift suspension"
        pendingLabel="Lifting…"
        variant="primary"
        onConfirm={() => confirm(actions.unsuspend(slug), `Lifted the suspension on ${name}.`)}
      />

      <ConfirmDialog
        open={dialog?.kind === 'remove-override'}
        onClose={close}
        title={`Remove the plan override on ${name}?`}
        description={`The team goes back to ${stripe ? 'its Stripe plan' : 'the default, Free'}. Owners aren’t emailed.`}
        confirmLabel="Remove override"
        pendingLabel="Removing…"
        onConfirm={() => confirm(actions.removeOverride(slug), `Removed the override on ${name}.`)}
      />

      <ConfirmDialog
        open={dialog?.kind === 'delete'}
        onClose={close}
        title={`Delete ${name}?`}
        description={`Removes the team, its collections and links${stripe ? ', and cancels its Stripe subscription' : ''}. This can’t be undone.`}
        confirmLabel="Delete team"
        pendingLabel="Deleting…"
        confirmText={slug}
        onConfirm={async () => {
          const r = await actions.remove(slug);
          if (!r.ok) {
            throw new Error(r.message);
          }
        }}
      />

      <EmailUserDialog
        open={dialog?.kind === 'email'}
        onClose={close}
        name={`the owners of ${name}`}
        onSubmit={(subject, body) => actions.emailOwners(slug, subject, body)}
        onDone={() => {
          setProblem(null);
          setNote(`Email queued for the owners of ${name}.`);
        }}
      />
    </>
  );
}

function StripeLink({ kind, id }: { kind: 'subscriptions' | 'customers'; id: string }) {
  const href = stripeUrl(kind, id);
  if (!href) {
    return <span>{id}</span>;
  }
  return (
    <a href={href} target="_blank" rel="noreferrer">
      {id} ↗
    </a>
  );
}
