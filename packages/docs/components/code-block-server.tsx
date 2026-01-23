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
    <div className={`relative overflow-hidden rounded-lg border bg-muted/20 max-w-full ${className}`}>
      {/* Terminal Header */}
      {displayTitle && (
        <div className="flex items-center justify-between border-b bg-gradient-to-r from-slate-900 to-slate-800 px-3 py-2 sm:px-4 sm:py-3">
          <div className="flex items-center space-x-2 sm:space-x-3 min-w-0">
            <div className="flex space-x-1 flex-shrink-0">
              <div className="w-2.5 h-2.5 sm:w-3 sm:h-3 bg-red-500 rounded-full"></div>
              <div className="w-2.5 h-2.5 sm:w-3 sm:h-3 bg-yellow-500 rounded-full"></div>
              <div className="w-2.5 h-2.5 sm:w-3 sm:h-3 bg-green-500 rounded-full"></div>
            </div>
            <div className="text-xs sm:text-sm font-medium text-slate-200 truncate">{displayTitle}</div>
          </div>
          <CopyButton className="text-white flex-shrink-0" value={children} />
        </div>
      )}

      <div className="relative">
        {!displayTitle && (
          <div className="absolute right-2 top-2 z-10">
            <CopyButton className="text-white flex-shrink-0" value={children} />
          </div>
        )}

        {/* Light theme - hidden in dark mode */}
        <div
          className={`overflow-x-auto text-xs sm:text-sm dark:hidden ${displayTitle ? '' : 'pr-12'} [&>pre]:!bg-transparent [&>pre]:!p-3 [&>pre]:sm:!p-4`}
          // biome-ignore lint/security/noDangerouslySetInnerHtml: we need to set the html
          dangerouslySetInnerHTML={{ __html: lightHtml }}
        />

        {/* Dark theme - hidden in light mode */}
        <div
          className={`overflow-x-auto text-xs sm:text-sm hidden dark:block ${displayTitle ? '' : 'pr-12'} [&>pre]:!bg-transparent [&>pre]:!p-3 [&>pre]:sm:!p-4`}
          // biome-ignore lint/security/noDangerouslySetInnerHtml: we need to set the html
          dangerouslySetInnerHTML={{ __html: darkHtml }}
        />
      </div>
    </div>
  );
}
