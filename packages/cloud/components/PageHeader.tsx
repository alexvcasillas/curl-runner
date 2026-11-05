import { cx } from '@/lib/cx';

/** A page's title, one line about it and the page's actions. */
export function PageHeader({
  title,
  children,
  actions,
  className,
}: {
  title: React.ReactNode;
  children?: React.ReactNode;
  actions?: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={cx('flex flex-wrap items-center gap-3', className)}>
      <div className="mr-auto min-w-0">
        <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em]">{title}</h1>
        {children ? <p className="mt-1 mb-0 text-fg-muted">{children}</p> : null}
      </div>
      {actions}
    </div>
  );
}
