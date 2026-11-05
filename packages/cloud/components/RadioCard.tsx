import type { InputHTMLAttributes } from 'react';
import { cx } from '@/lib/cx';

export type RadioCardProps = Omit<InputHTMLAttributes<HTMLInputElement>, 'type'> & {
  title: React.ReactNode;
  description?: React.ReactNode;
};

/**
 * One choice of a radio group, as a card: "Public, anyone with the link".
 * Group several with the same `name` inside a `fieldset`.
 */
export function RadioCard({ title, description, className, ...rest }: RadioCardProps) {
  return (
    <label
      className={cx(
        'flex cursor-pointer gap-3 rounded-[8px] border border-hairline bg-bg px-3.5 py-3 transition-colors duration-[120ms] hover:border-control',
        'has-checked:border-fg has-checked:bg-surface-200 has-focus-visible:shadow-focus has-disabled:cursor-not-allowed has-disabled:opacity-60',
        className,
      )}
    >
      <input type="radio" className="peer sr-only" {...rest} />
      <span
        aria-hidden="true"
        className="mt-0.5 box-border size-4 shrink-0 rounded-full border border-strong peer-checked:border-[5px] peer-checked:border-fg"
      />
      <span className="flex min-w-0 flex-col">
        <span className="font-medium text-fg">{title}</span>
        {description ? <span className="text-[12px] text-fg-muted">{description}</span> : null}
      </span>
    </label>
  );
}
