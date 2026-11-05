import { cx } from '@/lib/cx';

export type EmptyStateProps = {
  /** A 16px glyph, framed in a 40px box. */
  icon?: React.ReactNode;
  /** A status line above the title, like `404 Not Found`. */
  code?: string;
  title: React.ReactNode;
  children?: React.ReactNode;
  /** The one action that fills the screen. */
  actions?: React.ReactNode;
  /** `h1` when the empty state is the whole page. */
  as?: 'h1' | 'h2' | 'h3';
  className?: string;
};

/** An empty list or an error page: what's missing and the one thing to do. */
export function EmptyState({
  icon,
  code,
  title,
  children,
  actions,
  as: Heading = 'h2',
  className,
}: EmptyStateProps) {
  return (
    <div
      className={cx(
        'flex flex-col items-center gap-2.5 rounded-[8px] border border-hairline bg-bg px-6 py-10 text-center',
        className,
      )}
    >
      {icon ? (
        <span
          aria-hidden="true"
          className="grid size-10 place-items-center rounded-[10px] border border-control text-fg-muted"
        >
          {icon}
        </span>
      ) : null}
      {code ? <span className="font-mono text-[12px] text-fg-subtle">{code}</span> : null}
      <Heading className="m-0 text-[16px] leading-6 font-semibold">{title}</Heading>
      {children ? <p className="m-0 max-w-[400px] text-fg-muted">{children}</p> : null}
      {actions ? <div className="flex flex-wrap justify-center gap-2">{actions}</div> : null}
    </div>
  );
}
