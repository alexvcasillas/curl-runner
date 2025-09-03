import { codeToHtml } from 'shiki';
import { CopyButton } from '@/components/copy-button';

interface CodeBlockServerProps {
  children: string;
  language?: string;
  filename?: string;
  className?: string;
  title?: string;
}

export async function CodeBlockServer({
  children,
  language = 'text',
  filename,
  title,
  className = '',
}: CodeBlockServerProps) {
  // Generate both light and dark themes at build time
  const [lightHtml, darkHtml] = await Promise.all([
    codeToHtml(children.trim(), {
      lang: language,
      theme: 'github-light',
    }),
    codeToHtml(children.trim(), {
      lang: language,
      theme: 'dracula',
    }),
  ]);

  const displayTitle = filename || title;

  return (
    <div className={`relative overflow-hidden rounded-lg border bg-muted/20 ${className}`}>
      {/* Terminal Header */}
      {displayTitle && (
        <div className="flex items-center justify-between border-b bg-gradient-to-r from-slate-900 to-slate-800 px-4 py-3">
          <div className="flex items-center space-x-3">
            <div className="flex space-x-1">
              <div className="w-3 h-3 bg-red-500 rounded-full"></div>
              <div className="w-3 h-3 bg-yellow-500 rounded-full"></div>
              <div className="w-3 h-3 bg-green-500 rounded-full"></div>
            </div>
            <div className="text-sm font-medium text-slate-200">{displayTitle}</div>
          </div>
          <CopyButton className="text-white" value={children} />
        </div>
      )}

      <div className="relative">
        {!displayTitle && (
          <div className="absolute right-2 top-2 z-10">
            <CopyButton className="text-white" value={children} />
          </div>
        )}

        {/* Light theme - hidden in dark mode */}
        <div
          className={`overflow-x-auto text-sm dark:hidden ${displayTitle ? '' : 'pr-12'} [&>pre]:!bg-transparent [&>pre]:!p-4`}
          // biome-ignore lint/security/noDangerouslySetInnerHtml: we need to set the html
          dangerouslySetInnerHTML={{ __html: lightHtml }}
        />

        {/* Dark theme - hidden in light mode */}
        <div
          className={`overflow-x-auto text-sm hidden dark:block ${displayTitle ? '' : 'pr-12'} [&>pre]:!bg-transparent [&>pre]:!p-4`}
          // biome-ignore lint/security/noDangerouslySetInnerHtml: we need to set the html
          dangerouslySetInnerHTML={{ __html: darkHtml }}
        />
      </div>
    </div>
  );
}
