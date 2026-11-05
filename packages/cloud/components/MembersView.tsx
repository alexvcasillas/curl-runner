'use client';

import { useState } from 'react';
import type { InviteResult } from '@/app/(app)/t/[team]/members/actions';
import type { ActionResult } from '@/lib/action-result';
import { seatNote } from '@/lib/members';
import { normalizeEmail } from '@/lib/team-input';
import type { Member, Plan, Role } from '@/lib/types';
import { Avatar } from './Avatar';
import { Button } from './Button';
import { Card, CardBody, CardTitle } from './Card';
import { ConfirmDialog } from './ConfirmDialog';
import { inputClasses } from './field-styles';
import { LeaveTeamDialog } from './LeaveTeamDialog';
import { Notice } from './Notice';
import { PageHeader } from './PageHeader';
import { Pill } from './Pill';
import { UpgradeLink } from './RestrictedUpsell';
import { Select } from './Select';
import { Table, TableCell, TableHead, TableHeader, TableRow } from './Table';

export type InviteRow = { id: string; email: string; role: Role; expires: string };

const ROLES = [
  { value: 'member', label: 'Member' },
  { value: 'owner', label: 'Owner' },
];
const roleName = (r: Role) => (r === 'owner' ? 'Owner' : 'Member');
const FREE_SEATS = 3;

/** Members, pending invites and the invite form. Owners manage; anyone can leave. */
export function MembersView({
  team,
  teamName,
  me,
  owner,
  plan,
  billing,
  members,
  invites,
  costNote,
  onInvite,
  onRole,
  onRemove,
  onCancel,
  onLeave,
}: {
  team: string;
  teamName: string;
  me: { id: string; email: string };
  owner: boolean;
  plan: Plan;
  billing: boolean;
  members: Member[];
  invites: InviteRow[];
  /** What a new seat costs, when billing is on and the team is on Team. */
  costNote: string | null;
  onInvite: (email: string, role: Role) => Promise<InviteResult>;
  onRole: (userId: string, role: Role) => Promise<ActionResult>;
  onRemove: (userId: string) => Promise<ActionResult>;
  onCancel: (inviteId: string) => Promise<ActionResult>;
  onLeave: () => Promise<ActionResult>;
}) {
  const [email, setEmail] = useState('');
  const [role, setRole] = useState<Role>('member');
  const [sending, setSending] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const [limit, setLimit] = useState(false);
  const [sent, setSent] = useState<string | null>(null);
  const [roleError, setRoleError] = useState<string | null>(null);
  const [removing, setRemoving] = useState<Member | null>(null);
  const [cancelling, setCancelling] = useState<InviteRow | null>(null);
  const [leaving, setLeaving] = useState(false);

  const owners = members.filter((m) => m.role === 'owner');
  const soleOwner = owners.length === 1 && owners[0].user_id === me.id;
  const full = plan === 'free' && members.length + invites.length >= FREE_SEATS;
  const showLimit = full || limit;

  async function send(e: React.FormEvent) {
    e.preventDefault();
    if (sending || full) {
      return;
    }
    setSent(null);
    setProblem(null);
    setLimit(false);
    const clean = normalizeEmail(email);
    if (clean === null) {
      setProblem('Enter a valid email address.');
      return;
    }
    if (members.some((m) => m.email.toLowerCase() === clean)) {
      setProblem(`${clean} is already a member of ${teamName}.`);
      return;
    }
    if (invites.some((i) => i.email.toLowerCase() === clean)) {
      setProblem(`${clean} already has a pending invite.`);
      return;
    }
    setSending(true);
    try {
      const result = await onInvite(clean, role);
      if (result.ok) {
        setSent(result.email);
        setEmail('');
      } else if (result.code === 'plan_limit') {
        setLimit(true);
      } else {
        setProblem(result.message);
      }
    } catch {
      setProblem('That didn’t work. Try again.');
    } finally {
      setSending(false);
    }
  }

  async function changeRole(m: Member, next: Role) {
    setRoleError(null);
    try {
      const result = await onRole(m.user_id, next);
      if (!result.ok) {
        setRoleError(result.message);
      }
    } catch {
      setRoleError('That didn’t work. Try again.');
    }
  }

  return (
    <>
      <PageHeader title="Members">
        Members publish, share collections and make tokens. Owners also manage members and billing.
        Customers you share with aren’t members.
      </PageHeader>

      {owner ? (
        <Card aria-labelledby="invite-title">
          <CardBody>
            <CardTitle id="invite-title" className="text-[14px] leading-5">
              Invite someone
            </CardTitle>
            <form onSubmit={send} noValidate className="flex flex-wrap items-center gap-2">
              <input
                aria-label="Email address"
                type="email"
                inputMode="email"
                autoComplete="off"
                placeholder="name@example.com"
                value={email}
                disabled={full}
                aria-invalid={problem ? true : undefined}
                onChange={(e) => {
                  setEmail(e.target.value);
                  setProblem(null);
                  setSent(null);
                }}
                className={inputClasses(Boolean(problem), 'min-w-[220px] flex-1')}
              />
              <Select
                aria-label="Role"
                value={role}
                disabled={full}
                onChange={(e) => setRole(e.target.value as Role)}
                options={ROLES}
                className="w-[120px]"
              />
              <Button type="submit" variant="primary" disabled={full} loading={sending}>
                {sending ? 'Sending…' : 'Send invite'}
              </Button>
            </form>
            {showLimit ? (
              <Notice tone="warn" role="status">
                <span className="flex flex-wrap items-center gap-3">
                  <span className="min-w-[240px] flex-1">
                    The Free plan allows 3 members, counting pending invites. Upgrade to add more,
                    or cancel an invite to free a seat.
                  </span>
                  <UpgradeLink team={team} billing={billing} />
                </span>
              </Notice>
            ) : null}
            {costNote ? <span className="text-[12px] text-fg-subtle">{costNote}</span> : null}
            {sent ? (
              <Notice tone="ok" role="status">
                Invite sent to {sent}. The link works for 7 days.
              </Notice>
            ) : null}
            {problem ? (
              <span role="alert" className="text-[12px] text-danger">
                {problem}
              </span>
            ) : null}
          </CardBody>
        </Card>
      ) : null}

      {roleError ? (
        <Notice tone="err" role="alert">
          {roleError}
        </Notice>
      ) : null}

      <Table
        label="Team members"
        minWidth={560}
        footer={seatNote(plan, billing, members.length, invites.length)}
      >
        <TableHeader>
          <TableHead>Member</TableHead>
          <TableHead width={150}>Role</TableHead>
          <TableHead width={112} />
        </TableHeader>
        {members.map((m) => {
          const you = m.user_id === me.id;
          const who = m.name || m.email;
          return (
            <TableRow key={m.user_id}>
              <Avatar name={who} />
              <TableCell className="flex flex-col">
                <span className="font-medium [overflow-wrap:anywhere]">
                  {who}
                  {you ? <span className="font-normal text-fg-subtle"> (you)</span> : null}
                </span>
                {m.name ? <span className="text-[12px] text-fg-muted">{m.email}</span> : null}
              </TableCell>
              <TableCell width={150}>
                {owner ? (
                  <Select
                    aria-label={`Role for ${who}`}
                    value={m.role}
                    onChange={(e) => changeRole(m, e.target.value as Role)}
                    options={ROLES}
                    className="w-[120px] [&_select]:h-7 [&_select]:text-[12px]"
                  />
                ) : (
                  <span className="text-fg-muted">{roleName(m.role)}</span>
                )}
              </TableCell>
              <TableCell width={112} className="flex justify-end">
                {you ? (
                  <Button size="sm" onClick={() => setLeaving(true)}>
                    Leave team
                  </Button>
                ) : owner ? (
                  <Button size="sm" aria-label={`Remove ${who}`} onClick={() => setRemoving(m)}>
                    Remove
                  </Button>
                ) : null}
              </TableCell>
            </TableRow>
          );
        })}
      </Table>

      <Card aria-labelledby="pending-title">
        <CardBody className="pb-3">
          <CardTitle id="pending-title">Pending invites</CardTitle>
          <p className="m-0 leading-normal text-fg-muted">
            Invites expire after 7 days. Pending invites count toward the Free plan’s 3 members.
          </p>
        </CardBody>
        {invites.length === 0 ? (
          <p className="m-0 border-t border-hairline px-6 py-4 text-fg-subtle max-sm:px-4">
            No pending invites.
          </p>
        ) : (
          <ul className="m-0 list-none p-0">
            {invites.map((i) => (
              <li
                key={i.id}
                className="flex flex-wrap items-center gap-3 border-t border-hairline px-6 py-3 max-sm:px-4"
              >
                <Avatar kind="pending" />
                <span className="flex min-w-[180px] flex-1 flex-col">
                  <span className="font-mono text-[12px] [overflow-wrap:anywhere]">{i.email}</span>
                  <span className="text-[12px] text-fg-muted">{roleName(i.role)}</span>
                </span>
                <Pill tone="warn">{i.expires}</Pill>
                {owner ? (
                  <Button
                    size="sm"
                    variant="ghost"
                    className="text-danger! hover:text-danger!"
                    aria-label={`Cancel invite to ${i.email}`}
                    onClick={() => setCancelling(i)}
                  >
                    Cancel invite
                  </Button>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </Card>

      <ConfirmDialog
        open={removing !== null}
        onClose={() => setRemoving(null)}
        title={`Remove ${removing?.name || removing?.email || ''} from ${teamName}?`}
        description={`They lose access to ${teamName}’s collections and tokens straight away.`}
        confirmLabel="Remove member"
        pendingLabel="Removing…"
        onConfirm={async () => {
          if (!removing) {
            return;
          }
          const result = await onRemove(removing.user_id);
          if (!result.ok) {
            throw new Error(result.message);
          }
          setRemoving(null);
        }}
      >
        {removing ? (
          <>
            <div className="flex items-center gap-3 rounded-[6px] border border-hairline px-3 py-2.5">
              <Avatar name={removing.name || removing.email} />
              <span className="flex min-w-0 flex-1 flex-col">
                <span className="font-medium [overflow-wrap:anywhere]">
                  {removing.name || removing.email}
                </span>
                <span className="text-[12px] text-fg-muted [overflow-wrap:anywhere]">
                  {removing.email} · {roleName(removing.role)}
                </span>
              </span>
            </div>
            <p className="m-0 text-[12px] text-fg-subtle">
              Tokens they created keep working. Revoke them under Tokens if you need to.
              {plan === 'team' && billing ? ' The next invoice drops one seat.' : ''}
            </p>
          </>
        ) : null}
      </ConfirmDialog>

      <ConfirmDialog
        open={cancelling !== null}
        onClose={() => setCancelling(null)}
        title={`Cancel the invite to ${cancelling?.email ?? ''}?`}
        description="The link in their email stops working. You can invite them again later."
        confirmLabel="Cancel invite"
        cancelLabel="Keep invite"
        pendingLabel="Cancelling…"
        onConfirm={async () => {
          if (!cancelling) {
            return;
          }
          const result = await onCancel(cancelling.id);
          if (!result.ok) {
            throw new Error(result.message);
          }
          setCancelling(null);
        }}
      >
        {cancelling ? (
          <div className="flex items-center gap-3 rounded-[6px] border border-hairline px-3 py-2.5">
            <Avatar kind="pending" />
            <span className="flex min-w-0 flex-1 flex-col">
              <span className="font-mono text-[12px] [overflow-wrap:anywhere]">
                {cancelling.email}
              </span>
              <span className="text-[12px] text-fg-muted">
                {roleName(cancelling.role)} · {cancelling.expires.replace('Expires', 'expires')}
              </span>
            </span>
          </div>
        ) : null}
      </ConfirmDialog>

      <LeaveTeamDialog
        open={leaving}
        onClose={() => setLeaving(false)}
        teamName={teamName}
        email={me.email}
        soleOwner={soleOwner}
        onLeave={onLeave}
      />
    </>
  );
}
