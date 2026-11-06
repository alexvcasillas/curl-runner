'use client';

import { useRouter } from 'next/navigation';
import { useEffect } from 'react';
import { formatInt } from '@/lib/admin';
import {
  agoShort,
  formatBytes,
  formatUptime,
  type Health,
  type HealthJob,
  jobLabel,
} from '@/lib/admin-stats';
import { PageHeader } from '../PageHeader';
import { Pill, type PillTone } from '../Pill';
import { AdminCard, CardFoot, CardHead, StatGrid, type Tile } from './AdminCard';

/** How often the page asks the server for fresh numbers. */
export const REFRESH_MS = 30_000;

const JOB_STATUS: Record<HealthJob['status'], { label: string; tone: PillTone }> = {
  ok: { label: 'OK', tone: 'ok' },
  failed: { label: 'Failed', tone: 'err' },
  stale: { label: 'Stale', tone: 'warn' },
  never_run: { label: 'Not run yet', tone: 'muted' },
};

const plural = (n: number, one: string) => `${formatInt(n)} ${n === 1 ? one : `${one}s`}`;

function Row({ children }: { children: React.ReactNode }) {
  return (
    <div
      role="row"
      className="flex min-h-11 items-center gap-3 border-b border-hairline px-4 py-1.5 last:border-b-0"
    >
      {children}
    </div>
  );
}

function utcClock(d: Date): string {
  return d.toISOString().slice(11, 19);
}

/** What the server is doing right now. It asks for fresh numbers every 30 seconds. */
export function HealthView({ health: h, now: nowIso }: { health: Health; now: string }) {
  const router = useRouter();
  const now = new Date(nowIso);

  useEffect(() => {
    const timer = setInterval(() => {
      // A tab nobody is looking at doesn't need to poll.
      if (!document.hidden) {
        router.refresh();
      }
    }, REFRESH_MS);
    return () => clearInterval(timer);
  }, [router]);

  const s = h.stats;
  const failed5xx = s.requests_5m > 0 ? (s.errors_5xx_5m / s.requests_5m) * 100 : 0;
  const tiles: Tile[] = [
    { label: 'Server', value: s.version, sub: `up ${formatUptime(s.uptime_secs)}` },
    {
      label: 'API p95',
      value: s.p95_ms === null ? '—' : `${formatInt(s.p95_ms)} ms`,
      sub: `${formatInt(s.requests_5m)} requests in 5 min`,
      tone: 'ok',
    },
    {
      label: 'Postgres',
      value: formatBytes(s.db.size_bytes),
      sub: `${s.db.connections} of ${s.db.max_connections} connections${s.db.migration === null ? '' : ` · migration ${s.db.migration}`}`,
    },
    {
      label: 'Email queue',
      value: formatInt(s.email.waiting),
      sub:
        s.email.waiting === 0 && s.email.failed_today === 0
          ? 'Nothing waiting'
          : [
              s.email.oldest_waiting_secs === null
                ? null
                : `Oldest ${s.email.oldest_waiting_secs} s`,
              `${formatInt(s.email.failed_today)} failed today`,
            ]
              .filter(Boolean)
              .join(' · '),
      tone: s.email.failed_today > 0 ? 'warn' : 'ok',
    },
    {
      label: 'Errors (5xx)',
      value: formatInt(s.errors_5xx_5m),
      sub:
        s.requests_5m === 0
          ? 'No requests in 5 min'
          : `${failed5xx < 0.1 && failed5xx > 0 ? '<0.1' : failed5xx.toFixed(1)}% of requests in 5 min`,
      tone: s.errors_5xx_5m > 0 ? 'warn' : 'ok',
    },
  ];

  return (
    <>
      <PageHeader
        title="Health"
        actions={
          <span className="text-[12px] text-fg-subtle">
            Live · refreshes every 30 s · updated {utcClock(now)} UTC
          </span>
        }
      >
        What the server is doing right now. Nothing here changes anything. Counters live in memory
        and reset when the server restarts.
      </PageHeader>

      <StatGrid tiles={tiles} />

      <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,360px),1fr))] gap-4">
        <AdminCard label="Background jobs">
          <CardHead title="Background jobs" />
          <div role="table" aria-label="Background jobs">
            {h.jobs.map((j) => {
              const st = JOB_STATUS[j.status];
              return (
                <Row key={j.name}>
                  <span role="cell" className="min-w-0 flex-1">
                    <span className="block">{jobLabel(j.name)}</span>
                    <span className="block text-[12px] text-fg-muted">
                      {j.last_run
                        ? `Ran ${agoShort(j.last_run, now)}`
                        : 'Hasn’t run since the server started'}
                    </span>
                  </span>
                  <span role="cell">
                    <Pill tone={st.tone}>{st.label}</Pill>
                  </span>
                </Row>
              );
            })}
          </div>
        </AdminCard>

        <AdminCard label="Stripe webhooks">
          <CardHead title="Stripe webhooks, 24 h" />
          {h.webhooks_24h.length === 0 ? (
            <p className="m-0 px-4 py-6 text-fg-muted">No webhooks in the last 24 hours.</p>
          ) : (
            <div role="table" aria-label="Webhooks by type">
              {h.webhooks_24h.map((w) => (
                <Row key={w.type}>
                  <span role="cell" className="min-w-0 flex-1 truncate font-mono text-[12px]">
                    {w.type}
                  </span>
                  <span role="cell" className="tabular-nums">
                    {formatInt(w.n)}
                  </span>
                </Row>
              ))}
            </div>
          )}
          <CardFoot>
            <span>
              {`${plural(h.bad_signatures_24h, 'bad signature')} · ${plural(h.duplicates_24h, 'duplicate')} skipped · last ${h.last_webhook_at ? agoShort(h.last_webhook_at, now) : 'none yet'}`}
            </span>
          </CardFoot>
        </AdminCard>

        <AdminCard label="Rate limits">
          <CardHead
            title="Rate limits hit, last hour"
            aside={
              <span className="text-[12px] text-fg-subtle">Per process, reset on restart</span>
            }
          />
          {h.rate_limits.length === 0 ? (
            <p className="m-0 px-4 py-6 text-fg-muted">No rate limits hit in the last hour.</p>
          ) : (
            <div role="table" aria-label="Rate limits hit">
              {h.rate_limits.map((l) => (
                <Row key={l.key}>
                  <span role="cell" className="w-[200px] shrink-0 break-all font-mono text-[12px]">
                    {l.key}
                  </span>
                  <span role="cell" className="min-w-0 flex-1 text-fg-muted">
                    {l.what}
                  </span>
                  <span role="cell" className="tabular-nums">
                    {formatInt(l.n)}
                  </span>
                </Row>
              ))}
            </div>
          )}
        </AdminCard>
      </div>
    </>
  );
}
