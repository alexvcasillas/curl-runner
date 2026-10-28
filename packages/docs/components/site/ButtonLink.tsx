import Link from 'next/link';
import type { ReactNode } from 'react';

interface ButtonLinkProps {
  href: string;
  variant: 'primary' | 'secondary';
  size: 'md' | 'lg';
  children: ReactNode;
}

const VARIANTS = {
  primary: 'bg-fg text-ground hover:bg-white',
  secondary: 'border border-line text-fg hover:bg-hover',
} as const;

const SIZES = {
  // 36px tall; the ::before extends the touch target to 44px.
  md: "h-9 rounded-lg px-3.5 text-sm before:absolute before:inset-x-0 before:-inset-y-1 before:content-['']",
  lg: 'h-12 rounded-[10px] px-[22px] text-[15px]',
} as const;

export function ButtonLink({ href, variant, size, children }: ButtonLinkProps) {
  const className = `relative inline-flex shrink-0 items-center justify-center gap-2 font-medium whitespace-nowrap transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand ${VARIANTS[variant]} ${SIZES[size]}`;

  if (/^https?:\/\//.test(href)) {
    return (
      <a href={href} className={className}>
        {children}
      </a>
    );
  }
  return (
    <Link href={href} className={className}>
      {children}
    </Link>
  );
}
