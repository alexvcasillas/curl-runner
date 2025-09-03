'use client';

import { useTheme } from 'next-themes';
import { useEffect, useState } from 'react';
import { codeToHtml } from 'shiki';
import { CopyButton } from '@/components/copy-button';

interface CodeBlockProps {
  children: string;
  language?: string;
  filename?: string;
  className?: string;
}

export function CodeBlock({
  children,
  language = 'text',
  filename,
  className = '',
}: CodeBlockProps) {
  const [highlightedCode, setHighlightedCode] = useState<string>('');
  const [isLoading, setIsLoading] = useState(true);
  const { resolvedTheme } = useTheme();

  useEffect(() => {
    const highlightCode = async () => {
      try {
        const isDark = resolvedTheme === 'dark';
        const selectedTheme = isDark ? 'dracula' : 'github-light';

        const highlighted = await codeToHtml(children.trim(), {
          lang: language,
          theme: selectedTheme,
        });
        setHighlightedCode(highlighted);
      } catch {
        // Fallback to plain text if highlighting fails
        setHighlightedCode(`<pre><code>${children}</code></pre>`);
      } finally {
        setIsLoading(false);
      }
    };

    highlightCode();
  }, [children, language, resolvedTheme]);

  if (isLoading) {
    return (
      <div className={`relative overflow-hidden rounded-lg border bg-muted/20 ${className}`}>
        {filename && (
          <div className="flex items-center justify-between border-b bg-muted/40 px-4 py-2">
            <div className="text-sm font-medium text-muted-foreground">{filename}</div>
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
    <div className={`relative overflow-hidden rounded-lg border bg-muted/20 ${className}`}>
      {filename && (
        <div className="flex items-center justify-between border-b bg-muted/40 px-4 py-2">
          <div className="text-sm font-medium text-muted-foreground">{filename}</div>
          <CopyButton value={children} />
        </div>
      )}

      <div className="relative">
        {!filename && (
          <div className="absolute right-2 top-2 z-10">
            <CopyButton value={children} />
          </div>
        )}

        <div
          className={`overflow-x-auto text-sm ${filename ? '' : 'pr-12'} [&>pre]:!bg-transparent [&>pre]:!p-4`}
          // biome-ignore lint/security/noDangerouslySetInnerHtml: we need to set the html
          dangerouslySetInnerHTML={{ __html: highlightedCode }}
        />
      </div>
    </div>
  );
}
