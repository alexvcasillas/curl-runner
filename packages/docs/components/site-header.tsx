'use client';

import { Github, Menu, Search, Terminal } from 'lucide-react';
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
        <div className="container flex h-14 max-w-screen-2xl items-center px-4 sm:px-6 lg:px-8">
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

          <div className="flex flex-1 items-center justify-between space-x-2 md:justify-end">
            <div className="w-full flex-1 md:w-auto md:flex-none">
              <Button
                variant="outline"
                className="relative h-8 w-full justify-start rounded-md bg-background text-sm font-normal text-muted-foreground shadow-none sm:pr-12 md:w-40 lg:w-64"
                onClick={() => setSearchOpen(true)}
              >
                <Search className="mr-2 h-4 w-4" />
                <span className="hidden lg:inline-flex">Search documentation...</span>
                <span className="inline-flex lg:hidden">Search...</span>
                <kbd className="pointer-events-none absolute right-1.5 top-1.5 hidden h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium opacity-100 sm:flex">
                  <span className="text-xs">⌘</span>K
                </kbd>
              </Button>
            </div>
            <nav className="flex items-center gap-1">
              {/* <Button variant="ghost" size="icon" asChild className="h-8 w-8">
                <a
                  href="https://github.com/alexvcasillas/curl-runner"
                  target="_blank"
                  rel="noreferrer"
                >
                  <Github className="h-4 w-4" />
                  <span className="sr-only">GitHub</span>
                </a>
              </Button> */}
              <GitHubStarsButton username="alexvcasillas" repo="react-native-loading-dots" />
              <ThemeToggle />
            </nav>
          </div>
        </div>
      </header>

      <SearchDialog open={searchOpen} onOpenChange={setSearchOpen} />
    </>
  );
}
