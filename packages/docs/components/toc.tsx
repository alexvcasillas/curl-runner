'use client';

import { useEffect, useState } from 'react';
import { cn } from '@/lib/utils';

interface TocItem {
  id: string;
  title: string;
  level: number;
}

interface TocProps {
  className?: string;
}

export function TableOfContents({ className }: TocProps) {
  const [toc, setToc] = useState<TocItem[]>([]);
  const [activeId, setActiveId] = useState<string>('');

  useEffect(() => {
    const headings = document.querySelectorAll('h2, h3, h4, h5, h6');
    const tocItems: TocItem[] = [];

    headings.forEach((heading) => {
      const id = heading.id;
      const title = heading.textContent || '';
      const level = parseInt(heading.tagName.charAt(1), 10);

      if (id && title) {
        tocItems.push({ id, title, level });
      }
    });

    setToc(tocItems);
  }, []);

  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            setActiveId(entry.target.id);
          }
        });
      },
      {
        rootMargin: '0% 0% -80% 0%',
        threshold: 0.1,
      },
    );

    const headings = document.querySelectorAll('h2, h3, h4, h5, h6');
    headings.forEach((heading) => observer.observe(heading));

    return () => {
      headings.forEach((heading) => observer.unobserve(heading));
    };
  }, []);

  if (toc.length === 0) {
    return null;
  }

  return (
    <div className={cn('space-y-2', className)}>
      <p className="font-medium">On This Page</p>
      <ul className="m-0 list-none space-y-2">
        {toc.map((item) => (
          <li key={item.id} className="mt-0">
            <a
              href={`#${item.id}`}
              className={cn(
                'inline-block no-underline transition-colors hover:text-foreground text-sm',
                {
                  'pl-0': item.level === 2,
                  'pl-4': item.level === 3,
                  'pl-6': item.level === 4,
                  'pl-8': item.level === 5,
                  'pl-10': item.level === 6,
                },
                activeId === item.id ? 'text-foreground font-medium' : 'text-muted-foreground',
              )}
            >
              {item.title}
            </a>
          </li>
        ))}
      </ul>
    </div>
  );
}
