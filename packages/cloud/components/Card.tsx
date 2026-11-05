import type { HTMLAttributes } from 'react';
import { cx } from '@/lib/cx';

/** A hairline box. Give it `aria-label` or `aria-labelledby` when it's a section. */
export function Card({ className, children, ...rest }: HTMLAttributes<HTMLElement>) {
  return (
    <section
      className={cx('min-w-0 rounded-[8px] border border-hairline bg-bg', className)}
      {...rest}
    >
      {children}
    </section>
  );
}

export function CardBody({
  className,
  children,
}: {
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <div className={cx('flex flex-col gap-3 px-6 py-5 max-sm:px-4', className)}>{children}</div>
  );
}

export function CardTitle({
  as: Tag = 'h2',
  children,
  className,
  id,
}: {
  as?: 'h2' | 'h3';
  children: React.ReactNode;
  className?: string;
  id?: string;
}) {
  return (
    <Tag id={id} className={cx('m-0 text-[16px] leading-6 font-semibold', className)}>
      {children}
    </Tag>
  );
}

/** The fine print and one secondary action. */
export function CardFooter({
  className,
  children,
}: {
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <div
      className={cx(
        'box-border flex min-h-[52px] flex-wrap items-center gap-3 rounded-b-[8px] border-t border-hairline bg-raised px-6 py-2 text-[12px] text-fg-muted max-sm:px-4',
        className,
      )}
    >
      {children}
    </div>
  );
}
