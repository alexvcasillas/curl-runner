import Link from 'next/link';
import {
  type AdminOverview,
  adminRoutes,
  can,
  describeEvent,
  formatInt,
  formatMoney,
  type Perm,
} from '@/lib/admin';
import { cx } from '@/lib/cx';
import { initials } from '@/lib/initials';
import { relativeTime } from '@/lib/time';
import { CheckIcon, WarningIcon } from '../icons';
import { StatGrid } from './AdminCard';
import { DayBars } from './DayBars';
import { Sparkline } from './Sparkline';
import type { StatTone } from './StatTile';

const DAY_MS = 86_400_000;

function sum(days: { n: number }[]): number {
  return days.reduce((n, d) => n + d.n, 0);
}

function lastEvent(iso: string | null, now: Date): string {
  if (!iso) {
    return 'none yet';
  }
  const text = relativeTime(iso, now);
  if (text === 'Just now') {
    return 'last just now';
  }
  if (text === 'Yesterday') {
    return 'last yesterday';
  }
  return text.endsWith('ago') ? `last ${text}` : `last on ${text}`;
}

function daysLeft(deadline: string, now: Date): string {
  const n = Math.ceil((new Date(deadline).getTime() - now.getTime()) / DAY_MS);
  if (n <= 0) {
    return 'grace period over';
  }
  return `${n} ${n === 1 ? 'day' : 'days'} left`;
}

const SIGNUP: Record<AdminOverview['signup_mode'], string> = {
  open: 'Sign-up open',
  domains: 'Sign-up by domain',
  closed: 'Sign-up closed',
};

function Status({
  warn,
  name,
  children,
}: {
  warn?: boolean;
  name: string;
  children: React.ReactNode;
}) {
  return (
    <span className="inline-flex items-center gap-1.5">
      {warn ? (
        <WarningIcon size={13} className="text-warning" />
      ) : (
        <CheckIcon size={13} strokeWidth={1.8} className="text-success" />
      )}
      <span className="text-fg">{name}</span> · {children}
    </span>
  );
}

function CardHead({ title, aside }: { title: string; aside?: React.ReactNode }) {
  return (
    <div className="flex h-11 items-center gap-2 border-b border-hairline px-4">
      <h2 className="m-0 mr-auto text-[14px] font-semibold">{title}</h2>
      {aside}
    </div>
  );
}

function Card({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <section
      aria-label={label}
      className="min-w-0 overflow-hidden rounded-[8px] border border-hairline bg-bg"
    >
      {children}
    </section>
  );
}

type Action = { title: string; sub: string; d: string; href: string; perm: Perm };

/** The Overview page. Every figure comes from `GET /v1/admin/overview`. */
export function OverviewView({
  overview: o,
  permissions,
  now = new Date(),
}: {
  overview: AdminOverview;
  permissions: Perm[];
  now?: Date;
}) {
  const c = o.counts;
  const signups = sum(o.signups_30d);
  const teams = sum(o.teams_30d);
  const today = o.publishes_14d[o.publishes_14d.length - 1]?.n ?? 0;

  const tiles: { label: string; value: string; sub?: string; tone?: StatTone }[] = [
    {
      label: 'Users',
      value: formatInt(c.users),
      sub: `+${formatInt(signups)} in 30 days`,
      tone: 'ok',
    },
    {
      label: 'Teams',
      value: formatInt(c.teams),
      sub: `+${formatInt(teams)} in 30 days`,
      tone: 'ok',
    },
    {
      label: 'Collections',
      value: formatInt(c.collections),
      sub: `${formatInt(c.collections_public)} shared publicly, ${formatInt(c.collections_restricted)} restricted`,
    },
    {
      label: 'Customers with access',
      value: formatInt(c.access_list_people),
      sub: 'People on access lists, not members',
    },
  ];
  if (o.billing) {
    tiles.push(
      c.mrr_cents === null
        ? { label: 'MRR', value: 'Unavailable', sub: 'Stripe could not be read', tone: 'warn' }
        : {
            label: 'MRR',
            value: formatMoney(c.mrr_cents),
            sub:
              c.granted_teams > 0
                ? `Not counting ${formatInt(c.granted_teams)} granted ${c.granted_teams === 1 ? 'team' : 'teams'}`
                : undefined,
          },
      c.paying_teams === null
        ? {
            label: 'Paying teams',
            value: 'Unavailable',
            sub: 'Stripe could not be read',
            tone: 'warn',
          }
        : {
            label: 'Paying teams',
            value: formatInt(c.paying_teams),
            sub: `${formatInt(c.paying_annual ?? 0)} annual, ${formatInt(c.paying_monthly ?? 0)} monthly`,
          },
    );
  }
  tiles.push({
    label: 'Plan overrides',
    value: formatInt(c.plan_overrides),
    sub:
      c.plan_overrides_ending > 0
        ? `${formatInt(c.plan_overrides_ending)} end within 30 days`
        : undefined,
    tone: 'warn',
  });
  if (o.billing) {
    const next = o.past_due.next;
    tiles.push({
      label: 'Past due',
      value: formatInt(o.past_due.teams),
      sub:
        o.past_due.teams === 0
          ? 'None'
          : next
            ? `${next.name}, ${daysLeft(next.deadline, now)}`
            : undefined,
      tone: o.past_due.teams === 0 ? 'muted' : 'err',
    });
  }

  const actions: Action[] = [
    {
      title: 'Grant a plan',
      sub: 'Give a team Team without Stripe',
      d: 'M8 1.5l1.9 4 4.4.5-3.3 3 .9 4.3L8 11.1 4.1 13.3 5 9 1.7 6l4.4-.5z',
      href: adminRoutes.teams,
      perm: 'billing',
    },
    {
      title: 'New email',
      sub: 'Write to users, owners or one team',
      d: 'M2 3.5h12v9H2zM2 4l6 5 6-5',
      href: adminRoutes.emails,
      perm: 'emails',
    },
    {
      title: o.signup_mode === 'closed' ? 'Open sign-up' : 'Sign-up settings',
      sub:
        o.signup_mode === 'closed'
          ? 'Closed now. Change who can create teams'
          : `${o.signup_mode === 'open' ? 'Open' : 'By domain'} now. Change who can create teams`,
      d: 'M4.5 7.5h7v6h-7zM5.5 7.5V5.5a2.5 2.5 0 015-1',
      href: adminRoutes.settings,
      perm: 'settings',
    },
    {
      title: 'Add an admin',
      sub: 'Pick an account and what it can change',
      d: 'M8 1.5l5 2v4c0 3.2-2.2 5.6-5 7-2.8-1.4-5-3.8-5-7v-4zM8 6v4M6 8h4',
      href: adminRoutes.admins,
      perm: 'admins',
    },
  ];

  return (
    <>
      <div className="flex flex-wrap items-end gap-3">
        <div className="mr-auto">
          <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em]">Overview</h1>
          <p className="mt-1 mb-0 text-fg-muted">People, teams, publishing and revenue.</p>
        </div>
        {o.billing ? (
          <span className="text-[12px] text-fg-subtle">Live · Stripe figures refresh hourly</span>
        ) : null}
      </div>

      <section
        aria-label="Server health"
        className="flex flex-wrap items-center gap-x-6 gap-y-2 rounded-[8px] border border-hairline px-3.5 py-2.5 text-[12px] text-fg-muted"
      >
        <Status name="API">
          {o.p95_ms === null ? 'no traffic yet' : `p95 ${formatInt(o.p95_ms)} ms`}
        </Status>
        <Status name="Email" warn={o.outbox.failed_today > 0}>
          {formatInt(o.outbox.waiting)} waiting, {formatInt(o.outbox.failed_today)} failed today
        </Status>
        {o.billing ? (
          <Status name="Stripe webhooks">{lastEvent(o.last_webhook_at, now)}</Status>
        ) : null}
        <Status name={SIGNUP[o.signup_mode]} warn={o.signup_mode === 'closed'}>
          {o.signup_mode === 'closed' ? 'new accounts need an invite' : 'new accounts can join'}
        </Status>
        <Link href={adminRoutes.health} className="ml-auto">
          Health
        </Link>
      </section>

      <StatGrid tiles={tiles} />

      <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,440px),1fr))] gap-4">
        <Card label="Sign-ups and publishes">
          <CardHead
            title="Sign-ups and new teams"
            aside={<span className="text-[12px] text-fg-muted">Last 30 days</span>}
          />
          <Sparkline signups={o.signups_30d} teams={o.teams_30d} />
        </Card>

        <Card label="Publishes per day">
          <CardHead
            title="Publishes per day"
            aside={<span className="text-[12px] text-fg-muted">CI and studio · 14 days</span>}
          />
          <DayBars days={o.publishes_14d} label="Publishes per day" />
          <div className="flex min-h-10 items-center justify-between gap-3 border-t border-hairline px-4 text-[12px] text-fg-muted">
            <span>{formatInt(today)} today</span>
            <Link href={adminRoutes.collections}>Collections</Link>
          </div>
        </Card>

        <Card label="Recent admin activity">
          <CardHead
            title="Recent admin activity"
            aside={
              <Link href={adminRoutes.activity} className="text-[12px]">
                Activity log
              </Link>
            }
          />
          {o.recent_events.length === 0 ? (
            <p className="m-0 px-4 py-6 text-fg-muted">No admin activity yet.</p>
          ) : (
            <ul className="m-0 list-none p-0 py-1">
              {o.recent_events.map((e) => {
                const who = e.admin_email.split('@')[0];
                return (
                  <li
                    key={e.id}
                    className="flex min-h-10 items-center gap-2.5 border-b border-surface-200 px-4"
                  >
                    <span
                      aria-hidden="true"
                      className="inline-grid size-[22px] shrink-0 place-items-center rounded-full bg-surface-300 text-[10px] text-fg-muted"
                    >
                      {initials(e.admin_email)}
                    </span>
                    <span className="min-w-0 flex-1 text-fg-muted">
                      <span className="text-fg">{who}</span> {describeEvent(e.action)}
                      {e.target_label ? (
                        <>
                          {' '}
                          <span className="font-mono text-[12px] text-fg">{e.target_label}</span>
                        </>
                      ) : null}
                    </span>
                    <span className="text-[12px] whitespace-nowrap text-fg-subtle">
                      {relativeTime(e.at, now)}
                    </span>
                  </li>
                );
              })}
            </ul>
          )}
        </Card>

        <Card label="Quick actions">
          <CardHead title="Quick actions" />
          <div className="grid grid-cols-2">
            {actions.map((a) => {
              const allowed = can(permissions, a.perm);
              const body = (
                <>
                  <span className="inline-flex size-8 items-center justify-center rounded-[8px] border border-control text-fg-muted">
                    <svg
                      aria-hidden="true"
                      width="16"
                      height="16"
                      viewBox="0 0 16 16"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="1.5"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    >
                      <path d={a.d} />
                    </svg>
                  </span>
                  <span className="font-medium">{a.title}</span>
                  <span className="text-[12px] text-fg-muted">{allowed ? a.sub : 'Read only'}</span>
                </>
              );
              const cls = 'flex flex-col gap-1.5 border-r border-b border-hairline px-4 py-[18px]';
              return allowed ? (
                <Link
                  key={a.title}
                  href={a.href}
                  className={cx(cls, 'text-fg no-underline hover:bg-surface-100 hover:text-fg')}
                >
                  {body}
                </Link>
              ) : (
                <div
                  key={a.title}
                  title="You can read this but not change it."
                  className={cx(cls, 'cursor-not-allowed text-fg-disabled')}
                >
                  {body}
                </div>
              );
            })}
          </div>
        </Card>
      </div>
    </>
  );
}
