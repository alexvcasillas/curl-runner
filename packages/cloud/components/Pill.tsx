import { cx } from '@/lib/cx';

export type PillTone = 'neutral' | 'strong' | 'muted' | 'ok' | 'warn' | 'err' | 'info';

const TONES: Record<PillTone, string> = {
  // Source, access and plan are neutral facts.
  neutral: 'border-control text-fg-muted',
  strong: 'border-control text-fg',
  muted: 'border-control text-fg-subtle',
  // Only states that need attention get colour.
  ok: 'border-success-border bg-success-bg text-success',
  warn: 'border-warning-border bg-warning-bg text-warning',
  err: 'border-danger-border bg-danger-bg text-danger',
  info: 'border-info-border bg-info-bg text-info',
};

export type PillProps = {
  tone?: PillTone;
  /** `sm` sits inline with a name: the plan in the header and menus. */
  size?: 'sm' | 'md';
  icon?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
};

/** A short fact or state: "Team", "Public", "Past due". */
export function Pill({ tone = 'neutral', size = 'md', icon, children, className }: PillProps) {
  return (
    <span
      className={cx(
        'box-border inline-flex shrink-0 items-center gap-1.5 whitespace-nowrap rounded-full border font-medium',
        size === 'md' ? 'h-5 px-2 text-[12px]' : 'h-[18px] px-[7px] text-[11px]',
        TONES[tone],
        className,
      )}
    >
      {icon}
      {children}
    </span>
  );
}
