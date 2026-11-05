import { cx } from '@/lib/cx';

/** A request in flight. Only inside buttons and short waits; lists use skeletons. */
export function Spinner({ size = 14, className }: { size?: 14 | 20; className?: string }) {
  return (
    <span
      aria-hidden="true"
      className={cx(
        'inline-block shrink-0 animate-spin rounded-full border-2 border-control border-t-fg',
        size === 14 ? 'size-3.5' : 'size-5',
        className,
      )}
    />
  );
}
