import { Avatar } from './Avatar';

export type PublicShellProps = {
  /** The signed-in email, at the right of the header. */
  signedInAs?: string | null;
  /** `Cloud` follows the mark on the invite page. */
  cloud?: boolean;
  /** Column width: the landing card is 480px, the invite card 420px. */
  width?: 'landing' | 'invite';
  /** The line under the card. */
  footer?: React.ReactNode;
  /** The admins' announcement, across the very top of the page. */
  announcement?: React.ReactNode;
  children: React.ReactNode;
};

/** The frame of the two public pages: the mark, one centred card, a quiet footer. */
export function PublicShell({
  signedInAs,
  cloud = false,
  width = 'landing',
  footer,
  announcement,
  children,
}: PublicShellProps) {
  return (
    <div className="flex min-h-dvh flex-col bg-bg">
      {announcement}
      <header className="flex min-h-14 items-center gap-2.5 border-b border-hairline px-6 max-sm:px-4">
        <span className="flex items-center gap-2.5 text-[15px] font-semibold text-fg">
          <span
            aria-hidden="true"
            className="grid size-6 place-items-center rounded-[6px] bg-primary font-mono text-[11px] font-medium text-on-primary"
          >
            &gt;_
          </span>
          <span>
            curl-runner{cloud ? <span className="font-normal text-fg-muted"> Cloud</span> : null}
          </span>
        </span>
        <span className="flex-1" />
        {signedInAs ? (
          <span className="flex min-w-0 items-center gap-2 text-[12px] text-fg-muted">
            <Avatar name={signedInAs} size={22} />
            <span className="truncate max-sm:hidden">{signedInAs}</span>
          </span>
        ) : null}
      </header>
      <main
        className={`flex flex-1 justify-center px-4 pb-12 ${width === 'invite' ? 'pt-24' : 'pt-[72px]'} max-sm:pt-8`}
      >
        <div
          className={`flex w-full min-w-0 flex-col gap-4 ${width === 'invite' ? 'max-w-[420px]' : 'max-w-[480px]'}`}
        >
          {children}
        </div>
      </main>
      {footer ? (
        <footer className="flex flex-wrap justify-center gap-x-5 gap-y-2 border-t border-hairline px-4 py-5 text-[12px] text-fg-subtle">
          {footer}
        </footer>
      ) : null}
    </div>
  );
}
