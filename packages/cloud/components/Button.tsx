import type { ButtonHTMLAttributes } from 'react';
import { type ButtonSize, type ButtonVariant, buttonClasses } from './button-styles';
import { Spinner } from './Spinner';

export type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
  size?: ButtonSize;
  /** Shows a spinner and disables the button. Pass the verb in progress as children: "Creating team…". */
  loading?: boolean;
  ref?: React.Ref<HTMLButtonElement>;
};

/** One primary per view. Red fill only inside a confirm dialog. */
export function Button({
  variant = 'secondary',
  size = 'md',
  loading = false,
  disabled,
  className,
  type = 'button',
  children,
  ...rest
}: ButtonProps) {
  return (
    <button
      type={type}
      disabled={disabled || loading}
      aria-busy={loading || undefined}
      className={buttonClasses(variant, size, className)}
      {...rest}
    >
      {loading ? <Spinner /> : null}
      {children}
    </button>
  );
}
