import type { CSSProperties } from 'react';
import { cx } from '@/lib/cx';

/** A placeholder bar. Lists only: never a spinner for a list. */
export function Skeleton({
  width,
  height = 10,
  className,
}: {
  width?: CSSProperties['width'];
  height?: 8 | 10;
  className?: string;
}) {
  return (
    <span
      aria-hidden="true"
      className={cx('block rounded-[4px] bg-skeleton', className)}
      style={{ width, height }}
    />
  );
}
