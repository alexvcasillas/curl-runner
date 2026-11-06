import Link from 'next/link';
import { adminRoutes, formatInt, formatMoney } from '@/lib/admin';
import { eventKind, formatDelta, type StripeStats } from '@/lib/admin-stats';
import { relativeTime } from '@/lib/time';
import { ButtonLink } from '../ButtonLink';
import { EmptyState } from '../EmptyState';
import { PageHeader } from '../PageHeader';
import { Pill } from '../Pill';
import { AdminCard, CardFoot, CardHead, StatGrid, type Tile } from './AdminCard';
import { MrrChart } from './MrrChart';

const STRIPE = 'https://dashboard.stripe.com';
const plural = (n: number, one: string) => `${formatInt(n)} ${n === 1 ? one : `${one}s`}`;

const SOURCES: Record<StripeStats['by_source'][number]['source'], { label: string; dot: string }> =
  {
    stripe_annual: { label: 'Stripe, annual', dot: 'bg-success' },
    stripe_monthly: { label: 'Stripe, monthly', dot: 'bg-blue' },
    granted: { label: 'Granted by an admin', dot: 'bg-fg-subtle' },
  };

function StripeLink() {
  return (
    <ButtonLink href={STRIPE} external>
      Open Stripe dashboard
    </ButtonLink>
  );
}

/**
 * The Stripe page. `stats` is null when billing is off on this server (the
 * API's 404): the page then says so and counts the teams an admin granted.
 */
export function StripeView({
  stats,
  granted,
}: {
  stats: StripeStats | null;
  /** From the overview, which has these with billing off too. */
  granted: { teams: number; seats: number };
}) {
  if (!stats) {
    return (
      <>
        <PageHeader title="Stripe">
          Subscription revenue from Stripe webhooks. Annual plans count as one twelfth a month.
          Granted plans aren’t revenue and are listed apart.
        </PageHeader>
        <EmptyState title="Billing is off on this server" as="h2">
          <span className="block">
            There is no Stripe revenue to show. Set the Stripe keys in the environment and redeploy
            to turn billing on.
          </span>
          <span className="mt-2 block">
            {plural(granted.teams, 'team')} {granted.teams === 1 ? 'is' : 'are'} on a plan granted
            by an admin, {plural(granted.seats, 'seat')}. Granted plans are never revenue.
          </span>
        </EmptyState>
      </>
    );
  }

  const n = stats.net_new;
  const annual = stats.by_source.find((s) => s.source === 'stripe_annual');
  const monthly = stats.by_source.find((s) => s.source === 'stripe_monthly');
  const tiles: Tile[] = [
    {
      label: 'MRR',
      value: formatMoney(stats.mrr_cents),
      sub:
        n.total_cents === 0 ? 'No change this month' : `${formatDelta(n.total_cents)} this month`,
      tone: n.total_cents > 0 ? 'ok' : n.total_cents < 0 ? 'err' : 'muted',
    },
    { label: 'ARR', value: formatMoney(stats.arr_cents), sub: 'MRR × 12' },
    {
      label: 'Paying teams',
      value: formatInt(stats.paying_teams),
      sub: `${formatInt(annual?.teams ?? 0)} annual, ${formatInt(monthly?.teams ?? 0)} monthly`,
    },
    {
      label: 'Billed seats',
      value: formatInt(stats.seats),
      sub:
        stats.paying_teams > 0
          ? `Average ${(stats.seats / stats.paying_teams).toFixed(1)} per team`
          : undefined,
    },
    {
      label: 'Cancelled',
      value: formatInt(n.cancelled_teams),
      sub:
        n.cancelled_teams === 0
          ? 'None this month'
          : `${formatDelta(n.churned_cents)} MRR this month`,
      tone: n.cancelled_teams > 0 ? 'warn' : 'muted',
    },
    {
      label: 'Past due',
      value: formatInt(stats.past_due.teams),
      sub:
        stats.past_due.teams === 0
          ? 'None'
          : `${formatMoney(stats.past_due.at_risk_cents)} at risk`,
      tone: stats.past_due.teams === 0 ? 'muted' : 'err',
    },
  ];
  const total = Math.max(
    1,
    stats.by_source.reduce((t, s) => t + s.teams, 0),
  );
  const w = stats.webhooks;
  const now = new Date();

  return (
    <>
      <PageHeader title="Stripe" actions={<StripeLink />}>
        Subscription revenue from Stripe webhooks. Annual plans count as one twelfth a month.
        Granted plans aren’t revenue and are listed apart.
      </PageHeader>

      <StatGrid tiles={tiles} />

      <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,440px),1fr))] gap-4">
        <AdminCard label="MRR trend">
          <CardHead title="MRR, last 90 days" />
          <MrrChart points={stats.mrr_90d} />
          <CardFoot>
            <span>
              {`Net new this month ${formatDelta(n.total_cents)} · new ${formatDelta(n.new_cents)} · expansion ${formatDelta(n.expansion_cents)} · churned ${formatDelta(n.churned_cents)}`}
            </span>
          </CardFoot>
        </AdminCard>

        <AdminCard label="Plan mix">
          <CardHead title="Teams by how they get Team" />
          <div className="flex flex-col gap-3 p-4">
            <div
              role="img"
              aria-label={stats.by_source
                .map((s) => `${s.teams} ${SOURCES[s.source].label.toLowerCase()}`)
                .join(', ')}
              className="flex h-2 overflow-hidden rounded-full bg-surface-300"
            >
              {stats.by_source.map((s) => (
                <span
                  key={s.source}
                  className={SOURCES[s.source].dot}
                  style={{ width: `${(s.teams / total) * 100}%` }}
                />
              ))}
            </div>
            <ul className="m-0 flex list-none flex-col gap-2 p-0">
              {stats.by_source.map((s) => (
                <li key={s.source} className="flex items-center gap-2.5">
                  <span
                    aria-hidden="true"
                    className={`size-2 rounded-full ${SOURCES[s.source].dot}`}
                  />
                  <span className="flex-1">{SOURCES[s.source].label}</span>
                  {s.source === 'granted' ? (
                    <span className="text-[12px] text-fg-subtle">Not revenue</span>
                  ) : null}
                  <span className="text-[12px] text-fg-muted">{plural(s.seats, 'seat')}</span>
                  <span className="w-8 text-right tabular-nums">{formatInt(s.teams)}</span>
                </li>
              ))}
            </ul>
          </div>
          <CardFoot>
            <span>
              {`Webhooks: ${formatInt(w.events_24h)} in 24 h · ${plural(w.bad_signatures_24h, 'bad signature')} · ${plural(w.duplicates_24h, 'duplicate')} skipped`}
            </span>
            <Link href={adminRoutes.health}>Health</Link>
          </CardFoot>
        </AdminCard>
      </div>

      <AdminCard label="Recent subscription events">
        <CardHead title="Recent subscription events" />
        {stats.events.length === 0 ? (
          <p className="m-0 px-4 py-6 text-fg-muted">No subscription events yet.</p>
        ) : (
          <div className="overflow-x-auto">
            <div role="table" aria-label="Subscription events" style={{ minWidth: 760 }}>
              <div
                role="row"
                className="flex h-9 items-center gap-3 border-b border-hairline px-4 text-[12px] text-fg-subtle"
              >
                <span role="columnheader" className="w-[120px] shrink-0">
                  Event
                </span>
                <span role="columnheader" className="w-[140px] shrink-0">
                  Team
                </span>
                <span role="columnheader" className="min-w-0 flex-1">
                  Change
                </span>
                <span role="columnheader" className="w-[70px] shrink-0 text-right">
                  MRR
                </span>
                <span role="columnheader" className="w-[90px] shrink-0">
                  When
                </span>
                <span role="columnheader" className="w-[130px] shrink-0">
                  Stripe
                </span>
              </div>
              {stats.events.map((e) => {
                const kind = eventKind(e.type, e.summary);
                const delta = e.mrr_delta_cents;
                return (
                  <div
                    key={e.id}
                    role="row"
                    className="flex min-h-11 items-center gap-3 border-b border-hairline px-4 py-1.5 last:border-b-0"
                  >
                    <span role="cell" className="w-[120px] shrink-0">
                      <Pill tone={kind.tone}>{kind.label}</Pill>
                    </span>
                    <span role="cell" className="w-[140px] shrink-0 truncate">
                      {e.team ? (
                        <Link href={adminRoutes.team(e.team.slug)}>{e.team.name}</Link>
                      ) : (
                        '—'
                      )}
                    </span>
                    <span role="cell" className="min-w-0 flex-1 text-fg-muted">
                      {e.summary ?? '—'}
                    </span>
                    <span
                      role="cell"
                      className={`w-[70px] shrink-0 text-right tabular-nums ${delta !== null && delta > 0 ? 'text-success' : delta !== null && delta < 0 ? 'text-danger' : 'text-fg-muted'}`}
                    >
                      {formatDelta(delta)}
                    </span>
                    <span role="cell" className="w-[90px] shrink-0 text-fg-muted">
                      {relativeTime(e.at, now)}
                    </span>
                    <span role="cell" className="w-[130px] shrink-0 truncate font-mono text-[12px]">
                      <a
                        href={`${STRIPE}/events/${encodeURIComponent(e.id)}`}
                        target="_blank"
                        rel="noreferrer"
                        title="Open in Stripe"
                      >
                        {e.id}
                      </a>
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </AdminCard>
    </>
  );
}
