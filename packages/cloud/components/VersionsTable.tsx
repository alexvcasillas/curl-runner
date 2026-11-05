'use client';

import { useState } from 'react';
import { Button } from './Button';
import { ButtonLink } from './ButtonLink';
import { buttonClasses } from './button-styles';
import { DownloadIcon } from './icons';
import { Pill } from './Pill';
import { Table, TableCell, TableHead, TableHeader, TableRow } from './Table';

export type VersionRow = {
  number: number;
  current: boolean;
  /** "2 hours ago". */
  when: string;
  /** The exact time, for the tooltip. */
  abs: string;
  by: string;
  /** Short SHA, when the publish carried git info. */
  sha: string | null;
  /** Only for github.com and gitlab.com. */
  shaHref: string | null;
  ref: string | null;
  files: number;
  changesHref: string;
  downloadHref: string;
};

const PAGE = 10;

/** A collection's versions, newest first. Ten at a time, older ones on request. */
export function VersionsTable({
  rows,
  note,
}: {
  rows: VersionRow[];
  /** Fine print after the count. */
  note?: string;
}) {
  const [shown, setShown] = useState(PAGE);
  const visible = rows.slice(0, shown);
  const more = rows.length > shown;

  return (
    <Table
      label="Versions"
      minWidth={820}
      footer={
        <>
          <span className="min-w-0 flex-1">
            {rows.length} {rows.length === 1 ? 'version' : 'versions'}. Cloud keeps every one.
            {note ? ` ${note}` : ''}
          </span>
          {more ? (
            <Button size="sm" onClick={() => setShown((n) => n + PAGE)}>
              Show older
            </Button>
          ) : null}
        </>
      }
    >
      <TableHeader>
        <TableHead width={120}>Version</TableHead>
        <TableHead width={150}>Published</TableHead>
        <TableHead>By</TableHead>
        <TableHead width={180}>Commit</TableHead>
        <TableHead width={60} className="text-right">
          Files
        </TableHead>
        <TableHead width={108}>
          <span className="sr-only">Actions</span>
        </TableHead>
      </TableHeader>
      {visible.map((v) => (
        <TableRow key={v.number}>
          <TableCell width={120} className="flex items-center gap-2">
            <span className="font-mono font-medium">v{v.number}</span>
            {v.current ? (
              <Pill tone="ok" size="sm">
                Current
              </Pill>
            ) : null}
          </TableCell>
          <TableCell width={150} className="text-fg-muted">
            <span title={v.abs}>{v.when}</span>
          </TableCell>
          <TableCell className="truncate text-fg-muted">{v.by}</TableCell>
          <TableCell width={180} className="text-[12px]">
            {v.sha ? (
              <>
                {v.shaHref ? (
                  <a href={v.shaHref} className="font-mono text-[12px]">
                    {v.sha}
                  </a>
                ) : (
                  <span className="font-mono text-[12px] text-fg-muted">{v.sha}</span>
                )}{' '}
                {v.ref ? <span className="font-mono text-fg-subtle">{v.ref}</span> : null}
              </>
            ) : (
              <span className="text-fg-subtle">None</span>
            )}
          </TableCell>
          <TableCell width={60} className="text-right font-mono text-[12px] text-fg-muted">
            {v.files}
          </TableCell>
          <TableCell width={108} className="flex justify-end gap-1">
            <ButtonLink href={v.changesHref} size="sm">
              Changes
            </ButtonLink>
            {/* A plain link: the router would prefetch the zip. */}
            <a
              href={v.downloadHref}
              aria-label={`Download v${v.number} as a zip`}
              className={buttonClasses('ghost', 'sm', 'w-7 px-0!')}
            >
              <DownloadIcon size={14} />
            </a>
          </TableCell>
        </TableRow>
      ))}
    </Table>
  );
}
