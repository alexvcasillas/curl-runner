import { cx } from '@/lib/cx';
import { TONE_BOX, type Tone, ToneIcon } from './tones';

export type NoticeProps = {
  tone?: Tone;
  children: React.ReactNode;
  /** `alert` interrupts screen readers; use it for errors that just happened. */
  role?: 'alert' | 'status';
  className?: string;
};

/** A callout inside a card, form or dialog. Banners across a page use `Banner`. */
export function Notice({ tone = 'info', children, role, className }: NoticeProps) {
  return (
    <div
      role={role}
      className={cx(
        'flex items-start gap-2.5 rounded-[6px] border px-3 py-2.5 text-[12px] leading-[18px]',
        TONE_BOX[tone],
        className,
      )}
    >
      <span className="mt-0.5 flex">
        <ToneIcon tone={tone} />
      </span>
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
}
