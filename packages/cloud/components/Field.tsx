import { type InputHTMLAttributes, useId } from 'react';
import { cx } from '@/lib/cx';
import { inputClasses } from './field-styles';

export type FieldProps = Omit<InputHTMLAttributes<HTMLInputElement>, 'prefix'> & {
  label: React.ReactNode;
  /** What went wrong and what to do. Marks the input invalid. */
  error?: string | null;
  hint?: React.ReactNode;
  /** A fixed start, like `cloud.curl-runner.com/t/` before a slug. */
  prefix?: string;
  /** Geist Mono, for anything people paste or send. */
  mono?: boolean;
  ref?: React.Ref<HTMLInputElement>;
};

/** A labelled text input, with its error or hint underneath. */
export function Field({
  label,
  error,
  hint,
  prefix,
  mono = false,
  id,
  className,
  ...rest
}: FieldProps) {
  const auto = useId();
  const inputId = id ?? auto;
  const noteId = `${inputId}-note`;
  const note = error || hint;
  const input = (
    <input
      id={inputId}
      aria-invalid={error ? true : undefined}
      aria-describedby={note ? noteId : undefined}
      className={inputClasses(
        Boolean(error),
        cx(mono && 'font-mono text-[12px]', prefix && 'rounded-l-none'),
      )}
      {...rest}
    />
  );
  return (
    <div className={cx('flex min-w-0 flex-col gap-1.5 text-[12px]', className)}>
      <label htmlFor={inputId} className="text-fg-muted">
        {label}
      </label>
      {prefix ? (
        <div className="flex items-stretch">
          <span className="flex items-center rounded-l-[6px] border border-r-0 border-control bg-raised px-2.5 font-mono text-[12px] whitespace-nowrap text-fg-subtle max-sm:max-w-[55%] max-sm:overflow-hidden max-sm:text-ellipsis">
            {prefix}
          </span>
          {input}
        </div>
      ) : (
        input
      )}
      {note ? (
        <span id={noteId} className={error ? 'text-danger' : 'text-fg-subtle'}>
          {note}
        </span>
      ) : null}
    </div>
  );
}
