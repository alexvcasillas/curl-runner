'use client';

import { useState } from 'react';
import type { ActionResult } from '@/lib/action-result';
import { routes } from '@/lib/routes';
import type { Role } from '@/lib/types';
import { Button } from './Button';
import { buttonClasses } from './button-styles';
import { Notice } from './Notice';
import { Pill } from './Pill';
import { SwitchAccount } from './SwitchAccount';

export type InviteCardProps = {
  teamName: string;
  /** Who sent it. Null when that account is gone. */
  inviter: string | null;
  /** What they join as. */
  joinAs: Role;
  /** The invited address. */
  email: string;
  /** "Expires in 5 days". */
  expires: string;
  /** Who is signed in, or null. */
  viewer: { email: string } | null;
  /** The invite page, which sign-in and "Use another account" come back to. */
  returnTo: string;
  /** Accepts the invite. Goes to the team itself on success. */
  onJoin: () => Promise<ActionResult>;
};

const ROLE_NOTE: Record<Role, string> = {
  member: 'Members publish, share collections and make tokens.',
  owner: 'Owners also manage members, billing and settings.',
};

/** The invite: who invited whom to what, and the one way to join. */
export function InviteCard({
  teamName,
  inviter,
  joinAs,
  email,
  expires,
  viewer,
  returnTo,
  onJoin,
}: InviteCardProps) {
  const [pending, setPending] = useState(false);
  const [failure, setFailure] = useState<{ code?: string; message: string } | null>(null);
  const full = failure?.code === 'plan_limit';
  const otherAccount = viewer !== null && viewer.email.toLowerCase() !== email.toLowerCase();

  async function join() {
    setFailure(null);
    setPending(true);
    try {
      const result = await onJoin();
      if (!result.ok) {
        setFailure({ code: result.code, message: result.message });
      }
    } catch {
      // A successful accept redirects, which Next handles itself on the client, so a
      // throw that reaches here is the network.
      setFailure({ message: 'That didn’t work. Try again.' });
    } finally {
      setPending(false);
    }
  }

  const inviterName = inviter ?? 'A team owner';
  return (
    <div className="overflow-hidden rounded-[12px] border border-hairline bg-bg">
      <div className="flex flex-col items-center gap-4 px-7 pt-8 pb-6 text-center max-sm:px-5">
        <span
          aria-hidden="true"
          className="grid size-12 place-items-center rounded-[12px] bg-surface-300 text-[18px] font-semibold"
        >
          {teamName.trim().charAt(0).toUpperCase()}
        </span>
        <div className="flex flex-col gap-1.5">
          <h1 className="m-0 text-[20px] leading-7 font-semibold [overflow-wrap:anywhere]">
            Join {teamName} on curl-runner Cloud
          </h1>
          <p className="m-0 text-fg-muted [overflow-wrap:anywhere]">
            {inviter ? <span className="text-fg">{inviter}</span> : 'A team owner'} invited{' '}
            <span className="text-fg">{email}</span> as {joinAs === 'owner' ? 'an' : 'a'}{' '}
            <span className="text-fg">{joinAs}</span>. {ROLE_NOTE[joinAs]}
          </p>
        </div>
        <Pill>{expires}</Pill>
        {full ? (
          <Notice tone="warn" role="alert" className="text-left">
            {teamName} is on the Free plan, which allows 3 members, and it’s full. Ask {inviterName}{' '}
            to upgrade {teamName} or remove someone, then open this link again.
          </Notice>
        ) : failure ? (
          <Notice tone="err" role="alert" className="text-left">
            {failure.message}
          </Notice>
        ) : null}
        {otherAccount && viewer ? (
          <Notice tone="warn" className="text-left">
            This invite was sent to {email}, and you’re signed in as {viewer.email}. Joining adds{' '}
            {viewer.email} to {teamName}.
          </Notice>
        ) : null}
      </div>
      <div className="flex flex-col gap-2.5 px-7 pb-7 max-sm:px-5">
        {viewer ? (
          <Button
            variant="primary"
            size="lg"
            className="h-10! w-full max-sm:h-11!"
            onClick={join}
            disabled={full}
            loading={pending}
          >
            {pending ? `Joining ${teamName}…` : `Join ${teamName}`}
          </Button>
        ) : (
          <a
            href={`${routes.signIn}?return_to=${encodeURIComponent(returnTo)}`}
            className={buttonClasses('primary', 'lg', 'h-10! w-full text-[14px]! max-sm:h-11!')}
          >
            Sign in to join
          </a>
        )}
      </div>
      <div className="flex items-center gap-2 border-t border-hairline bg-raised px-7 py-3 text-[12px] text-fg-subtle max-sm:px-5">
        <span className="min-w-0 flex-1 [overflow-wrap:anywhere]">
          {viewer
            ? `Signed in as ${viewer.email}`
            : `Sign in with ${email}, or any account you use.`}
        </span>
        {viewer ? <SwitchAccount returnTo={returnTo} as="link" /> : null}
      </div>
    </div>
  );
}
