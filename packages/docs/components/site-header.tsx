'use client';

import { Menu, Terminal } from 'lucide-react';
import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useState } from 'react';
import { DocsSidebar } from '@/components/docs-sidebar';
import { SearchDialog } from '@/components/search-dialog';
import { ThemeToggle } from '@/components/theme-toggle';
import { Button } from '@/components/ui/button';
import { Sheet, SheetContent, SheetTrigger } from '@/components/ui/sheet';
import { docsConfig } from '@/lib/docs-config';
import { cn } from '@/lib/utils';
import { GitHubStarsButton } from './animate-ui/buttons/github-stars';

export function SiteHeader() {
  const [searchOpen, setSearchOpen] = useState(false);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const pathname = usePathname();

  return (
    <>
      <header className="sticky top-0 z-50 w-full border-b border-border bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="w-full flex h-14 items-center px-4 sm:px-6 lg:px-8">
          {/* Mobile menu */}
          <Sheet open={mobileMenuOpen} onOpenChange={setMobileMenuOpen}>
            <SheetTrigger asChild>
              <Button
                variant="ghost"
                className="mr-2 px-0 text-base hover:bg-transparent focus-visible:bg-transparent focus-visible:ring-0 focus-visible:ring-offset-0 md:hidden"
              >
                <Menu className="h-5 w-5" />
                <span className="sr-only">Toggle Menu</span>
              </Button>
            </SheetTrigger>
            <SheetContent side="left" className="pr-0">
              <DocsSidebar />
            </SheetContent>
          </Sheet>

          {/* Logo */}
          <Link href="/" className="mr-6 flex items-center space-x-2">
            <Terminal className="h-6 w-6" />
            <span className="hidden font-mono font-bold sm:inline-block">curl-runner</span>
          </Link>

          {/* Desktop navigation */}
          <nav className="hidden md:flex items-center gap-6 text-sm">
            {docsConfig.mainNav
              .filter((item) => item.href)
              .map((item) => {
                const isActive =
                  item.href === '/docs' ? pathname?.startsWith('/docs') : pathname === item.href;

                return (
                  <Link
                    key={item.href}
                    href={item.href!}
                    className={cn(
                      'transition-colors hover:text-foreground/80',
                      isActive ? 'text-foreground font-medium' : 'text-foreground/60',
                    )}
                  >
                    {item.title}
                  </Link>
                );
              })}
          </nav>

          <div className="flex flex-1 items-center justify-end space-x-2">
            <nav className="flex items-center gap-1">
              <GitHubStarsButton username="alexvcasillas" repo="curl-runner" />
              <ThemeToggle />
            </nav>
          </div>
        </div>
      </header>

      <SearchDialog open={searchOpen} onOpenChange={setSearchOpen} />
    </>
  );
}
