import { routes } from '@/lib/routes';
import { buttonClasses } from './button-styles';
import { Card } from './Card';
import { LockIcon } from './icons';
import { Notice } from './Notice';
import { SwitchAccount } from './SwitchAccount';

export type ShareGateProps = {
  /** Where to come back to after signing in or switching account. */
  returnTo: string;
  /** Set on a 403: the API's message, which names the team, and who is signed in. */
  denied?: { message: string; email: string | null };
};

/**
 * A restricted link the visitor can't open yet. The API says nothing about the
 * collection until they can, so this card names nothing but what it was told.
 */
export function ShareGate({ returnTo, denied }: ShareGateProps) {
  return (
    <Card className="flex flex-col gap-4 rounded-[12px] px-7 py-8 max-sm:px-4 max-sm:py-6">
      <span
        aria-hidden="true"
        className="grid size-11 place-items-center rounded-[11px] border border-control bg-raised text-fg-muted"
      >
        <LockIcon size={18} />
      </span>
      {denied ? (
        <>
          <h1 className="m-0 text-[20px] leading-7 font-semibold">You don’t have access yet</h1>
          <Notice tone="err" role="alert">
            {denied.message}
          </Notice>
          {denied.email ? (
            <p className="m-0 text-[12px] text-fg-subtle">Signed in as {denied.email}.</p>
          ) : null}
          <SwitchAccount returnTo={returnTo} />
        </>
      ) : (
        <>
          <div className="flex flex-col gap-1.5">
            <h1 className="m-0 text-[20px] leading-7 font-semibold">
              Sign in to open this collection.
            </h1>
            <p className="m-0 text-fg-muted">
              The team shares it with specific people. Sign in with the email they added, and it
              opens.
            </p>
          </div>
          <a
            href={`${routes.signIn}?return_to=${encodeURIComponent(returnTo)}`}
            className={buttonClasses('primary', 'lg', 'h-10! w-full text-[14px]! max-sm:h-11!')}
          >
            Sign in to open
          </a>
        </>
      )}
    </Card>
  );
}
