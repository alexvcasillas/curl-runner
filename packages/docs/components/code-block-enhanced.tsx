'use client';

import { useTheme } from 'next-themes';
import { useEffect, useState } from 'react';
import { codeToHtml } from 'shiki';
import { CopyButton } from '@/components/copy-button';

interface CodeBlockEnhancedProps {
  children: string;
  language?: string;
  filename?: string;
  className?: string;
  title?: string;
}

export function CodeBlockEnhanced({
  children,
  language = 'text',
  filename,
  title,
  className = '',
}: CodeBlockEnhancedProps) {
  const [highlightedCode, setHighlightedCode] = useState<string>('');
  const [isLoading, setIsLoading] = useState(true);
  const { resolvedTheme } = useTheme();

  useEffect(() => {
    async function highlightCode() {
      const isDark = resolvedTheme === 'dark';
      const selectedTheme = isDark ? 'dracula' : 'github-light';

      try {
        const highlighted = await codeToHtml(children.trim(), {
          lang: language,
          theme: selectedTheme,
        });
        setHighlightedCode(highlighted);
      } catch {
        // Fallback for unsupported languages
        setHighlightedCode(`<pre><code>${children.trim()}</code></pre>`);
      }
      setIsLoading(false);
    }

    highlightCode();
  }, [children, language, resolvedTheme]);

  if (isLoading) {
    return (
      <div className={`relative overflow-hidden rounded-lg border bg-muted/20 ${className}`}>
        {(filename || title) && (
          <div className="flex items-center justify-between border-b bg-muted/40 px-4 py-2">
            <div className="text-sm font-medium text-muted-foreground">{filename || title}</div>
            <CopyButton value={children} />
          </div>
        )}
        <div className="relative">
          <div className="animate-pulse">
            <div className="h-16 bg-muted/20 rounded"></div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      className={`group relative overflow-hidden rounded-lg border-2 border-primary/10 bg-gradient-to-br from-background to-muted/30 ${className}`}
    >
      {(filename || title) && (
        <div className="flex items-center justify-between border-b border-primary/10 bg-primary/5 px-4 py-3">
          <div className="flex items-center space-x-2">
            <div className="flex space-x-1">
              <div className="h-2 w-2 rounded-full bg-red-400"></div>
              <div className="h-2 w-2 rounded-full bg-yellow-400"></div>
              <div className="h-2 w-2 rounded-full bg-green-400"></div>
            </div>
            <div className="text-sm font-medium text-primary">{filename || title}</div>
          </div>
          <CopyButton value={children} />
        </div>
      )}

      <div className="relative">
        {!(filename || title) && (
          <div className="absolute right-2 top-2 z-10 opacity-0 group-hover:opacity-100 transition-opacity">
            <CopyButton value={children} />
          </div>
        )}

        <div
          className="overflow-x-auto p-4 text-sm [&>pre]:!bg-transparent [&>pre]:!m-0 [&>pre]:!p-0"
          // biome-ignore lint/security/noDangerouslySetInnerHtml: we need to set the html
          dangerouslySetInnerHTML={{ __html: highlightedCode }}
        />
      </div>
    </div>
  );
}
