'use client';

import { useRouter } from 'next/navigation';
import { useEffect, useRef, useState } from 'react';
import {
  KIND_LABELS,
  kindLabel,
  LOG_STATUSES,
  type LogPage,
  type LogStatusFilter,
  logDetail,
  logHref,
  logWhen,
  MAX_ATTEMPTS,
  previousLogCursor,
} from '@/lib/admin-emails';
import { FIRST_PAGE } from '@/lib/admin-teams';
import { Button } from '../Button';
import { ButtonLink } from '../ButtonLink';
import { EmptyState } from '../EmptyState';
import { inputClasses } from '../field-styles';
import { Pill, type PillTone } from '../Pill';
import { Segmented } from '../Segmented';
import { Select } from '../Select';
import { Table, TableCell, TableHead, TableHeader, TableRow } from '../Table';

const SEARCH_DELAY = 350;

const STATUS: Record<string, { label: string; tone: PillTone }> = {
  sent: { label: 'Sent', tone: 'ok' },
  failed: { label: 'Failed', tone: 'err' },
  queued: { label: 'Waiting', tone: 'warn' },
};

/** Every email the server queued: who, what, and where it stands. Newest first. */
export function DeliveryLogView({
  page,
  q,
  status,
  kind,
  cursor,
  trail,
  now,
}: {
  page: LogPage;
  q: string;
  status: LogStatusFilter;
  kind: string;
  cursor?: string;
  trail: string[];
  now: string;
}) {
  const router = useRouter();
  const clock = new Date(now);
  const [search, setSearch] = useState(q);
  const filters = { q, status, kind };

  const lastQ = useRef(q);
  lastQ.current = q;
  useEffect(() => {
    const next = search.trim();
    if (next === lastQ.current) {
      return;
    }
    const timer = setTimeout(
      () => router.replace(logHref({ q: next, status, kind, trail: [] })),
      SEARCH_DELAY,
    );
    return () => clearTimeout(timer);
  }, [search, status, kind, router]);

  const items = page.items;
  const filtered = q !== '' || status !== 'all' || kind !== 'all';

  return (
    <>
      <div className="flex flex-wrap items-end gap-3">
        <form
          role="search"
          className="relative flex min-w-[220px] max-w-[340px] flex-1 items-center"
          onSubmit={(e) => {
            e.preventDefault();
            router.push(logHref({ ...filters, q: search.trim(), trail: [] }));
          }}
        >
          <svg
            aria-hidden="true"
            width="14"
            height="14"
            viewBox="0 0 16 16"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            className="pointer-events-none absolute left-2.5 text-fg-subtle"
          >
            <circle cx="7" cy="7" r="4.5" />
            <path d="M10.5 10.5L14 14" />
          </svg>
          <input
            type="search"
            aria-label="Search by recipient"
            placeholder="Search by recipient"
            value={search}
            maxLength={200}
            autoComplete="off"
            onChange={(e) => setSearch(e.target.value)}
            className={inputClasses(false, 'pl-8')}
          />
        </form>
        <Select
          label="Kind"
          value={kind}
          className="w-[190px]"
          options={[
            { value: 'all', label: 'All kinds' },
            ...Object.keys(KIND_LABELS).map((k) => ({ value: k, label: kindLabel(k) })),
          ]}
          onChange={(e) => router.push(logHref({ ...filters, kind: e.target.value, trail: [] }))}
        />
        <Segmented
          label="Status"
          options={LOG_STATUSES.map((s) => ({ value: s.value, label: s.label }))}
          value={status}
          onChange={(value) => router.push(logHref({ ...filters, status: value, trail: [] }))}
        />
      </div>

      {items.length === 0 ? (
        <EmptyState
          title={filtered ? 'No emails match' : 'No emails yet'}
          actions={
            filtered ? (
              <ButtonLink href={logHref({ q: '', status: 'all', kind: 'all', trail: [] })}>
                Clear filters
              </ButtonLink>
            ) : null
          }
        >
          {filtered
            ? 'Try another recipient, kind or status.'
            : 'Emails show up here as the server queues them.'}
        </EmptyState>
      ) : (
        <Table
          label="Delivery log"
          minWidth={980}
          footer={
            <>
              <span className="mr-auto">
                Sign-in codes never show here in full. Failed emails retry up to {MAX_ATTEMPTS}{' '}
                times, waiting longer each time.
              </span>
              <span className="flex gap-1.5">
                {trail.length > 0 ? (
                  <ButtonLink
                    size="sm"
                    href={logHref({
                      ...filters,
                      cursor: previousLogCursor(trail),
                      trail: trail.slice(0, -1),
                    })}
                  >
                    Previous
                  </ButtonLink>
                ) : (
                  <Button size="sm" disabled>
                    Previous
                  </Button>
                )}
                {page.next_cursor ? (
                  <ButtonLink
                    size="sm"
                    href={logHref({
                      ...filters,
                      cursor: page.next_cursor,
                      trail: [...trail, cursor ?? FIRST_PAGE],
                    })}
                  >
                    Next
                  </ButtonLink>
                ) : (
                  <Button size="sm" disabled>
                    Next
                  </Button>
                )}
              </span>
            </>
          }
        >
          <TableHeader>
            <TableHead width={120}>Queued</TableHead>
            <TableHead width={280}>To</TableHead>
            <TableHead width={230}>Email</TableHead>
            <TableHead width={100}>Status</TableHead>
            <TableHead>Detail</TableHead>
          </TableHeader>
          {items.map((r) => {
            const s = STATUS[r.status] ?? STATUS.queued;
            return (
              <TableRow key={r.id} className="min-h-[52px] py-2">
                <TableCell width={120} className="font-mono text-[12px] text-fg-subtle">
                  {logWhen(r.created_at, clock)}
                </TableCell>
                <TableCell width={280} className="[overflow-wrap:anywhere]">
                  {r.to}
                </TableCell>
                <TableCell width={230}>
                  <span className="flex min-w-0 flex-col">
                    <span className="text-fg-muted">{kindLabel(r.kind)}</span>
                    <span className="truncate text-[12px] text-fg-subtle" title={r.subject}>
                      {r.subject}
                    </span>
                  </span>
                </TableCell>
                <TableCell width={100}>
                  <Pill tone={s.tone}>{s.label}</Pill>
                </TableCell>
                <TableCell width={280} className="truncate font-mono text-[12px] text-fg-muted">
                  <span title={logDetail(r, clock)}>{logDetail(r, clock)}</span>
                </TableCell>
              </TableRow>
            );
          })}
        </Table>
      )}
    </>
  );
}
