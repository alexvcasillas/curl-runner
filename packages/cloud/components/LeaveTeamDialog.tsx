'use client';

import type { ActionResult } from '@/lib/action-result';
import { ConfirmDialog } from './ConfirmDialog';

/**
 * Leave a team, behind a confirm. Never offered to the only owner: Members and
 * Settings hide it, and the API's 409 backs that up.
 */
export function LeaveTeamDialog({
  open,
  onClose,
  teamName,
  email,
  onLeave,
}: {
  open: boolean;
  onClose: () => void;
  teamName: string;
  /** The signed-in person, named in "shared with …". */
  email: string;
  onLeave: () => Promise<ActionResult>;
}) {
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
