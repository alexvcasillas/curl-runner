'use client';

import { Link as LinkIcon } from 'lucide-react';
import Link from 'next/link';
import type React from 'react';
import { cn } from '@/lib/utils';

interface DocsHeadingProps {
  id: string;
  level?: 1 | 2 | 3 | 4 | 5 | 6;
  children: React.ReactNode;
  className?: string;
}

export function DocsHeading({ id, level = 2, children, className }: DocsHeadingProps) {
  const headingClasses = cn(
    'group scroll-mt-20 relative flex items-center',
    {
      'text-4xl font-bold tracking-tight': level === 1,
      'text-2xl font-semibold tracking-tight mb-4': level === 2,
      'text-lg font-medium': level === 3,
      'text-base font-medium': level === 4,
      'text-sm font-medium': level === 5,
      'text-xs font-medium': level === 6,
    },
    className,
  );

  const Heading = `h${level}` as keyof React.JSX.IntrinsicElements;

  return (
    <Heading id={id} className={headingClasses}>
      <Link
        href={`#${id}`}
        className="absolute -left-8 flex items-center justify-center w-8 h-full opacity-0 group-hover:opacity-100 transition-opacity"
        aria-label={`Link to section`}
      >
        <LinkIcon className="h-4 w-4 text-muted-foreground hover:text-foreground" />
      </Link>
      {children}
    </Heading>
  );
}

// Export convenience components for each heading level
export const H1 = (props: Omit<DocsHeadingProps, 'level'>) => <DocsHeading level={1} {...props} />;
export const H2 = (props: Omit<DocsHeadingProps, 'level'>) => <DocsHeading level={2} {...props} />;
export const H3 = (props: Omit<DocsHeadingProps, 'level'>) => <DocsHeading level={3} {...props} />;
export const H4 = (props: Omit<DocsHeadingProps, 'level'>) => <DocsHeading level={4} {...props} />;
export const H5 = (props: Omit<DocsHeadingProps, 'level'>) => <DocsHeading level={5} {...props} />;
export const H6 = (props: Omit<DocsHeadingProps, 'level'>) => <DocsHeading level={6} {...props} />;
