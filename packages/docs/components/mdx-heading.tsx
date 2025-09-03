'use client';

import { Link as LinkIcon } from 'lucide-react';
import Link from 'next/link';
import React from 'react';
import { cn } from '@/lib/utils';

interface HeadingProps extends React.HTMLAttributes<HTMLHeadingElement> {
  level: 1 | 2 | 3 | 4 | 5 | 6;
  children: React.ReactNode;
}

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^\w\s-]/g, '')
    .replace(/[\s_-]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

export function MdxHeading({ level, children, className, ...props }: HeadingProps) {
  const childText = typeof children === 'string' ? children : extractText(children);
  const slug = slugify(childText);

  const headingClasses = cn(
    'group scroll-mt-20 relative',
    {
      'text-4xl font-bold tracking-tight': level === 1,
      'text-3xl font-semibold tracking-tight mt-10 mb-4 pb-2 border-b': level === 2,
      'text-2xl font-semibold tracking-tight mt-8 mb-3': level === 3,
      'text-xl font-semibold tracking-tight mt-6 mb-2': level === 4,
      'text-lg font-semibold tracking-tight mt-4 mb-2': level === 5,
      'text-base font-semibold tracking-tight mt-4 mb-1': level === 6,
    },
    className,
  );

  // Dynamically create the appropriate heading element
  const Element = React.createElement(
    `h${level}`,
    {
      id: slug,
      className: headingClasses,
      ...props,
    },
    children,
    React.createElement(
      Link,
      {
        href: `#${slug}`,
        className:
          'absolute -left-6 top-0 flex items-center justify-center w-5 h-full opacity-0 group-hover:opacity-100 transition-opacity',
        'aria-label': `Link to ${childText}`,
      },
      React.createElement(LinkIcon, { className: 'h-4 w-4 text-muted-foreground' }),
    ),
  );

  return Element;
}

function extractText(children: React.ReactNode): string {
  if (typeof children === 'string') return children;
  if (typeof children === 'number') return String(children);
  if (Array.isArray(children)) {
    return children.map(extractText).join('');
  }
  if (children && typeof children === 'object' && 'props' in children) {
    return extractText((children as any).props.children);
  }
  return '';
}

// Export specific heading components
export const H1 = (props: Omit<HeadingProps, 'level'>) => <MdxHeading level={1} {...props} />;
export const H2 = (props: Omit<HeadingProps, 'level'>) => <MdxHeading level={2} {...props} />;
export const H3 = (props: Omit<HeadingProps, 'level'>) => <MdxHeading level={3} {...props} />;
export const H4 = (props: Omit<HeadingProps, 'level'>) => <MdxHeading level={4} {...props} />;
export const H5 = (props: Omit<HeadingProps, 'level'>) => <MdxHeading level={5} {...props} />;
export const H6 = (props: Omit<HeadingProps, 'level'>) => <MdxHeading level={6} {...props} />;
