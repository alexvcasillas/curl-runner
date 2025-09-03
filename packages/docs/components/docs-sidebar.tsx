'use client';

import { ChevronRight } from 'lucide-react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { ScrollArea } from '@/components/ui/scroll-area';
import { docsConfig, type NavItem } from '@/lib/docs-config';
import { cn } from '@/lib/utils';

export function DocsSidebar({ className }: { className?: string }) {
  const pathname = usePathname();

  return (
    <div className={cn('pb-12', className)}>
      <div className="space-y-4 py-4">
        <div className="px-3 py-2">
          <h2 className="mb-2 px-4 text-lg font-semibold tracking-tight">Documentation</h2>
          <ScrollArea className="h-[calc(100vh-8rem)] px-1">
            <div className="space-y-1">
              {docsConfig.sidebarNav.map((section) => (
                <SidebarSection key={section.title} section={section} pathname={pathname} />
              ))}
            </div>
          </ScrollArea>
        </div>
      </div>
    </div>
  );
}

function SidebarSection({ section, pathname }: { section: NavItem; pathname: string }) {
  const [isOpen, setIsOpen] = useState(() => {
    // Open section if any of its items are active
    return (
      section.items?.some(
        (item) => pathname === item.href || pathname.startsWith(`${item.href}/`),
      ) ?? false
    );
  });

  if (!section.items) {
    if (!section.href) {
      return null;
    }
    return (
      <Link href={section.href}>
        <span
          className={cn(
            'group flex w-full items-center rounded-md border border-transparent px-2 py-1 hover:bg-accent hover:text-accent-foreground',
            pathname === section.href && 'bg-accent',
          )}
        >
          {section.title}
        </span>
      </Link>
    );
  }

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <CollapsibleTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className="w-full justify-between px-2 py-1 font-normal hover:bg-accent hover:text-accent-foreground"
        >
          <span>{section.title}</span>
          <ChevronRight className={cn('h-4 w-4 transition-transform', isOpen && 'rotate-90')} />
        </Button>
      </CollapsibleTrigger>
      <CollapsibleContent className="space-y-1">
        <div className="ml-4 border-l border-border pl-4 space-y-1">
          {section.items.map((item) =>
            item.href ? (
              <Link key={item.href} href={item.href}>
                <span
                  className={cn(
                    'group flex w-full items-center rounded-md border border-transparent px-2 py-1 text-sm hover:bg-accent hover:text-accent-foreground',
                    pathname === item.href && 'bg-accent text-accent-foreground',
                  )}
                >
                  {item.title}
                </span>
              </Link>
            ) : null,
          )}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}
