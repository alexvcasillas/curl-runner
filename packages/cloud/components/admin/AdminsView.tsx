'use client';

import { useEffect, useId, useState } from 'react';
import type { ActionResult } from '@/lib/action-result';
import { can, PERM_LABELS, PERMS, type Perm, permissionsLabel } from '@/lib/admin';
import type { AdminRow } from '@/lib/admin-settings';
import { firstName } from '@/lib/admin-users';
import { relativeTime, shortDate } from '@/lib/time';
import { Avatar } from '../Avatar';
import { Button } from '../Button';
import { Checkbox } from '../Checkbox';
import { ConfirmDialog } from '../ConfirmDialog';
import { Dialog } from '../Dialog';
import { Field } from '../Field';
import { Notice } from '../Notice';
import { PageHeader } from '../PageHeader';
import { Pill } from '../Pill';
import { Table, TableCell, TableHead, TableHeader, TableRow } from '../Table';

export type AdminsActions = {
  add: (email: string, permissions: Perm[]) => Promise<ActionResult>;
  update: (userId: string, permissions: Perm[]) => Promise<ActionResult>;
  remove: (userId: string) => Promise<ActionResult>;
};

type Dialog_ =
  | { kind: 'add' }
  | { kind: 'edit'; row: AdminRow }
  | { kind: 'remove'; row: AdminRow };

const nameOf = (r: Pick<AdminRow, 'name' | 'email'>) => r.name || r.email;

/** Every permission as a labelled box, in the server's order. */
function PermissionBoxes({
  value,
  onChange,
}: {
  value: ReadonlySet<Perm>;
  onChange: (next: Set<Perm>) => void;
}) {
  return (
    <div className="flex flex-col gap-2.5">
      {PERMS.map((p) => (
        <Checkbox
          key={p}
          aria-label={PERM_LABELS[p].title}
          checked={value.has(p)}
          onChange={(e) => {
            const next = new Set(value);
            if (e.target.checked) {
              next.add(p);
            } else {
              next.delete(p);
            }
            onChange(next);
          }}
          label={
            <span className="flex flex-col">
              <span className="text-[13px] text-fg">{PERM_LABELS[p].title}</span>
              <span className="text-[12px] text-fg-muted">{PERM_LABELS[p].desc}</span>
            </span>
          }
          className="items-start [&>span:nth-child(2)]:mt-0.5"
        />
      ))}
    </div>
  );
}

const ordered = (s: ReadonlySet<Perm>) => PERMS.filter((p) => s.has(p));

/** Add an existing account as an admin, or change what one can do. */
function AdminDialog({
  open,
  row,
  onClose,
  onSubmit,
  onRemove,
}: {
  open: boolean;
  /** The admin being edited; null adds a new one. */
  row: AdminRow | null;
  onClose: () => void;
  onSubmit: (email: string, permissions: Perm[]) => Promise<ActionResult>;
  onRemove: () => void;
}) {
  const [email, setEmail] = useState('');
  const [perms, setPerms] = useState<Set<Perm>>(new Set());
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const formId = useId();

  useEffect(() => {
    if (open) {
      setEmail('');
      setPerms(new Set(row?.permissions ?? []));
      setPending(false);
      setError(null);
    }
  }, [open, row]);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (pending) {
      return;
    }
    setPending(true);
    setError(null);
    try {
      const result = await onSubmit(email, ordered(perms));
      if (result.ok) {
        onClose();
      } else {
        setError(result.message);
      }
    } catch {
      setError('That didn’t work. Try again.');
    } finally {
      setPending(false);
    }
  }

  return (
    <Dialog
      open={open}
      onClose={() => {
        if (!pending) {
          onClose();
        }
      }}
      title={row ? `Permissions for ${nameOf(row)}` : 'Add an admin'}
      description={
        row
          ? `${firstName(row)} sees the change on their next page load.`
          : 'They need an account already, and nothing is emailed. Pick only what they need.'
      }
      footer={
        <>
          {row ? (
            <Button variant="danger" onClick={onRemove} disabled={pending} className="mr-auto">
              Remove admin
            </Button>
          ) : null}
          <Button onClick={onClose} disabled={pending}>
            Cancel
          </Button>
          <Button type="submit" form={formId} variant="primary" loading={pending}>
            {row ? 'Save permissions' : 'Add admin'}
          </Button>
        </>
      }
    >
      <form id={formId} onSubmit={submit} className="flex flex-col gap-3.5">
        {row ? null : (
          <Field
            label="Email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="marco@curl-runner.com"
            autoComplete="off"
            required
          />
        )}
        <PermissionBoxes value={perms} onChange={setPerms} />
        <span className="text-[12px] text-fg-subtle">
          Every admin can see users, teams, collections and the activity log. Changes apply on the
          next page load.
        </span>
        {error ? (
          <Notice tone="err" role="alert">
            {error}
          </Notice>
        ) : null}
      </form>
    </Dialog>
  );
}

/** Who can open the admin panel, and what each of them can change. */
export function AdminsView({
  admins,
  permissions,
  now,
  actions,
}: {
  admins: AdminRow[];
  permissions: Perm[];
  now: string;
  actions: AdminsActions;
}) {
  const canWrite = can(permissions, 'admins');
  const clock = new Date(now);
  const [dialog, setDialog] = useState<Dialog_ | null>(null);
  const close = () => setDialog(null);
  const editing = dialog?.kind === 'edit' ? dialog.row : null;
  const removing = dialog?.kind === 'remove' ? dialog.row : null;

  return (
    <>
      <PageHeader
        title="Admins"
        actions={
          <Button
            variant="primary"
            disabled={!canWrite}
            title={canWrite ? undefined : 'Read only: you need the admins permission.'}
            onClick={() => setDialog({ kind: 'add' })}
          >
            Add admin
          </Button>
        }
      >
        People who can open this panel, and what each of them can change. Owners come from the
        environment and have every permission.
      </PageHeader>

      <Table
        label="Admins"
        minWidth={820}
        footer={<span>Removing an admin keeps their user account and their teams.</span>}
      >
        <TableHeader>
          <TableHead>Admin</TableHead>
          <TableHead width={260}>Permissions</TableHead>
          <TableHead width={110}>Last active</TableHead>
          <TableHead width={130}>Added by</TableHead>
          <TableHead width={150} />
        </TableHeader>
        {admins.map((a) => (
          <TableRow key={a.email} className="min-h-[56px] py-2">
            <TableCell>
              <span className="flex min-w-0 items-center gap-2.5">
                <Avatar name={nameOf(a)} size={28} />
                <span className="flex min-w-0 flex-col">
                  <span className="font-medium [overflow-wrap:anywhere]">{nameOf(a)}</span>
                  {a.name ? (
                    <span className="text-[12px] text-fg-muted [overflow-wrap:anywhere]">
                      {a.email}
                    </span>
                  ) : null}
                </span>
              </span>
            </TableCell>
            <TableCell width={260}>
              <span className="flex flex-wrap items-center gap-x-2 gap-y-1">
                <Pill tone={a.role === 'owner' ? 'strong' : 'neutral'}>
                  {a.role === 'owner' ? 'Owner' : 'Admin'}
                </Pill>
                <span className="text-[12px] text-fg-muted">{permissionsLabel(a.permissions)}</span>
              </span>
            </TableCell>
            <TableCell width={110} className="text-fg-muted">
              {a.last_active
                ? relativeTime(a.last_active, clock)
                : a.user_id
                  ? 'Never'
                  : 'Not signed in yet'}
            </TableCell>
            <TableCell width={130} className="text-[12px] text-fg-muted">
              {a.role === 'owner'
                ? 'Environment'
                : a.added_by
                  ? `${nameOf(a.added_by)}${a.added_at ? `, ${shortDate(a.added_at)}` : ''}`
                  : '—'}
            </TableCell>
            <TableCell width={150} className="text-right">
              {a.role === 'admin' && canWrite ? (
                <Button
                  size="sm"
                  aria-label={`Edit permissions for ${nameOf(a)}`}
                  onClick={() => setDialog({ kind: 'edit', row: a })}
                >
                  Edit permissions
                </Button>
              ) : null}
            </TableCell>
          </TableRow>
        ))}
      </Table>

      <AdminDialog
        open={dialog?.kind === 'add' || editing !== null}
        row={editing}
        onClose={close}
        onSubmit={(email, perms) =>
          editing?.user_id ? actions.update(editing.user_id, perms) : actions.add(email, perms)
        }
        onRemove={() => editing && setDialog({ kind: 'remove', row: editing })}
      />

      <ConfirmDialog
        open={removing !== null}
        onClose={close}
        title={removing ? `Remove ${nameOf(removing)} as an admin?` : 'Remove admin'}
        description={
          removing
            ? `${nameOf(removing)} keeps their account and their teams. They just can’t open the admin panel any more.`
            : undefined
        }
        confirmLabel="Remove admin"
        pendingLabel="Removing…"
        onConfirm={async () => {
          if (!removing?.user_id) {
            return;
          }
          const result = await actions.remove(removing.user_id);
          if (!result.ok) {
            throw new Error(result.message);
          }
          close();
        }}
      />
    </>
  );
}
