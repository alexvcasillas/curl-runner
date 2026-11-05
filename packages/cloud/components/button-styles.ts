import { cx } from '@/lib/cx';

export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'danger-fill';
export type ButtonSize = 'sm' | 'md' | 'lg';

const VARIANTS: Record<ButtonVariant, string> = {
  primary:
    'border-primary bg-primary font-medium text-on-primary hover:border-primary-hover hover:bg-primary-hover hover:text-on-primary',
  secondary: 'border-control bg-bg text-fg hover:bg-surface-200 hover:text-fg',
  ghost: 'border-transparent bg-transparent text-fg-muted hover:bg-surface-200 hover:text-fg',
  danger:
    'border-danger-border bg-danger-bg font-medium text-danger hover:border-danger hover:text-danger',
  'danger-fill':
    'border-danger bg-danger font-medium text-on-primary hover:opacity-90 hover:text-on-primary',
};

const SIZES: Record<ButtonSize, string> = {
  sm: 'h-7 px-2.5 text-[12px] rounded-[6px]',
  md: 'h-8 px-3 text-[13px] rounded-[6px]',
  // Phone layouts: a 44px target.
  lg: 'h-11 px-3.5 text-[14px] rounded-[8px]',
};

// Disabled comes last so it wins over every variant's colours.
const DISABLED =
  'disabled:cursor-not-allowed disabled:border-surface-300 disabled:bg-surface-300 disabled:text-fg-disabled disabled:opacity-100 aria-disabled:pointer-events-none aria-disabled:border-surface-300 aria-disabled:bg-surface-300 aria-disabled:text-fg-disabled';

export function buttonClasses(
  variant: ButtonVariant = 'secondary',
  size: ButtonSize = 'md',
  className?: string,
): string {
  return cx(
    'box-border inline-flex cursor-pointer items-center justify-center gap-2 whitespace-nowrap border no-underline transition-colors duration-[120ms]',
    SIZES[size],
    VARIANTS[variant],
    DISABLED,
    className,
  );
}
