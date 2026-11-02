import { Terminal } from 'lucide-react';
import Link from 'next/link';
import { CopyButton } from '@/components/site/CopyButton';
import {
  STUDIO_CASK_COMMAND,
  STUDIO_CHANGELOG_HREF,
  STUDIO_DOWNLOAD_HREF,
  STUDIO_REQUIREMENTS,
  STUDIO_VERSION,
} from '@/lib/studio';

/** How to get the studio: the dmg on the downloads page, or the Homebrew cask. */
export function StudioPreview({ className = '' }: { className?: string }) {
  return (
    <aside
      aria-labelledby="studio-preview-heading"
      className={`overflow-hidden rounded-[14px] border border-line bg-surface ${className}`}
    >
      <div className="px-6 pt-5 pb-5">
        <h2
          id="studio-preview-heading"
          className="m-0 flex items-center gap-2 text-[15px] font-semibold text-fg"
        >
          <span aria-hidden="true" className="size-1.5 rounded-full bg-brand" />
          Download the studio
        </h2>
        <p className="mt-2 mb-0 text-[15px] leading-[1.6] text-fg-secondary">
          {STUDIO_REQUIREMENTS}. Signed and notarized. Sync and sharing are coming soon. Get the dmg
          from the{' '}
          <Link
            href={STUDIO_DOWNLOAD_HREF}
            className="text-fg underline underline-offset-4 focus-visible:outline-2 focus-visible:outline-brand"
          >
            downloads page
          </Link>
          , or read{' '}
          <Link
            href={STUDIO_CHANGELOG_HREF}
            className="text-fg underline underline-offset-4 focus-visible:outline-2 focus-visible:outline-brand"
          >
            {`what shipped in ${STUDIO_VERSION}`}
          </Link>
          .
        </p>
      </div>
      <div className="border-t border-hairline bg-ground">
        <div className="flex h-11 items-center gap-2 border-b border-hairline pr-0 pl-4 font-mono text-[13px] text-fg-secondary">
          <Terminal aria-hidden="true" className="size-3.5 shrink-0" />
          <span className="grow truncate">terminal</span>
          <CopyButton text={STUDIO_CASK_COMMAND} label="Copy the install command" />
        </div>
        <pre className="m-0 overflow-x-auto px-5 py-4 font-mono text-[13.5px] leading-[1.75] text-fg focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-brand">
          <code className="font-mono">
            <span className="block">
              <span aria-hidden="true" className="text-fg-tertiary select-none">
                ${' '}
              </span>
              {STUDIO_CASK_COMMAND}
            </span>
          </code>
        </pre>
      </div>
    </aside>
  );
}
