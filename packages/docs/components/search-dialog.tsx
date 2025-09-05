'use client';

import { Command as CommandPrimitive } from 'cmdk';
import { FileText, Hash, Search } from 'lucide-react';
import { useEffect, useState } from 'react';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { docsConfig } from '@/lib/docs-config';

interface SearchResult {
  title: string;
  href: string;
  description?: string;
  type: 'page' | 'section';
}

interface SearchDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SearchDialog({ open, onOpenChange }: SearchDialogProps) {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);

  // Flatten all navigation items for search
  const searchableItems: SearchResult[] = [
    ...docsConfig.sidebarNav.flatMap(
      (section) =>
        section.items
          ?.filter((item) => item.href)
          .map((item) => ({
            title: item.title,
            href: item.href!,
            description: item.description,
            type: 'page' as const,
          })) || [],
    ),
  ];

  useEffect(() => {
    if (!query) {
      setResults([]);
      return;
    }

    const filtered = searchableItems.filter(
      (item) =>
        item.title.toLowerCase().includes(query.toLowerCase()) ||
        item.description?.toLowerCase().includes(query.toLowerCase()),
    );

    setResults(filtered.slice(0, 10));
  }, [query, searchableItems.filter]);

  const handleSelect = (href: string) => {
    onOpenChange(false);
    window.location.href = href;
  };

  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.key === 'k' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        onOpenChange(!open);
      }
    };
    document.addEventListener('keydown', down);
    return () => document.removeEventListener('keydown', down);
  }, [open, onOpenChange]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle className="sr-only">Search Documentation</DialogTitle>
        </DialogHeader>
        <CommandPrimitive className="overflow-hidden rounded-md border bg-popover text-popover-foreground shadow-md">
          <div className="flex items-center border-b border-border px-4">
            <Search className="mr-2 h-4 w-4 shrink-0 opacity-50" />
            <CommandPrimitive.Input
              value={query}
              onValueChange={setQuery}
              placeholder="Search documentation..."
              className="flex h-12 w-full rounded-md bg-transparent py-3 text-sm outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-50"
            />
          </div>

          <CommandPrimitive.List className="max-h-[400px] overflow-y-auto p-2">
            {!query && (
              <CommandPrimitive.Empty className="py-6 text-center text-sm text-muted-foreground">
                Type to search documentation...
              </CommandPrimitive.Empty>
            )}

            {query && results.length === 0 && (
              <CommandPrimitive.Empty className="py-6 text-center text-sm text-muted-foreground">
                No results found for "{query}"
              </CommandPrimitive.Empty>
            )}

            {results.map((result) => (
              <CommandPrimitive.Item
                key={result.href}
                onSelect={() => handleSelect(result.href)}
                className="flex cursor-pointer items-start space-x-3 rounded-md px-3 py-2 text-sm hover:bg-accent hover:text-accent-foreground"
              >
                <div className="mt-0.5">
                  {result.type === 'page' ? (
                    <FileText className="h-4 w-4" />
                  ) : (
                    <Hash className="h-4 w-4" />
                  )}
                </div>
                <div className="flex-1 space-y-1">
                  <p className="font-medium">{result.title}</p>
                  {result.description && (
                    <p className="text-xs text-muted-foreground">{result.description}</p>
                  )}
                </div>
              </CommandPrimitive.Item>
            ))}
          </CommandPrimitive.List>
        </CommandPrimitive>
      </DialogContent>
    </Dialog>
  );
}
