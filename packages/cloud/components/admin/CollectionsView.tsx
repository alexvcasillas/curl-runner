'use client';

import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useEffect, useRef, useState } from 'react';
import { adminRoutes, can, formatInt, type Perm } from '@/lib/admin';
import {
  accessLabel,
  COLLECTION_FILTERS,
  type CollectionFilter,
  type CollectionRow,
  type CollectionStats,
  type CollectionsPage,
  collectionsExportHref,
  collectionsHref,
  formatBytes,
} from '@/lib/admin-collections';
import { nextPage, previousPage } from '@/lib/admin-paging';
import { relativeTime } from '@/lib/time';
import { Button } from '../Button';
import { ButtonLink } from '../ButtonLink';
import { buttonClasses } from '../button-styles';
import { EmptyState } from '../EmptyState';
import { inputClasses } from '../field-styles';
import { Notice } from '../Notice';
import { PageHeader } from '../PageHeader';
import { Pill } from '../Pill';
import { Segmented } from '../Segmented';
import { Table, TableCell, TableHead, TableHeader, TableRow } from '../Table';
import type { CollectionActions } from './collection-actions';
import { DisableLinkDialog } from './DisableLinkDialog';
import { UnlistDialog } from './UnlistDialog';

const SEARCH_DELAY = 350;

const NO_PERM = 'You can read this page but not change it.';

/**
 * Every collection on the server. Reads are open to any admin; taking a share
 * link down or putting it back needs `teams`.
 */
export function CollectionsView({
  page,
  stats,
  q,
  filter,
  cursor,
  trail,
  pageSize,
  now,
  permissions,
  actions,
}: {
  page: CollectionsPage;
  stats: CollectionStats;
  q: string;
  filter: CollectionFilter;
  cursor?: string;
  trail: string[];
  pageSize: number;
  now: string;
  permissions: Perm[];
  actions: CollectionActions;
}) {
  const router = useRouter();
  const clock = new Date(now);
  const canTeams = can(permissions, 'teams');
  const [search, setSearch] = useState(q);
  const [target, setTarget] = useState<CollectionRow | null>(null);
  const [unlisting, setUnlisting] = useState<CollectionRow | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  // Search as they type, after a pause. Enter goes now.
  const lastQ = useRef(q);
  lastQ.current = q;
  useEffect(() => {
    const next = search.trim();
    if (next === lastQ.current) {
      return;
    }
    const timer = setTimeout(
      () => router.replace(collectionsHref({ q: next, filter, trail: [] })),
      SEARCH_DELAY,
    );
    return () => clearTimeout(timer);
  }, [search, filter, router]);

  async function enable(c: CollectionRow) {
    const key = `${c.team}/${c.slug}`;
    setBusy(key);
    setNote(null);
    setProblem(null);
    try {
      const r = await actions.enable(c.team, c.slug);
      if (r.ok) {
        setNote(`Enabled the share link of ${c.name}.`);
        router.refresh();
      } else {
        setProblem(r.message);
      }
    } catch {
      setProblem('That didn’t work. Try again.');
    } finally {
      setBusy(null);
    }
  }

  const items = page.items;
  const start = trail.length * pageSize + 1;
  const end = start + items.length - 1;
  const filtered = q !== '' || filter !== 'all';

  return (
    <>
      <PageHeader
        title="Collections"
        actions={
          <a href={collectionsExportHref({ q, filter })} download className={buttonClasses()}>
            Export CSV
          </a>
        }
      >
        Every collection on the server. Disable a share link when it’s abused: the team keeps the
        collection, and customers see “This link no longer works”.
      </PageHeader>

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

      <div className="flex flex-wrap items-center gap-3">
        <form
          role="search"
          className="relative flex min-w-[220px] max-w-[340px] flex-1 items-center"
          onSubmit={(e) => {
            e.preventDefault();
            router.push(collectionsHref({ q: search.trim(), filter, trail: [] }));
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
            aria-label="Search collections"
            placeholder="Search by name, slug or team"
            value={search}
            maxLength={200}
            autoComplete="off"
            onChange={(e) => setSearch(e.target.value)}
            className={inputClasses(false, 'pl-8')}
          />
        </form>
        <Segmented
          label="Filter collections"
          options={COLLECTION_FILTERS.map((f) => ({ value: f.value, label: f.label }))}
          value={filter}
          onChange={(value) => router.push(collectionsHref({ q, filter: value, trail: [] }))}
        />
      </div>

      {items.length === 0 ? (
        <EmptyState
          title={filtered ? 'No collections match' : 'No collections yet'}
          actions={
            filtered ? <ButtonLink href={adminRoutes.collections}>Clear search</ButtonLink> : null
          }
        >
          {filtered
            ? 'Try another name, slug, team or filter.'
            : 'Collections show up here once a team publishes one.'}
        </EmptyState>
      ) : (
        <Table
          label="Collections"
          minWidth={960}
          footer={
            <>
              <span className="mr-auto">
                {formatInt(stats.count)} {stats.count === 1 ? 'collection' : 'collections'} ·{' '}
                {formatBytes(stats.bytes_raw)} of YAML, de-duplicated to{' '}
                {formatBytes(stats.bytes_stored)}
                <span className="ml-3 text-fg-subtle">
                  Showing {formatInt(start)}–{formatInt(end)} of {formatInt(page.total)}
                </span>
              </span>
              <span className="flex gap-1.5">
                {trail.length > 0 ? (
                  <ButtonLink
                    size="sm"
                    href={collectionsHref({
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
                    href={collectionsHref({
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
            <TableHead>Collection</TableHead>
            <TableHead width={100}>Team</TableHead>
            <TableHead width={84}>Access</TableHead>
            <TableHead width={64} className="text-right">
              Versions
            </TableHead>
            <TableHead width={64} className="text-right">
              Size
            </TableHead>
            <TableHead width={110} className="text-right">
              Signed-in opens (30d)
            </TableHead>
            <TableHead width={100}>Last publish</TableHead>
            <TableHead width={160}>
              <span className="sr-only">Actions</span>
            </TableHead>
          </TableHeader>
          {items.map((c) => {
            const key = `${c.team}/${c.slug}`;
            return (
              <TableRow key={key} className="min-h-[52px] py-2">
                <TableCell>
                  <span className="flex min-w-0 flex-col">
                    <span className="flex flex-wrap items-center gap-2">
                      <span className="font-medium [overflow-wrap:anywhere]">{c.name}</span>
                      {c.link_disabled ? <Pill tone="err">Link disabled</Pill> : null}
                      {c.listed_at ? <Pill tone="info">Listed</Pill> : null}
                    </span>
                    <span className="font-mono text-[12px] text-fg-subtle [overflow-wrap:anywhere]">
                      {c.slug} · {c.source}
                    </span>
                  </span>
                </TableCell>
                <TableCell width={100} className="[overflow-wrap:anywhere]">
                  <Link href={adminRoutes.team(c.team)} className="text-fg hover:text-fg">
                    {c.team}
                  </Link>
                </TableCell>
                <TableCell width={84}>
                  <Pill>{accessLabel(c.access)}</Pill>
                </TableCell>
                <TableCell width={64} className="text-right tabular-nums">
                  {c.versions}
                </TableCell>
                <TableCell width={64} className="text-right tabular-nums text-fg-muted">
                  {formatBytes(c.size_bytes)}
                </TableCell>
                <TableCell width={110} className="text-right tabular-nums text-fg-muted">
                  {formatInt(c.opens_30d)}
                </TableCell>
                <TableCell width={100} className="text-fg-muted">
                  {c.last_publish ? relativeTime(c.last_publish, clock) : 'Never'}
                </TableCell>
                <TableCell width={160} className="flex justify-end gap-1.5">
                  {c.listed_at ? (
                    <Button
                      size="sm"
                      disabled={!canTeams}
                      title={canTeams ? undefined : NO_PERM}
                      onClick={() => {
                        setNote(null);
                        setProblem(null);
                        setUnlisting(c);
                      }}
                    >
                      Unlist
                    </Button>
                  ) : null}
                  {c.link_disabled ? (
                    <Button
                      size="sm"
                      disabled={!canTeams}
                      loading={busy === key}
                      title={canTeams ? undefined : NO_PERM}
                      onClick={() => enable(c)}
                    >
                      Enable link
                    </Button>
                  ) : c.access === 'off' ? (
                    <ButtonLink size="sm" href={adminRoutes.team(c.team)}>
                      Versions
                    </ButtonLink>
                  ) : (
                    <Button
                      size="sm"
                      disabled={!canTeams}
                      title={canTeams ? undefined : NO_PERM}
                      onClick={() => {
                        setNote(null);
                        setProblem(null);
                        setTarget(c);
                      }}
                    >
                      Disable link
                    </Button>
                  )}
                </TableCell>
              </TableRow>
            );
          })}
        </Table>
      )}

      <DisableLinkDialog
        collection={target}
        onClose={() => setTarget(null)}
        onSubmit={(reason) =>
          target ? actions.disable(target.team, target.slug, reason) : Promise.reject()
        }
        onDone={(c) => {
          setNote(`Disabled the share link of ${c.name}.`);
          router.refresh();
        }}
      />

      <UnlistDialog
        collection={unlisting}
        onClose={() => setUnlisting(null)}
        onSubmit={(reason) =>
          unlisting ? actions.unlist(unlisting.team, unlisting.slug, reason) : Promise.reject()
        }
        onDone={(c) => {
          setNote(`Unlisted ${c.name} from the gallery.`);
          router.refresh();
        }}
      />
    </>
  );
}
