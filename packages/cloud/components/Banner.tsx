import { cx } from '@/lib/cx';
import { TONE_BOX, type Tone, ToneIcon } from './tones';

export type BannerProps = {
  tone: Exclude<Tone, 'ok'>;
  /** The first sentence, in medium weight. */
  title: React.ReactNode;
  /** What to do, in secondary text. */
  children?: React.ReactNode;
  /** One small button or link: "Update card". */
  action?: React.ReactNode;
  className?: string;
};

/** Sits at the top of a team page until its cause is gone. One at a time. */
export function Banner({ tone, title, children, action, className }: BannerProps) {
  return (
    <div
      role={tone === 'err' ? 'alert' : 'status'}
      className={cx(
        'flex flex-wrap items-start gap-3 rounded-[8px] border px-4 py-3 text-[13px] leading-5',
        TONE_BOX[tone],
        className,
      )}
    >
      <span className="mt-[3px] flex">
        <ToneIcon tone={tone} />
      </span>
      <div className="flex min-w-0 flex-1 basis-[200px] flex-col gap-0.5">
        <span className="font-medium">{title}</span>
        {children ? <span className="text-fg-muted">{children}</span> : null}
      </div>
      {action}
    </div>
  );
}
