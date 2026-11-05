import Link from 'next/link';
import type { ComponentProps, CSSProperties } from 'react';
import { cx } from '@/lib/cx';
import { Card, CardFooter } from './Card';
import { Skeleton } from './Skeleton';

export type TableProps = {
  /** Names the table for screen readers: "Collections". */
  label: string;
  /** Below this width the table scrolls sideways inside its card. */
  minWidth?: number;
  /** Shown in the card footer: "4 collections". */
  footer?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
};

/**
 * Rows of facts in a card, the way the boards draw them: a 36px header, 60px
 * rows with hairlines. Rows can be links (`TableRowLink`).
 */
export function Table({ label, minWidth = 720, footer, children, className }: TableProps) {
  return (
    <Card aria-label={label} className={className}>
      <div className="overflow-x-auto">
        <div role="table" aria-label={label} style={{ minWidth }}>
          {children}
        </div>
      </div>
      {footer ? <CardFooter>{footer}</CardFooter> : null}
    </Card>
  );
}

type CellProps = {
  /** Fixed width in px. Without it the cell takes the free space. */
  width?: number;
  className?: string;
  children?: React.ReactNode;
};

function cellStyle(width?: number): CSSProperties | undefined {
  return width === undefined ? undefined : { width, flexShrink: 0 };
}

export function TableHeader({ children }: { children: React.ReactNode }) {
  return (
    <div
      role="row"
      className="flex h-9 items-center gap-3 border-b border-hairline px-4 text-[12px] text-fg-subtle"
    >
      {children}
    </div>
  );
}

export function TableHead({ width, className, children }: CellProps) {
  return (
    <span
      role="columnheader"
      className={cx(width === undefined && 'min-w-0 flex-1', className)}
      style={cellStyle(width)}
    >
      {children}
    </span>
  );
}

export function TableRow({
  className,
  children,
}: {
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <div
      role="row"
      className={cx(
        'relative flex min-h-[60px] items-center gap-3 border-b border-hairline px-4 text-fg transition-colors duration-[120ms] last:border-b-0 has-[a[data-row-link]]:hover:bg-surface-100',
        className,
      )}
    >
      {children}
    </div>
  );
}

export function TableCell({ width, className, children }: CellProps) {
  return (
    <span
      role="cell"
      className={cx('min-w-0', width === undefined && 'flex-1', className)}
      style={cellStyle(width)}
    >
      {children}
    </span>
  );
}

/** The row's main link. It covers the whole row, so the row is one click target. */
export function TableRowLink({ className, children, ...rest }: ComponentProps<typeof Link>) {
  return (
    <Link
      data-row-link=""
      className={cx(
        'font-medium text-fg after:absolute after:inset-0 after:content-[""] hover:text-fg focus-visible:shadow-none focus-visible:after:shadow-[inset_0_0_0_2px_#52a8ff]',
        className,
      )}
      {...rest}
    >
      {children}
    </Link>
  );
}

/** Skeleton rows while a list loads. Never a spinner for a list. */
export function TableLoading({ rows = 4 }: { rows?: number }) {
  return (
    <>
      <span role="status" className="sr-only">
        Loading
      </span>
      {Array.from({ length: rows }, (_, i) => (
        <div
          key={i}
          aria-hidden="true"
          className="flex items-center gap-4 border-b border-hairline p-4 last:border-b-0"
        >
          <span className="flex flex-1 flex-col gap-2">
            <Skeleton width={`${50 - (i % 3) * 5}%`} />
            <Skeleton width={`${30 - (i % 3) * 5}%`} height={8} />
          </span>
          <Skeleton width={60} />
        </div>
      ))}
    </>
  );
}
