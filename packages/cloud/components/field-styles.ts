import { cx } from '@/lib/cx';

/** The text input look, shared by Field, Select and search boxes. */
export function inputClasses(invalid = false, className?: string): string {
  return cx(
    'box-border h-8 w-full min-w-0 rounded-[6px] border bg-bg px-2.5 text-[13px] text-fg outline-none placeholder:text-fg-subtle',
    'focus-visible:border-blue disabled:cursor-not-allowed disabled:text-fg-disabled',
    invalid ? 'border-danger' : 'border-control',
    className,
  );
}
