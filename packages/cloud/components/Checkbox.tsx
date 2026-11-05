import type { InputHTMLAttributes } from 'react';
import { cx } from '@/lib/cx';

export type CheckboxProps = Omit<InputHTMLAttributes<HTMLInputElement>, 'type'> & {
  label: React.ReactNode;
};

/** A native checkbox drawn the studio way: a light box with a dark tick. */
export function Checkbox({ label, className, ...rest }: CheckboxProps) {
  return (
    <label
      className={cx(
        'inline-flex cursor-pointer items-center gap-2 text-[12px] text-fg-muted has-disabled:cursor-not-allowed',
        className,
      )}
    >
      <input type="checkbox" className="peer sr-only" {...rest} />
      <span
        aria-hidden="true"
        className="box-border inline-flex size-4 shrink-0 items-center justify-center rounded-[4px] border border-strong text-transparent peer-checked:border-fg peer-checked:bg-fg peer-checked:text-on-primary peer-focus-visible:shadow-focus"
      >
        <svg
          width="10"
          height="10"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M3 8.5l3 3 7-7" />
        </svg>
      </span>
      {label}
    </label>
  );
}
