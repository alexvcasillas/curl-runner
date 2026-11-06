import { cx } from '@/lib/cx';

export type StatTone = 'ok' | 'muted' | 'warn' | 'err';

const TONE: Record<StatTone, string> = {
  ok: 'text-success',
  muted: 'text-fg-muted',
  warn: 'text-warning',
  err: 'text-danger',
};

/** A label, a big number and one line under it. */
export function StatTile({
  label,
  value,
  sub,
  tone = 'muted',
  className,
}: {
  label: string;
  value: string;
  sub?: string;
  tone?: StatTone;
  className?: string;
}) {
  return (
    <div
      className={cx('flex flex-col gap-0.5 border-r border-b border-hairline px-5 py-4', className)}
    >
      <span className="text-[12px] text-fg-muted">{label}</span>
      <span className="text-[24px] leading-8 font-semibold tracking-[-0.02em] tabular-nums">
        {value}
      </span>
      {sub ? <span className={cx('text-[12px]', TONE[tone])}>{sub}</span> : null}
    </div>
  );
}
