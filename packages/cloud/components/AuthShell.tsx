import Link from 'next/link';
import { external, routes } from '@/lib/routes';
import { Avatar } from './Avatar';

export type AuthShellProps = {
  /** The signed-in email, shown at the right of the header on the hand-off pages. */
  signedInAs?: string;
  /** `md` is the 440px column the link and hand-off pages use. */
  width?: 'sm' | 'md';
  /** Footer links under the page. Sign-in only. */
  footer?: boolean;
  children: React.ReactNode;
};

/** The signed-out frame: the mark, Docs, one centred column, an optional footer. */
export function AuthShell({ signedInAs, width = 'sm', footer = false, children }: AuthShellProps) {
  return (
    <div className="flex min-h-dvh flex-col bg-bg">
      <header className="flex min-h-14 flex-wrap items-center gap-2.5 border-b border-hairline px-6 max-sm:px-4">
        <Link
          href={routes.home}
          aria-label="curl-runner Cloud home"
          className="flex items-center gap-2.5 text-[15px] font-semibold text-fg hover:text-fg"
        >
          <span
            aria-hidden="true"
            className="grid size-6 place-items-center rounded-[6px] bg-primary font-mono text-[11px] font-medium text-on-primary"
          >
            &gt;_
          </span>
          <span>
            curl-runner <span className="font-normal text-fg-muted">Cloud</span>
          </span>
        </Link>
        <span className="flex-1" />
        {signedInAs ? (
          <span className="flex min-w-0 items-center gap-2 text-fg-muted">
            <Avatar name={signedInAs} size={22} />
            <span className="truncate max-sm:hidden">{signedInAs}</span>
          </span>
        ) : (
          <a href={external.docs} className="text-fg-muted hover:text-fg">
            Docs
          </a>
        )}
      </header>
      <main className="flex flex-1 justify-center px-4 pt-24 pb-12 max-sm:pt-12">
        <div
          className={`flex w-full flex-col gap-6 ${width === 'md' ? 'max-w-[440px]' : 'max-w-[380px]'}`}
        >
          {children}
        </div>
      </main>
      {footer ? (
        <footer className="flex flex-wrap justify-center gap-5 border-t border-hairline px-4 py-6 text-[12px] text-fg-subtle">
          <span>curl-runner is open source</span>
          <a href={external.docs} className="text-fg-subtle hover:text-fg">
            Self-host it
          </a>
        </footer>
      ) : null}
    </div>
  );
}
