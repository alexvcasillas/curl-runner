'use client';

import Link from 'next/link';
import { useState } from 'react';
import type { ActionResult } from '@/lib/action-result';
import { adminRoutes, can, type Perm } from '@/lib/admin';
import {
  displayName,
  eventLine,
  firstName,
  providerName,
  type UserDetail,
} from '@/lib/admin-users';
import { initials } from '@/lib/initials';
import { formatDate, relativeTimeInline, shortDate } from '@/lib/time';
import { Banner } from '../Banner';
import { Breadcrumb } from '../Breadcrumb';
import { Button } from '../Button';
import { Card } from '../Card';
import { ConfirmDialog } from '../ConfirmDialog';
import { Notice } from '../Notice';
import { Pill, type PillTone } from '../Pill';
import { EmailUserDialog } from './EmailUserDialog';
import { SuspendDialog } from './SuspendDialog';
import type { UserDetailActions } from './user-actions';

const READ_ONLY = 'Read only: you need the users permission.';
const READ_ONLY_EMAIL = 'Read only: you need the emails permission.';
const ROW =
  'flex min-h-[52px] items-center gap-3 border-b border-hairline px-4 py-2 last:border-b-0';

type Dialog =
  | { kind: 'suspend' | 'unsuspend' | 'signout' | 'delete' | 'email' }
  | { kind: 'unlink'; provider: string };

function planPill(plan: UserDetail['teams'][number]): { tone: PillTone; label: string } {
  if (plan.plan !== 'team') {
    return { tone: 'neutral', label: 'Free' };
  }
  if (plan.plan_source === 'override') {
    return { tone: 'info', label: 'Team · Granted' };
  }
  return { tone: 'ok', label: 'Team · Stripe' };
}

const KIND: Record<string, { name: string; fallback: string }> = {
  web: { name: 'Web', fallback: 'Browser' },
  device: { name: 'Device', fallback: 'CLI or studio' },
};

/** "Nov 5 13:02", UTC, for the events list. */
function eventTime(iso: string): string {
  const d = new Date(iso);
  const hh = String(d.getUTCHours()).padStart(2, '0');
  const mm = String(d.getUTCMinutes()).padStart(2, '0');
  return `${shortDate(iso)} ${hh}:${mm}`;
}

function CardHeader({
  id,
  title,
  children,
}: {
  id: string;
  title: string;
  children?: React.ReactNode;
}) {
  return (
    <div className="flex min-h-11 flex-wrap items-center gap-2 border-b border-hairline px-4">
      <h2 id={id} className="m-0 mr-auto text-[14px] font-semibold">
        {title}
      </h2>
      {children}
    </div>
  );
}

function Empty({ children }: { children: React.ReactNode }) {
  return <p className="m-0 px-4 py-3 text-fg-subtle">{children}</p>;
}

/** One account: who they are, what they belong to, and what an admin can do to it. */
export function UserDetailView({
  detail,
  permissions,
  now,
  actions,
}: {
  detail: UserDetail;
  permissions: Perm[];
  now: string;
  actions: UserDetailActions;
}) {
  const { user } = detail;
  const clock = new Date(now);
  const canWrite = can(permissions, 'users');
  const canEmail = can(permissions, 'emails');
  const name = displayName(user);
  const first = firstName(user);
  const suspended = user.status === 'suspended';

  const [dialog, setDialog] = useState<Dialog | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  // The API's refusal to delete, shown in the Delete card the way the board draws it.
  const [blocked, setBlocked] = useState<string | null>(null);

  const close = () => setDialog(null);

  /** For a confirm dialog: the API's message stays in the dialog. */
  async function confirm(result: Promise<ActionResult>, done: string) {
    const r = await result;
    if (!r.ok) {
      throw new Error(r.message);
    }
    setProblem(null);
    setNote(done);
    setDialog(null);
  }

  async function revoke(sessionId: string, label: string) {
    setBusy(sessionId);
    setProblem(null);
    setNote(null);
    try {
      const r = await actions.revokeSession(user.id, sessionId);
      if (r.ok) {
        setNote(`Signed out ${label}.`);
      } else {
        setProblem(r.message);
      }
    } catch {
      setProblem('That didn’t work. Try again.');
    } finally {
      setBusy(null);
    }
  }

  async function remove() {
    const r = await actions.remove(user.id);
    if (r.ok) {
      return;
    }
    if (r.code === 'conflict') {
      setBlocked(r.message);
      setDialog(null);
      return;
    }
    throw new Error(r.message);
  }

  const su = user.suspension;

  return (
    <>
      <Breadcrumb items={[{ label: 'Users', href: adminRoutes.users }, { label: name }]} />

      {suspended ? (
        <Banner
          tone="err"
          title={
            su
              ? `Suspended${su.by ? ` by ${su.by}` : ''} on ${shortDate(su.at)}${su.reason ? `: “${su.reason}”` : '.'}`
              : 'Suspended.'
          }
          action={
            <Button
              size="sm"
              disabled={!canWrite}
              title={canWrite ? undefined : READ_ONLY}
              onClick={() => setDialog({ kind: 'unsuspend' })}
            >
              Lift suspension
            </Button>
          }
        >
          {first} can’t sign in, and every session was signed out. Teams and collections are
          untouched.
        </Banner>
      ) : null}

      {note ? (
        <Notice tone="ok" role="status">
          {note}
        </Notice>
      ) : null}
      {problem ? (
        <Notice tone="err" role="alert">
          {problem}
        </Notice>
      ) : null}

      <div className="flex flex-wrap items-center gap-4">
        <span
          aria-hidden="true"
          className="inline-grid size-14 shrink-0 place-items-center rounded-full bg-surface-300 text-[18px] font-medium text-fg-muted"
        >
          {initials(name)}
        </span>
        <div className="flex min-w-[240px] flex-1 flex-col gap-0.5">
          <div className="flex flex-wrap items-center gap-2">
            <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em] [overflow-wrap:anywhere]">
              {name}
            </h1>
            <Pill tone={suspended ? 'err' : 'ok'}>{suspended ? 'Suspended' : 'Active'}</Pill>
          </div>
          <span className="text-fg-muted [overflow-wrap:anywhere]">
            {user.email} · joined {formatDate(user.created_at)}
            {user.last_active
              ? ` · last active ${relativeTimeInline(user.last_active, clock)}`
              : ''}
          </span>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            disabled={!canEmail}
            title={canEmail ? undefined : READ_ONLY_EMAIL}
            onClick={() => setDialog({ kind: 'email' })}
          >
            Email {first}
          </Button>
          <Button
            disabled={!canWrite}
            title={canWrite ? undefined : READ_ONLY}
            onClick={() => setDialog({ kind: 'signout' })}
          >
            Sign out everywhere
          </Button>
          {suspended ? null : (
            <Button
              variant="danger"
              disabled={!canWrite}
              title={canWrite ? undefined : READ_ONLY}
              onClick={() => setDialog({ kind: 'suspend' })}
            >
              Suspend
            </Button>
          )}
        </div>
      </div>

      <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,440px),1fr))] items-start gap-4">
        <Card aria-labelledby="u-teams" className="overflow-hidden">
          <CardHeader id="u-teams" title="Teams" />
          {detail.teams.length === 0 ? (
            <Empty>Not on a team.</Empty>
          ) : (
            <ul className="m-0 list-none p-0">
              {detail.teams.map((t) => {
                const pill = planPill(t);
                return (
                  <li key={t.slug}>
                    <Link
                      href={adminRoutes.team(t.slug)}
                      className={`${ROW} text-fg hover:bg-surface-100 hover:text-fg`}
                    >
                      <span
                        aria-hidden="true"
                        className="inline-grid size-7 shrink-0 place-items-center rounded-[7px] bg-surface-300 text-[12px] font-semibold"
                      >
                        {t.name.charAt(0).toUpperCase()}
                      </span>
                      <span className="flex min-w-0 flex-1 flex-col">
                        <span className="font-medium [overflow-wrap:anywhere]">{t.name}</span>
                        <span className="text-[12px] text-fg-muted">
                          {t.role === 'owner' ? 'Owner' : 'Member'} · joined{' '}
                          {formatDate(t.joined_at)}
                        </span>
                      </span>
                      <Pill tone={pill.tone}>{pill.label}</Pill>
                    </Link>
                  </li>
                );
              })}
            </ul>
          )}
        </Card>

        <Card aria-labelledby="u-sign" className="overflow-hidden">
          <CardHeader id="u-sign" title="Sign-in methods" />
          <ul className="m-0 list-none p-0">
            {detail.identities.map((i) => {
              const isEmail = i.provider === 'email';
              const label = isEmail ? 'Email code' : providerName(i.provider);
              const who = [i.email, i.subject ? `id ${i.subject}` : null]
                .filter(Boolean)
                .join(' · ');
              return (
                <li key={i.provider} className={ROW}>
                  <span className="flex-1">{label}</span>
                  <span
                    className={`min-w-0 text-[12px] text-fg-muted [overflow-wrap:anywhere] ${isEmail ? '' : 'font-mono'}`}
                  >
                    {who}
                  </span>
                  {isEmail ? null : (
                    <Button
                      size="sm"
                      aria-label={`Unlink ${label}`}
                      disabled={!canWrite}
                      title={canWrite ? undefined : READ_ONLY}
                      onClick={() => setDialog({ kind: 'unlink', provider: i.provider })}
                    >
                      Unlink
                    </Button>
                  )}
                </li>
              );
            })}
          </ul>
          {detail.identities.some((i) => i.provider !== 'email') ? (
            <p className="m-0 border-t border-hairline bg-raised px-4 py-2 text-[12px] text-fg-muted">
              Unlinking makes the next{' '}
              {providerName(detail.identities.find((i) => i.provider !== 'email')?.provider ?? '')}{' '}
              sign-in create a new link by email again.
            </p>
          ) : null}
        </Card>

        <Card aria-labelledby="u-dev" className="overflow-hidden">
          <CardHeader id="u-dev" title="Devices">
            <Button
              size="sm"
              disabled={!canWrite || detail.sessions.length === 0}
              title={canWrite ? undefined : READ_ONLY}
              onClick={() => setDialog({ kind: 'signout' })}
            >
              Sign out all
            </Button>
          </CardHeader>
          {detail.sessions.length === 0 ? (
            <Empty>No signed-in devices.</Empty>
          ) : (
            <ul className="m-0 list-none p-0">
              {detail.sessions.map((s) => {
                const kind = KIND[s.kind] ?? {
                  name: providerName(s.kind),
                  fallback: providerName(s.kind),
                };
                const label = s.label || kind.fallback;
                return (
                  <li key={s.id} className={ROW}>
                    <span className="flex min-w-0 flex-1 flex-col">
                      <span className="[overflow-wrap:anywhere]">{label}</span>
                      <span className="text-[12px] text-fg-muted">
                        {kind.name} · last used {relativeTimeInline(s.last_used_at, clock)}
                      </span>
                    </span>
                    <Button
                      size="sm"
                      aria-label={`Revoke ${label}`}
                      disabled={!canWrite}
                      loading={busy === s.id}
                      title={canWrite ? undefined : READ_ONLY}
                      onClick={() => revoke(s.id, label)}
                    >
                      Revoke
                    </Button>
                  </li>
                );
              })}
            </ul>
          )}
        </Card>

        <Card aria-labelledby="u-shared" className="overflow-hidden">
          <CardHeader id="u-shared" title={`Shared with ${first}`}>
            <span className="text-[12px] text-fg-muted">Collections from other teams</span>
          </CardHeader>
          {detail.shared.length === 0 ? (
            <Empty>Nothing is shared with {first}.</Empty>
          ) : (
            <ul className="m-0 list-none p-0">
              {detail.shared.map((c) => (
                <li key={`${c.team_slug}/${c.collection}`} className={ROW}>
                  <span className="flex-1 [overflow-wrap:anywhere]">{c.name}</span>
                  <span className="text-right text-[12px] text-fg-muted">
                    {c.team} · {c.access === 'public' ? 'public link' : 'access list'}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Card>

        <Card aria-labelledby="u-ev" className="col-span-full overflow-hidden">
          <CardHeader id="u-ev" title="Recent events">
            <Link href={adminRoutes.activity} className="text-[12px]">
              Admin actions on {first}
            </Link>
          </CardHeader>
          {detail.events.length === 0 ? (
            <Empty>No admin actions on {first} yet.</Empty>
          ) : (
            <ul className="m-0 list-none p-0">
              {detail.events.map((e) => (
                <li
                  key={e.id}
                  className="flex min-h-10 flex-wrap items-center gap-x-3 gap-y-0.5 border-b border-hairline px-4 py-1.5 last:border-b-0"
                >
                  <span className="w-[110px] shrink-0 font-mono text-[12px] text-fg-subtle">
                    {eventTime(e.at)}
                  </span>
                  <span className="min-w-[200px] flex-1 text-fg-muted [overflow-wrap:anywhere]">
                    {eventLine(e, name)}
                  </span>
                  {e.ip ? (
                    <span className="font-mono text-[12px] text-fg-subtle">{e.ip}</span>
                  ) : null}
                </li>
              ))}
            </ul>
          )}
        </Card>

        <Card aria-labelledby="u-del" className="col-span-full border-danger-border">
          <div className="flex flex-wrap items-center gap-4 p-4">
            <div className="flex min-w-[260px] flex-1 flex-col gap-1">
              <h2 id="u-del" className="m-0 text-[14px] font-semibold">
                Delete user
              </h2>
              <span className="text-fg-muted">
                Removes the account, sign-ins, devices and access-list matches. Teams stay with
                their other owners.
              </span>
            </div>
            <Button
              variant="danger"
              disabled={!canWrite}
              title={canWrite ? undefined : READ_ONLY}
              onClick={() => {
                setBlocked(null);
                setDialog({ kind: 'delete' });
              }}
            >
              Delete {first}…
            </Button>
          </div>
          {blocked ? (
            <div className="px-4 pb-4">
              <Notice tone="warn" role="alert">
                {blocked}
              </Notice>
            </div>
          ) : null}
        </Card>
      </div>

      <SuspendDialog
        open={dialog?.kind === 'suspend'}
        onClose={close}
        title={`Suspend ${name}?`}
        description={`${first} can’t sign in, and every device is signed out now. Teams, tokens and collections keep working, so CI isn’t broken.`}
        confirmLabel="Suspend user"
        notifyLabel={`Email ${first} that the account is suspended`}
        onSubmit={(reason, notify) => actions.suspend(user.id, reason, notify)}
        onDone={() => setProblem(null)}
      />

      <ConfirmDialog
        open={dialog?.kind === 'unsuspend'}
        onClose={close}
        title={`Lift the suspension on ${name}?`}
        description={`${first} can sign in again. Their teams and collections were never touched.`}
        confirmLabel="Lift suspension"
        pendingLabel="Lifting…"
        variant="primary"
        onConfirm={() => confirm(actions.unsuspend(user.id), `Lifted the suspension on ${name}.`)}
      />

      <ConfirmDialog
        open={dialog?.kind === 'signout'}
        onClose={close}
        title={`Sign out ${name} everywhere?`}
        description="Every device they’re signed in on stops working now. They can sign in again."
        confirmLabel="Sign out everywhere"
        pendingLabel="Signing out…"
        onConfirm={() => confirm(actions.signOut(user.id), `Signed out ${name} everywhere.`)}
      />

      <ConfirmDialog
        open={dialog?.kind === 'unlink'}
        onClose={close}
        title={`Unlink ${dialog?.kind === 'unlink' ? providerName(dialog.provider) : ''} from ${name}?`}
        description={`They can still sign in with an email code. The next ${dialog?.kind === 'unlink' ? providerName(dialog.provider) : ''} sign-in creates a new link by email again.`}
        confirmLabel="Unlink"
        pendingLabel="Unlinking…"
        onConfirm={() => {
          if (dialog?.kind !== 'unlink') {
            return;
          }
          const provider = dialog.provider;
          return confirm(
            actions.unlink(user.id, provider),
            `Unlinked ${providerName(provider)} from ${name}.`,
          );
        }}
      />

      <ConfirmDialog
        open={dialog?.kind === 'delete'}
        onClose={close}
        title={`Delete ${name}?`}
        description="Removes the account, sign-ins, devices and access-list matches. Teams stay with their other owners. This can’t be undone."
        confirmLabel="Delete user"
        pendingLabel="Deleting…"
        confirmText={user.email}
        onConfirm={remove}
      />

      <EmailUserDialog
        open={dialog?.kind === 'email'}
        onClose={close}
        name={name}
        onSubmit={(subject, body) => actions.email(user.id, subject, body)}
        onDone={() => {
          setProblem(null);
          setNote(`Email queued for ${user.email}.`);
        }}
      />
    </>
  );
}
