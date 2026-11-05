import { type SelectHTMLAttributes, useId } from 'react';
import { cx } from '@/lib/cx';
import { inputClasses } from './field-styles';
import { ChevronDownIcon } from './icons';

export type SelectProps = SelectHTMLAttributes<HTMLSelectElement> & {
  /** Omit only when something else labels it (`aria-label`). */
  label?: React.ReactNode;
  options: { value: string; label: string }[];
};

/** A native select with the input look. */
export function Select({ label, options, id, className, ...rest }: SelectProps) {
  const auto = useId();
  const selectId = id ?? auto;
  return (
    <div className={cx('flex min-w-0 flex-col gap-1.5 text-[12px]', className)}>
      {label ? (
        <label htmlFor={selectId} className="text-fg-muted">
          {label}
        </label>
      ) : null}
      <span className="relative flex items-center">
        <select
          id={selectId}
          className={inputClasses(false, 'cursor-pointer appearance-none pr-8')}
          {...rest}
        >
          {options.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
        <ChevronDownIcon
          size={12}
          className="pointer-events-none absolute right-2.5 text-fg-muted"
        />
      </span>
    </div>
  );
}
