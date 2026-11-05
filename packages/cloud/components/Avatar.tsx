import { cx } from '@/lib/cx';
import { initials, teamLetter } from '@/lib/initials';

export type AvatarProps = {
  /** A name or email. People get two initials, teams one letter. */
  name?: string;
  /** People are round, teams are square, a pending invite is a dashed circle. */
  kind?: 'person' | 'team' | 'pending';
  size?: 20 | 22 | 28 | 32;
  /** The current team in the switcher menu. */
  active?: boolean;
  className?: string;
};

const SIZE = { 20: 'size-5', 22: 'size-[22px]', 28: 'size-7', 32: 'size-8' } as const;

/** Initials, no photos or gradients. Decorative: the name sits next to it. */
export function Avatar({ name = '', kind = 'person', size = 32, active, className }: AvatarProps) {
  const base = cx('box-border inline-grid shrink-0 place-items-center', SIZE[size], className);
  if (kind === 'pending') {
    return (
      <span
        aria-hidden="true"
        className={cx(base, 'rounded-full border border-dashed border-strong')}
      />
    );
  }
  if (kind === 'team') {
    return (
      <span
        aria-hidden="true"
        className={cx(
          base,
          'font-semibold text-fg',
          size === 32 ? 'rounded-[8px] text-[13px]' : 'rounded-[5px] text-[11px]',
          active ? 'bg-control' : 'bg-surface-300',
        )}
      >
        {teamLetter(name)}
      </span>
    );
  }
  return (
    <span
      aria-hidden="true"
      className={cx(
        base,
        'rounded-full bg-surface-300 font-medium text-fg-muted',
        size === 32 ? 'text-[12px]' : 'text-[11px]',
      )}
    >
      {initials(name)}
    </span>
  );
}
