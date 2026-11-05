'use client';

import { useState } from 'react';
import { cx } from '@/lib/cx';
import type { AccessMode, Source } from '@/lib/types';
import { AccessBadge, SourceBadge } from './Badge';
import { inputClasses } from './field-styles';
import { ChevronRightIcon, SearchIcon } from './icons';
import { Segmented } from './Segmented';
import { Table, TableCell, TableHead, TableHeader, TableRow, TableRowLink } from './Table';

export type CollectionRow = {
  slug: string;
  name: string;
  source: Source;
  access: AccessMode;
  /** "v14", or null before the first publish. */
  version: string | null;
  /** Relative: "2 hours ago". Null before the first publish. */
  when: string | null;
  /** The exact time, for the tooltip. */
  abs: string | null;
  by: string | null;
  href: string;
};

type Filter = 'all' | AccessMode;

const FILTERS: { value: Filter; label: string }[] = [
  { value: 'all', label: 'All' },
  { value: 'public', label: 'Public' },
  { value: 'restricted', label: 'Restricted' },
  { value: 'off', label: 'Off' },
];

/** The team's collections, with a search box and an access filter. */
export function CollectionsView({ rows, note }: { rows: CollectionRow[]; note: string }) {
  const [filter, setFilter] = useState<Filter>('all');
  const [query, setQuery] = useState('');
  const q = query.trim().toLowerCase();
  const shown = rows.filter(
    (r) =>
      (filter === 'all' || r.access === filter) &&
      (!q || r.name.toLowerCase().includes(q) || r.slug.toLowerCase().includes(q)),
  );
  const count = (n: number) => `${n} ${n === 1 ? 'collection' : 'collections'}`;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <span className="relative flex min-w-[220px] max-w-[320px] flex-1 items-center max-sm:max-w-none">
          <SearchIcon size={14} className="pointer-events-none absolute left-2.5 text-fg-subtle" />
          <input
            type="search"
            aria-label="Search collections"
            placeholder="Search collections"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            autoComplete="off"
            className={inputClasses(false, 'pl-8')}
          />
        </span>
        <Segmented label="Filter by access" options={FILTERS} value={filter} onChange={setFilter} />
      </div>

      <Table
        label="Collections"
        minWidth={820}
        footer={
          <>
            <span className="flex-1">
              {shown.length === rows.length
                ? count(rows.length)
                : `${shown.length} of ${count(rows.length)}`}
            </span>
            <span>{note}</span>
          </>
        }
      >
        <TableHeader>
          <TableHead>Collection</TableHead>
          <TableHead width={96}>Source</TableHead>
          <TableHead width={120}>Access</TableHead>
          <TableHead width={72}>Version</TableHead>
          <TableHead width={220}>Last publish</TableHead>
          <TableHead width={16}>
            <span className="sr-only">Open</span>
          </TableHead>
        </TableHeader>
        {shown.length === 0 ? (
          <div role="row" className="px-4 py-8 text-center text-fg-muted">
            <span role="cell">No collections match.</span>
          </div>
        ) : null}
        {shown.map((r) => (
          <TableRow key={r.slug}>
            <TableCell className="flex flex-col">
              <TableRowLink href={r.href}>{r.name}</TableRowLink>
              <span className="truncate font-mono text-[12px] text-fg-subtle">{r.slug}</span>
            </TableCell>
            <TableCell width={96}>
              <SourceBadge source={r.source} />
            </TableCell>
            <TableCell width={120}>
              <AccessBadge access={r.access} />
            </TableCell>
            <TableCell width={72} className="font-mono text-[12px]">
              {r.version ?? <span className="text-fg-subtle">None</span>}
            </TableCell>
            <TableCell width={220} className="flex flex-col text-[12px] leading-4">
              {r.when ? (
                <>
                  <span title={r.abs ?? undefined}>{r.when}</span>
                  <span className={cx('truncate text-fg-subtle')}>{r.by}</span>
                </>
              ) : (
                <span className="text-fg-subtle">Not published</span>
              )}
            </TableCell>
            <TableCell width={16} className="flex text-fg-subtle">
              <ChevronRightIcon size={14} />
            </TableCell>
          </TableRow>
        ))}
      </Table>
    </div>
  );
}
