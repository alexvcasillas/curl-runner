'use client';

import type { ActionResult } from '@/lib/action-result';
import { Button } from './Button';
import { ButtonLink } from './ButtonLink';
import { ConfirmDialog } from './ConfirmDialog';
import { Dialog } from './Dialog';

/**
 * Leave a team. The only owner can't: the dialog says to make someone else an
 * owner first, and the API's 409 backs that up. Used on Members and Settings.
 */
export function LeaveTeamDialog({
  open,
  onClose,
  teamName,
  email,
  soleOwner,
  onLeave,
  membersHref,
}: {
  open: boolean;
  onClose: () => void;
  teamName: string;
  /** The signed-in person, named in "shared with …". */
  email: string;
  soleOwner: boolean;
  onLeave: () => Promise<ActionResult>;
  /** Where "Go to members" points. Left out on the Members page itself. */
  membersHref?: string;
}) {
  if (soleOwner) {
    return (
      <Dialog
        open={open}
        onClose={onClose}
        role="alertdialog"
        title="Make someone else an owner first"
        description={`A team needs at least one owner, and you’re the only one in ${teamName}.`}
        footer={
          <>
            <Button onClick={onClose}>Close</Button>
            {membersHref ? (
              <ButtonLink href={membersHref} variant="primary">
                Go to members
              </ButtonLink>
            ) : null}
          </>
        }
      >
        <p className="m-0 text-[12px] text-fg-subtle">
          Change a member’s role to Owner on the Members page, then leave.
        </p>
      </Dialog>
    );
  }
  return (
    <ConfirmDialog
      open={open}
      onClose={onClose}
      title={`Leave ${teamName}?`}
      description="You lose access to its collections. An owner can invite you again."
      confirmLabel="Leave team"
      pendingLabel="Leaving…"
      onConfirm={async () => {
        const result = await onLeave();
        if (!result.ok) {
          throw new Error(result.message);
        }
      }}
    >
      <p className="m-0 text-[12px] text-fg-subtle">
        Collections shared with {email} still open for you, from Shared with me.
      </p>
    </ConfirmDialog>
  );
}
