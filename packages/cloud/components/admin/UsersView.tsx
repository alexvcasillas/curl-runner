'use client';

import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useEffect, useRef, useState } from 'react';
import { adminRoutes, can, formatInt, type Perm } from '@/lib/admin';
import { nextPage, previousPage } from '@/lib/admin-paging';
import {
  displayName,
  firstName,
  methodsLabel,
  teamsLabel,
  USER_FILTERS,
  type UserFilter,
  type UserRow,
  type UsersPage,
  usersExportHref,
  usersHref,
} from '@/lib/admin-users';
import { relativeTime } from '@/lib/time';
import { Avatar } from '../Avatar';
import { Button } from '../Button';
import { ButtonLink } from '../ButtonLink';
import { buttonClasses } from '../button-styles';
import { Checkbox } from '../Checkbox';
import { ConfirmDialog } from '../ConfirmDialog';
import { EmptyState } from '../EmptyState';
import { inputClasses } from '../field-styles';
import { Menu, MenuItem, MenuSeparator } from '../Menu';
import { Notice } from '../Notice';
import { PageHeader } from '../PageHeader';
import { Pill } from '../Pill';
import { Segmented } from '../Segmented';
import { Table, TableCell, TableHead, TableHeader, TableRow } from '../Table';
import { EmailUserDialog } from './EmailUserDialog';
import { SuspendDialog } from './SuspendDialog';
import type { UserListActions } from './user-actions';

const SEARCH_DELAY = 350;
const plural = (n: number, one: string) => `${n} ${n === 1 ? one : `${one}s`}`;
const LEFT_ALONE = 'Owners, you, other admins and anyone already suspended are left alone.';

type Dialog =
  | { kind: 'suspend' | 'delete' | 'signout' | 'unsuspend' | 'email'; user: UserRow }
  | { kind: 'bulk-suspend' | 'bulk-signout' };

type Note = { tone: 'ok' | 'warn'; text: string };

/** Everyone with an account. Reads are open to any admin; writes need `users`, email needs `emails`. */
export function UsersView({
  page,
  q,
  filter,
  cursor,
  trail,
  pageSize,
  permissions,
  now,
  actions,
}: {
  page: UsersPage;
  q: string;
  filter: UserFilter;
  cursor?: string;
  trail: string[];
  pageSize: number;
  permissions: Perm[];
  /** ISO time the page was rendered, so server and browser agree on "6 minutes ago". */
  now: string;
  actions: UserListActions;
}) {
  const router = useRouter();
  const canWrite = can(permissions, 'users');
  const canEmail = can(permissions, 'emails');
  const clock = new Date(now);

  const [search, setSearch] = useState(q);
  const [selected, setSelected] = useState<ReadonlySet<string>>(new Set());
  const [dialog, setDialog] = useState<Dialog | null>(null);
  const [note, setNote] = useState<Note | null>(null);

  const query = { q, filter, trail: [] as string[] };

  // A new page (or a refresh after an action) starts with nothing selected.
  // biome-ignore lint/correctness/useExhaustiveDependencies: the page object is the trigger
  useEffect(() => {
    setSelected(new Set());
  }, [page]);

  // Search as they type, after a pause. Enter goes now.
  const lastQ = useRef(q);
  lastQ.current = q;
  useEffect(() => {
    const next = search.trim();
    if (next === lastQ.current) {
      return;
    }
    const timer = setTimeout(
      () => router.replace(usersHref({ q: next, filter, trail: [] })),
      SEARCH_DELAY,
    );
    return () => clearTimeout(timer);
  }, [search, filter, router]);

  const items = page.items;
  const chosen = items.filter((u) => selected.has(u.id));
  const allOn = items.length > 0 && chosen.length === items.length;
  const start = trail.length * pageSize + 1;
  const end = start + items.length - 1;
  const nameOf = (id: string) => {
    const u = items.find((i) => i.id === id);
    return u ? displayName(u) : 'a user not on this page';
  };

  function toggle(id: string, on: boolean) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (on) {
        next.add(id);
      } else {
        next.delete(id);
      }
      return next;
    });
  }

  async function bulk(action: 'sign_out' | 'suspend', reason: string) {
    const ids = chosen.map((u) => u.id);
    const result = await actions.bulk(ids, action, reason);
    if (!result.ok) {
      throw new Error(result.message);
    }
    const verb = action === 'sign_out' ? 'Signed out' : 'Suspended';
    let text = `${verb} ${plural(result.done, 'user')}.`;
    if (result.skipped.length > 0) {
      text += ` Skipped ${result.skipped.map(nameOf).join(', ')}.`;
      if (action === 'suspend') {
        text += ` ${LEFT_ALONE}`;
      }
    }
    setNote({ tone: result.skipped.length > 0 ? 'warn' : 'ok', text });
    setSelected(new Set());
    setDialog(null);
  }

  /** A one-user action for a confirm dialog: the API's message stays in the dialog. */
  async function one(result: Promise<{ ok: boolean; message?: string }>, done: string) {
    const r = await result;
    if (!r.ok) {
      throw new Error(r.message);
    }
    setNote({ tone: 'ok', text: done });
    setDialog(null);
  }

  const target = dialog && 'user' in dialog ? dialog.user : null;
  const targetName = target ? displayName(target) : '';
  const targetFirst = target ? firstName(target) : '';
  const close = () => setDialog(null);

  return (
    <>
      <PageHeader
        title="Users"
        actions={
          <a href={usersExportHref({ q, filter })} download className={buttonClasses()}>
            Export CSV
          </a>
        }
      >
        Everyone with an account: team members and customers who signed in to open a restricted
        link. Every action here is logged.
      </PageHeader>

      <div className="flex flex-wrap items-center gap-3">
        <form
          role="search"
          className="relative flex min-w-[220px] max-w-[340px] flex-1 items-center"
          onSubmit={(e) => {
            e.preventDefault();
            router.push(usersHref({ q: search.trim(), filter, trail: [] }));
          }}
        >
          <svg
            aria-hidden="true"
            width="14"
            height="14"
            viewBox="0 0 16 16"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            className="pointer-events-none absolute left-2.5 text-fg-subtle"
          >
            <circle cx="7" cy="7" r="4.5" />
            <path d="M10.5 10.5L14 14" />
          </svg>
          <input
            type="search"
            aria-label="Search users"
            placeholder="Search by name or email"
            value={search}
            maxLength={200}
            autoComplete="off"
            onChange={(e) => setSearch(e.target.value)}
            className={inputClasses(false, 'pl-8')}
          />
        </form>
        <Segmented
          label="Filter users"
          options={USER_FILTERS.map((f) => ({ value: f.value, label: f.label }))}
          value={filter}
          onChange={(value) => router.push(usersHref({ ...query, filter: value }))}
        />
      </div>

      {note ? (
        <Notice tone={note.tone} role="status">
          {note.text}
        </Notice>
      ) : null}

      {chosen.length > 0 ? (
        <div
          role="toolbar"
          aria-label="Bulk actions"
          className="flex flex-wrap items-center gap-2 rounded-[8px] border border-control bg-raised px-3 py-2"
        >
          <span className="mr-auto">{chosen.length} selected</span>
          {canEmail ? (
            <ButtonLink size="sm" href={adminRoutes.emails}>
              Email them
            </ButtonLink>
          ) : (
            <Button size="sm" disabled title="Read only: you need the emails permission.">
              Email them
            </Button>
          )}
          <Button size="sm" onClick={() => setDialog({ kind: 'bulk-signout' })}>
            Sign out everywhere
          </Button>
          <Button size="sm" variant="danger" onClick={() => setDialog({ kind: 'bulk-suspend' })}>
            Suspend
          </Button>
          <Button size="sm" variant="ghost" onClick={() => setSelected(new Set())}>
            Clear
          </Button>
        </div>
      ) : null}

      {items.length === 0 ? (
        <EmptyState
          title={q || filter !== 'all' ? 'No users match' : 'No users yet'}
          actions={
            q || filter !== 'all' ? (
              <ButtonLink href={adminRoutes.users}>Clear search</ButtonLink>
            ) : null
          }
        >
          {q || filter !== 'all'
            ? 'Try another name, email or filter.'
            : 'Accounts show up here once someone signs in.'}
        </EmptyState>
      ) : (
        <Table
          label="Users"
          minWidth={canWrite ? 940 : 900}
          footer={
            <>
              <span className="mr-auto">
                Showing {formatInt(start)}–{formatInt(end)} of {formatInt(page.total)}
              </span>
              <span className="flex gap-1.5">
                {trail.length > 0 ? (
                  <ButtonLink
                    size="sm"
                    href={usersHref({
                      q,
                      filter,
                      ...previousPage(trail),
                    })}
                  >
                    Previous
                  </ButtonLink>
                ) : (
                  <Button size="sm" disabled>
                    Previous
                  </Button>
                )}
                {page.next_cursor ? (
                  <ButtonLink
                    size="sm"
                    href={usersHref({
                      q,
                      filter,
                      ...nextPage(trail, cursor, page.next_cursor),
                    })}
                  >
                    Next
                  </ButtonLink>
                ) : (
                  <Button size="sm" disabled>
                    Next
                  </Button>
                )}
              </span>
            </>
          }
        >
          <TableHeader>
            {canWrite ? (
              <TableHead width={16}>
                <Checkbox
                  label={<span className="sr-only">Select all on this page</span>}
                  checked={allOn}
                  onChange={(e) =>
                    setSelected(e.target.checked ? new Set(items.map((u) => u.id)) : new Set())
                  }
                />
              </TableHead>
            ) : null}
            <TableHead>User</TableHead>
            <TableHead width={120}>Sign-in</TableHead>
            <TableHead width={150}>Teams</TableHead>
            <TableHead width={90} className="text-right">
              Shared in
            </TableHead>
            <TableHead width={110}>Last active</TableHead>
            <TableHead width={90}>Status</TableHead>
            <TableHead width={28} />
          </TableHeader>
          {items.map((u) => {
            const name = displayName(u);
            const first = firstName(u);
            const suspended = u.status === 'suspended';
            return (
              <TableRow key={u.id} className="min-h-[52px] py-2">
                {canWrite ? (
                  <TableCell width={16}>
                    <Checkbox
                      label={<span className="sr-only">Select {name}</span>}
                      checked={selected.has(u.id)}
                      onChange={(e) => toggle(u.id, e.target.checked)}
                    />
                  </TableCell>
                ) : null}
                <TableCell>
                  <Link
                    href={adminRoutes.user(u.id)}
                    className="flex min-w-0 items-center gap-2.5 text-fg hover:text-fg"
                  >
                    <Avatar name={name} size={28} />
                    <span className="flex min-w-0 flex-col">
                      <span className="font-medium [overflow-wrap:anywhere]">{name}</span>
                      {u.name ? (
                        <span className="text-[12px] text-fg-muted [overflow-wrap:anywhere]">
                          {u.email}
                        </span>
                      ) : null}
                    </span>
                  </Link>
                </TableCell>
                <TableCell width={120} className="text-[12px] text-fg-muted">
                  {methodsLabel(u.methods)}
                </TableCell>
                <TableCell width={150} className="text-[12px] text-fg-muted">
                  {teamsLabel(u.teams)}
                </TableCell>
                <TableCell width={90} className="text-right tabular-nums text-fg-muted">
                  {u.shared_in}
                </TableCell>
                <TableCell width={110} className="text-fg-muted">
                  {u.last_active ? relativeTime(u.last_active, clock) : 'Never'}
                </TableCell>
                <TableCell width={90}>
                  <Pill tone={suspended ? 'err' : 'neutral'}>
                    {suspended ? 'Suspended' : 'Active'}
                  </Pill>
                </TableCell>
                <TableCell width={28}>
                  <Menu
                    label="User actions"
                    triggerLabel={`Actions for ${name}`}
                    align="end"
                    fixed
                    width={220}
                    triggerClassName={buttonClasses('ghost', 'sm', 'w-7 px-0!')}
                    trigger={
                      <svg
                        aria-hidden="true"
                        width="16"
                        height="16"
                        viewBox="0 0 16 16"
                        fill="currentColor"
                      >
                        <circle cx="3.5" cy="8" r="1.2" />
                        <circle cx="8" cy="8" r="1.2" />
                        <circle cx="12.5" cy="8" r="1.2" />
                      </svg>
                    }
                  >
                    <MenuItem href={adminRoutes.user(u.id)}>Open</MenuItem>
                    {canEmail ? (
                      <MenuItem onSelect={() => setDialog({ kind: 'email', user: u })}>
                        Email {first}
                      </MenuItem>
                    ) : null}
                    {canWrite ? (
                      <>
                        <MenuItem onSelect={() => setDialog({ kind: 'signout', user: u })}>
                          Sign out everywhere
                        </MenuItem>
                        <MenuSeparator />
                        {suspended ? (
                          <MenuItem onSelect={() => setDialog({ kind: 'unsuspend', user: u })}>
                            Lift suspension
                          </MenuItem>
                        ) : (
                          <MenuItem onSelect={() => setDialog({ kind: 'suspend', user: u })}>
                            <span className="text-danger">Suspend…</span>
                          </MenuItem>
                        )}
                        <MenuItem onSelect={() => setDialog({ kind: 'delete', user: u })}>
                          <span className="text-danger">Delete user…</span>
                        </MenuItem>
                      </>
                    ) : null}
                  </Menu>
                </TableCell>
              </TableRow>
            );
          })}
        </Table>
      )}

      <SuspendDialog
        open={dialog?.kind === 'suspend'}
        onClose={close}
        title={`Suspend ${targetName}?`}
        description={`${targetFirst} can’t sign in, and every device is signed out now. Teams, tokens and collections keep working, so CI isn’t broken.`}
        confirmLabel="Suspend user"
        notifyLabel={`Email ${targetFirst} that the account is suspended`}
        onSubmit={(reason, notify) =>
          target ? actions.suspend(target.id, reason, notify) : Promise.reject()
        }
        onDone={() => setNote({ tone: 'ok', text: `Suspended ${targetName}.` })}
      />

      <SuspendDialog
        open={dialog?.kind === 'bulk-suspend'}
        onClose={close}
        title={`Suspend ${plural(chosen.length, 'user')}?`}
        description={`They can’t sign in, and every device is signed out now. Teams, tokens and collections keep working, so CI isn’t broken. ${LEFT_ALONE}`}
        confirmLabel={`Suspend ${plural(chosen.length, 'user')}`}
        onSubmit={async (reason) => {
          try {
            await bulk('suspend', reason);
            return { ok: true as const };
          } catch (e) {
            return {
              ok: false as const,
              message: e instanceof Error ? e.message : 'That didn’t work. Try again.',
            };
          }
        }}
        onDone={() => {}}
      />

      <ConfirmDialog
        open={dialog?.kind === 'bulk-signout'}
        onClose={close}
        title={`Sign out ${plural(chosen.length, 'user')} everywhere?`}
        description="Every device they’re signed in on stops working now. They can sign in again."
        confirmLabel={`Sign out ${plural(chosen.length, 'user')}`}
        pendingLabel="Signing out…"
        onConfirm={() => bulk('sign_out', '')}
      />

      <ConfirmDialog
        open={dialog?.kind === 'signout'}
        onClose={close}
        title={`Sign out ${targetName} everywhere?`}
        description="Every device they’re signed in on stops working now. They can sign in again."
        confirmLabel="Sign out everywhere"
        pendingLabel="Signing out…"
        onConfirm={() =>
          target
            ? one(actions.signOut(target.id), `Signed out ${targetName} everywhere.`)
            : undefined
        }
      />

      <ConfirmDialog
        open={dialog?.kind === 'unsuspend'}
        onClose={close}
        title={`Lift the suspension on ${targetName}?`}
        description={`${targetFirst} can sign in again. Their teams and collections were never touched.`}
        confirmLabel="Lift suspension"
        pendingLabel="Lifting…"
        variant="primary"
        onConfirm={() =>
          target
            ? one(actions.unsuspend(target.id), `Lifted the suspension on ${targetName}.`)
            : undefined
        }
      />

      <ConfirmDialog
        open={dialog?.kind === 'delete'}
        onClose={close}
        title={`Delete ${targetName}?`}
        description="Removes the account, sign-ins, devices and access-list matches. Teams stay with their other owners. This can’t be undone."
        confirmLabel="Delete user"
        pendingLabel="Deleting…"
        confirmText={target?.email ?? ''}
        onConfirm={() =>
          target ? one(actions.remove(target.id), `Deleted ${targetName}.`) : undefined
        }
      />

      <EmailUserDialog
        open={dialog?.kind === 'email'}
        onClose={close}
        name={targetName}
        onSubmit={(subject, body) =>
          target ? actions.email(target.id, subject, body) : Promise.reject()
        }
        onDone={() => setNote({ tone: 'ok', text: `Email queued for ${target?.email}.` })}
      />
    </>
  );
}
