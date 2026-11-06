'use client';

import { useRouter } from 'next/navigation';
import { useState } from 'react';
import { describeEvent, formatInt } from '@/lib/admin';
import {
  ACTION_GROUPS,
  type ActionGroup,
  type ActivityItem,
  type ActivityPage,
  type ActivityQuery,
  activityExportHref,
  activityHref,
  detailPairs,
  SINCE_OPTIONS,
  type Since,
  utcStamp,
} from '@/lib/admin-activity';
import { nextPage, previousPage } from '@/lib/admin-paging';
import { Button } from '../Button';
import { ButtonLink } from '../ButtonLink';
import { buttonClasses } from '../button-styles';
import { EmptyState } from '../EmptyState';
import { PageHeader } from '../PageHeader';
import { Select } from '../Select';
import { Table, TableCell, TableHead, TableHeader, TableRow } from '../Table';

export type ActivityPerson = { id: string; label: string };

function who(item: ActivityItem): string {
  return item.admin ? item.admin.name || item.admin.email : 'Deleted admin';
}

function what(item: ActivityItem): string {
  return `${describeEvent(item.action)}${item.target_label ? ` ${item.target_label}` : ''}`;
}

function Detail({ item, label }: { item: ActivityItem; label: string }) {
  const pairs = detailPairs(item.detail);
  return (
    <div role="group" aria-label={label} className="border-b border-hairline bg-raised px-4 py-3">
      {pairs.length === 0 ? (
        <p className="m-0 text-[12px] text-fg-muted">No further details.</p>
      ) : (
        <dl className="m-0 grid grid-cols-[max-content_minmax(0,1fr)] gap-x-6 gap-y-1.5 text-[12px]">
          {pairs.map(([k, v], i) => (
            <div key={`${k}-${i}`} className="col-span-2 grid grid-cols-subgrid">
              <dt className="text-fg-muted">{k}</dt>
              <dd className="m-0 font-mono [overflow-wrap:anywhere] whitespace-pre-wrap">{v}</dd>
            </div>
          ))}
        </dl>
      )}
    </div>
  );
}

/** Every change an admin made, newest first. Filters and the page cursor live in the URL. */
export function ActivityView({
  page,
  people,
  query,
  pageSize,
}: {
  page: ActivityPage;
  /** Everyone who can be filtered on: the admins and owners with an account. */
  people: ActivityPerson[];
  query: ActivityQuery;
  pageSize: number;
}) {
  const router = useRouter();
  const [open, setOpen] = useState<string | null>(null);
  const { admin, group, since, cursor, trail } = query;
  const filters = { admin, group, since };
  const go = (next: Partial<typeof filters>) =>
    router.push(activityHref({ ...filters, ...next, trail: [] }));

  const items = page.items;
  const start = trail.length * pageSize + 1;
  const end = start + items.length - 1;

  return (
    <>
      <PageHeader
        title="Activity log"
        actions={
          <a href={activityExportHref(filters)} download className={buttonClasses()}>
            Export CSV
          </a>
        }
      >
        Every change an admin makes, kept for 2 years. Customers’ own actions are on each user’s
        page.
      </PageHeader>

      <div className="flex flex-wrap items-end gap-3">
        <Select
          aria-label="Admin"
          value={admin ?? ''}
          onChange={(e) => go({ admin: e.target.value || undefined })}
          options={[
            { value: '', label: 'All admins' },
            ...people.map((p) => ({ value: p.id, label: p.label })),
          ]}
          className="w-[190px]"
        />
        <Select
          aria-label="Action"
          value={group}
          onChange={(e) => go({ group: e.target.value as ActionGroup })}
          options={ACTION_GROUPS.map((g) => ({ value: g.value, label: g.label }))}
          className="w-[170px]"
        />
        <Select
          aria-label="Period"
          value={since}
          onChange={(e) => go({ since: e.target.value as Since })}
          options={SINCE_OPTIONS.map((s) => ({ value: s.value, label: s.label }))}
          className="w-[160px]"
        />
      </div>

      {items.length === 0 ? (
        <EmptyState title="No activity matches">
          {since === 'all' && group === 'all' && !admin
            ? 'Changes admins make show up here.'
            : 'Try another admin, action or period.'}
        </EmptyState>
      ) : (
        <Table
          label="Admin activity"
          minWidth={760}
          footer={
            <>
              <span className="mr-auto">
                Showing {formatInt(start)}–{formatInt(end)} of {formatInt(page.total)}
              </span>
              <span className="flex gap-1.5">
                {trail.length > 0 ? (
                  <ButtonLink
                    size="sm"
                    href={activityHref({
                      ...filters,
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
                    href={activityHref({
                      ...filters,
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
            <TableHead width={104}>Time (UTC)</TableHead>
            <TableHead width={150}>Admin</TableHead>
            <TableHead>Action</TableHead>
            <TableHead width={120}>IP</TableHead>
            <TableHead width={32} />
          </TableHeader>
          {items.map((item) => {
            const text = what(item);
            const expanded = open === item.id;
            const label = `Details for ${text}`;
            return (
              <div key={item.id} role="rowgroup">
                <TableRow className="min-h-12 py-1.5">
                  <TableCell width={104} className="font-mono text-[12px] text-fg-muted">
                    {utcStamp(item.at)}
                  </TableCell>
                  <TableCell width={150} className="truncate">
                    {who(item)}
                  </TableCell>
                  <TableCell className="[overflow-wrap:anywhere]">
                    <span className="text-fg-muted">{describeEvent(item.action)}</span>
                    {item.target_label ? (
                      <>
                        {' '}
                        <span className="font-mono text-[12px]">{item.target_label}</span>
                      </>
                    ) : null}
                  </TableCell>
                  <TableCell width={120} className="font-mono text-[12px] text-fg-muted">
                    {item.ip ?? '—'}
                  </TableCell>
                  <TableCell width={32}>
                    <Button
                      size="sm"
                      variant="ghost"
                      aria-label={label}
                      aria-expanded={expanded}
                      className="w-7 px-0"
                      onClick={() => setOpen(expanded ? null : item.id)}
                    >
                      {expanded ? '−' : '+'}
                    </Button>
                  </TableCell>
                </TableRow>
                {expanded ? <Detail item={item} label={label} /> : null}
              </div>
            );
          })}
        </Table>
      )}
    </>
  );
}
