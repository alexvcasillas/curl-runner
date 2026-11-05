import Link from 'next/link';
import type { ComponentProps } from 'react';
import { type ButtonSize, type ButtonVariant, buttonClasses } from './button-styles';

export type ButtonLinkProps = ComponentProps<typeof Link> & {
  variant?: ButtonVariant;
  size?: ButtonSize;
  /** Opens off-site in a new tab, outside the Next router. */
  external?: boolean;
};

/** A link that looks like a button: navigation, not an action. */
export function ButtonLink({
  variant = 'secondary',
  size = 'md',
  external = false,
  className,
  href,
  children,
  ...rest
}: ButtonLinkProps) {
  const classes = buttonClasses(variant, size, className);
  if (external) {
    return (
      <a
        href={String(href)}
        target="_blank"
        rel="noreferrer"
        className={classes}
        {...(rest as ComponentProps<'a'>)}
      >
        {children}
      </a>
    );
  }
  return (
    <Link href={href} className={classes} {...rest}>
      {children}
    </Link>
  );
}
