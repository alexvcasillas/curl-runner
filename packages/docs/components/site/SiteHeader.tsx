import Link from 'next/link';
import { ButtonLink } from '@/components/site/ButtonLink';

export type SiteSection = 'features' | 'alternatives' | 'downloads' | 'changelog';

const NAV: { label: string; href: string; section?: SiteSection }[] = [
  { label: 'Docs', href: '/docs' },
  { label: 'Features', href: '/features/parallel-execution', section: 'features' },
  { label: 'Alternatives', href: '/alternatives', section: 'alternatives' },
  { label: 'Downloads', href: '/downloads', section: 'downloads' },
  { label: 'Changelog', href: '/changelog', section: 'changelog' },
];

export const GITHUB_URL = 'https://github.com/alexvcasillas/curl-runner';

export function TerminalMark({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className={className}
    >
      <polyline points="4 17 10 11 4 5" />
      <line x1="12" y1="19" x2="20" y2="19" />
    </svg>
  );
}

export function SiteHeader({ active }: { active?: SiteSection }) {
  return (
    <header className="border-b border-hairline">
      <nav
        aria-label="Main"
        className="mx-auto flex max-w-[1200px] flex-wrap items-center gap-x-8 gap-y-2 px-6 py-4"
      >
        <Link
          href="/"
          className="flex h-11 items-center gap-2.5 text-[17px] font-semibold tracking-[-0.01em] text-fg hover:text-white"
        >
          <TerminalMark className="size-[22px] text-brand" />
          curl-runner
        </Link>
        <div className="flex grow flex-wrap gap-x-6 text-sm">
          {NAV.map((item) => {
            const isActive = item.section !== undefined && item.section === active;
            return (
              <Link
                key={item.href}
                href={item.href}
                aria-current={isActive ? 'page' : undefined}
                className={`inline-flex h-11 items-center transition-colors hover:text-fg ${isActive ? 'text-fg' : 'text-fg-secondary'}`}
              >
                {item.label}
              </Link>
            );
          })}
        </div>
        <div className="flex items-center gap-3">
          <a
            href={GITHUB_URL}
            className="flex size-11 items-center justify-center text-fg-secondary transition-colors hover:text-fg"
          >
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.8"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden="true"
            >
              <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22" />
            </svg>
            <span className="sr-only">GitHub</span>
          </a>
          <ButtonLink href="/docs/quick-start" variant="primary" size="md">
            Get started
          </ButtonLink>
        </div>
      </nav>
    </header>
  );
}
