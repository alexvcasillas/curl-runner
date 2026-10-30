import { Terminal } from 'lucide-react';
import Link from 'next/link';
import { CopyButton } from '@/components/site/CopyButton';
import { STUDIO_BUILD_COMMAND, STUDIO_CHANGELOG_HREF, STUDIO_REPO_URL } from '@/lib/studio';

export const STUDIO_BUILD_STEPS = [
  `git clone ${STUDIO_REPO_URL}`,
  'cd curl-runner',
  STUDIO_BUILD_COMMAND,
] as const;

/** How to try the studio: it is a preview with no download, built from source. */
export function StudioPreview({ className = '' }: { className?: string }) {
  const script = STUDIO_BUILD_STEPS.join('\n');
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
          <span aria-hidden="true" className="size-1.5 rounded-full bg-brand" />A preview, built
          from source
        </h2>
        <p className="mt-2 mb-0 text-[15px] leading-[1.6] text-fg-secondary">
          The studio runs on macOS and isn’t distributed yet: there is no signed build, Homebrew
          formula or download. To try it, build it from the repository with Rust (the repository
          pins the toolchain). Read{' '}
          <Link
            href={STUDIO_CHANGELOG_HREF}
            className="text-fg underline underline-offset-4 focus-visible:outline-2 focus-visible:outline-brand"
          >
            what shipped in 2.2.0
          </Link>
          .
        </p>
      </div>
      <div className="border-t border-hairline bg-ground">
        <div className="flex h-11 items-center gap-2 border-b border-hairline pr-0 pl-4 font-mono text-[13px] text-fg-secondary">
          <Terminal aria-hidden="true" className="size-3.5 shrink-0" />
          <span className="grow truncate">terminal</span>
          <CopyButton text={script} label="Copy the build commands" />
        </div>
        <pre className="m-0 overflow-x-auto px-5 py-4 font-mono text-[13.5px] leading-[1.75] text-fg focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-brand">
          <code className="font-mono">
            {STUDIO_BUILD_STEPS.map((step) => (
              <span key={step} className="block">
                <span aria-hidden="true" className="text-fg-tertiary select-none">
                  ${' '}
                </span>
                {step}
              </span>
            ))}
          </code>
        </pre>
      </div>
    </aside>
  );
}
