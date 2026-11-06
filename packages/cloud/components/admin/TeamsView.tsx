'use client';

import { useRouter } from 'next/navigation';
import { useEffect, useRef, useState } from 'react';
import { adminRoutes, formatInt, formatMoney } from '@/lib/admin';
import { nextPage, previousPage } from '@/lib/admin-paging';
import {
  planName,
  sourceOf,
  TEAM_FILTERS,
  type TeamFilter,
  type TeamsPage,
  teamsExportHref,
  teamsHref,
} from '@/lib/admin-teams';
import { teamLetter } from '@/lib/initials';
import { shortDate } from '@/lib/time';
import { Button } from '../Button';
import { ButtonLink } from '../ButtonLink';
import { buttonClasses } from '../button-styles';
import { EmptyState } from '../EmptyState';
import { inputClasses } from '../field-styles';
import { PageHeader } from '../PageHeader';
import { Pill } from '../Pill';
import { Segmented } from '../Segmented';
import { Table, TableCell, TableHead, TableHeader, TableRow, TableRowLink } from '../Table';

const SEARCH_DELAY = 350;

/** Every team, where its plan comes from and what it pays. Reads are open to any admin. */
export function TeamsView({
  page,
  q,
  filter,
  cursor,
  trail,
  pageSize,
  now,
}: {
  page: TeamsPage;
  q: string;
  filter: TeamFilter;
  cursor?: string;
  trail: string[];
  pageSize: number;
  now: string;
}) {
  const router = useRouter();
  const clock = new Date(now);
  const [search, setSearch] = useState(q);
  const query = { q, filter, trail: [] as string[] };

  // Search as they type, after a pause. Enter goes now.
  const lastQ = useRef(q);
  lastQ.current = q;
  useEffect(() => {
    const next = search.trim();
    if (next === lastQ.current) {
      return;
    }
    const timer = setTimeout(
      () => router.replace(teamsHref({ q: next, filter, trail: [] })),
      SEARCH_DELAY,
    );
    return () => clearTimeout(timer);
  }, [search, filter, router]);

  const items = page.items;
  const start = trail.length * pageSize + 1;
  const end = start + items.length - 1;
  const filtered = q !== '' || filter !== 'all';

  return (
    <>
      <PageHeader
        title="Teams"
        actions={
          <a href={teamsExportHref({ q, filter })} download className={buttonClasses()}>
            Export CSV
          </a>
        }
      >
        Every team, where its plan comes from, and what it pays.
      </PageHeader>

      <div className="flex flex-wrap items-center gap-3">
        <form
          role="search"
          className="relative flex min-w-[220px] max-w-[340px] flex-1 items-center"
          onSubmit={(e) => {
            e.preventDefault();
            router.push(teamsHref({ q: search.trim(), filter, trail: [] }));
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
            aria-label="Search teams"
            placeholder="Search by name, slug or owner"
            value={search}
            maxLength={200}
            autoComplete="off"
            onChange={(e) => setSearch(e.target.value)}
            className={inputClasses(false, 'pl-8')}
          />
        </form>
        <Segmented
          label="Filter teams"
          options={TEAM_FILTERS.map((f) => ({ value: f.value, label: f.label }))}
          value={filter}
          onChange={(value) => router.push(teamsHref({ ...query, filter: value }))}
        />
      </div>

      {items.length === 0 ? (
        <EmptyState
          title={filtered ? 'No teams match' : 'No teams yet'}
          actions={filtered ? <ButtonLink href={adminRoutes.teams}>Clear search</ButtonLink> : null}
        >
          {filtered
            ? 'Try another name, slug, owner or filter.'
            : 'Teams show up here once someone creates one.'}
        </EmptyState>
      ) : (
        <Table
          label="Teams"
          minWidth={1060}
          footer={
            <>
              <span className="mr-auto">
                Showing {formatInt(start)}–{formatInt(end)} of {formatInt(page.total)}
              </span>
              <span className="flex gap-1.5">
                {trail.length > 0 ? (
                  <ButtonLink
                    size="sm"
                    href={teamsHref({
                      q,
                      filter,
                      ...previousPage(trail),
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
                    href={teamsHref({
                      q,
                      filter,
                      ...nextPage(trail, cursor, page.next_cursor),
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
            <TableHead>Team</TableHead>
            <TableHead width={230}>Owner</TableHead>
            <TableHead width={70} className="text-right">
              Members
            </TableHead>
            <TableHead width={90} className="text-right">
              Collections
            </TableHead>
            <TableHead width={70}>Plan</TableHead>
            <TableHead width={210}>Source</TableHead>
            <TableHead width={80} className="text-right">
              MRR
            </TableHead>
          </TableHeader>
          {items.map((t) => {
            const source = sourceOf(t, clock);
            return (
              <TableRow key={t.slug} className="min-h-[52px] py-2">
                <TableCell>
                  <span className="flex min-w-0 items-center gap-2.5">
                    <span
                      aria-hidden="true"
                      className="inline-grid size-7 shrink-0 place-items-center rounded-[7px] bg-surface-300 text-[12px] font-semibold"
                    >
                      {teamLetter(t.name)}
                    </span>
                    <span className="flex min-w-0 flex-col">
                      <TableRowLink
                        href={adminRoutes.team(t.slug)}
                        className="[overflow-wrap:anywhere]"
                      >
                        {t.name}
                      </TableRowLink>
                      <span className="font-mono text-[12px] text-fg-subtle [overflow-wrap:anywhere]">
                        {t.slug} · since {shortDate(t.created_at)}
                      </span>
                    </span>
                  </span>
                </TableCell>
                <TableCell
                  width={230}
                  className="text-[12px] text-fg-muted [overflow-wrap:anywhere]"
                >
                  {t.owner?.email ?? 'No owner'}
                </TableCell>
                <TableCell width={70} className="text-right tabular-nums">
                  {t.members}
                </TableCell>
                <TableCell width={90} className="text-right tabular-nums">
                  {t.collections}
                </TableCell>
                <TableCell width={70}>
                  <Pill>{planName(t.plan)}</Pill>
                </TableCell>
                <TableCell width={210}>
                  <Pill tone={source.tone}>{source.label}</Pill>
                </TableCell>
                <TableCell width={80} className="text-right tabular-nums text-fg-muted">
                  {t.mrr_cents === null ? '–' : formatMoney(t.mrr_cents)}
                </TableCell>
              </TableRow>
            );
          })}
        </Table>
      )}
    </>
  );
}
