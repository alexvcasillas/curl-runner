'use client';

import { useState } from 'react';
import type { ActionResult } from '@/lib/action-result';
import { routes } from '@/lib/routes';
import { Button } from './Button';
import { ButtonLink } from './ButtonLink';
import { Card, CardBody, CardFooter, CardTitle } from './Card';
import { ConfirmDialog } from './ConfirmDialog';
import { inputClasses } from './field-styles';
import { LeaveTeamDialog } from './LeaveTeamDialog';
import { PageHeader } from './PageHeader';

const plural = (n: number, one: string) => `${n} ${n === 1 ? one : `${one}s`}`;

/** Rename, the slug, Leave and Delete. Owners change things; anyone can leave. */
export function TeamSettings({
  team,
  name,
  owner,
  me,
  soleOwner,
  slugHost,
  collections,
  tokens,
  subscription,
  onRename,
  onDelete,
  onLeave,
}: {
  team: string;
  name: string;
  owner: boolean;
  me: { id: string; email: string };
  soleOwner: boolean;
  /** The dashboard's host, for the slug line. */
  slugHost: string;
  collections: number;
  tokens: number;
  /** A Stripe subscription backs the plan, so a delete cancels it. */
  subscription: boolean;
  onRename: (name: string) => Promise<ActionResult>;
  onDelete: () => Promise<ActionResult>;
  onLeave: () => Promise<ActionResult>;
}) {
  const [draft, setDraft] = useState(name);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const [leaving, setLeaving] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const clean = draft.trim();
  const changed = clean !== '' && clean !== name;

  async function save(e: React.FormEvent) {
    e.preventDefault();
    if (!changed || saving) {
      return;
    }
    setSaving(true);
    setSaved(false);
    setProblem(null);
    try {
      const result = await onRename(clean);
      if (result.ok) {
        setSaved(true);
      } else {
        setProblem(result.message);
      }
    } catch {
      setProblem('That didn’t work. Try again.');
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <PageHeader title="Settings">
        {owner
          ? `Everything about ${name} itself. Plans and seats are under Billing.`
          : 'Only owners can change these.'}
      </PageHeader>

      <Card aria-label="Rename team">
        <form onSubmit={save} noValidate>
          <CardBody>
            <CardTitle>Team name</CardTitle>
            <p className="m-0 leading-normal text-fg-muted">
              Shown to members, and to customers on share pages and emails.
            </p>
            <input
              aria-label="Team name"
              value={draft}
              disabled={!owner}
              maxLength={100}
              autoComplete="off"
              onChange={(e) => {
                setDraft(e.target.value);
                setSaved(false);
                setProblem(null);
              }}
              className={inputClasses(Boolean(problem))}
            />
          </CardBody>
          <CardFooter>
            {problem ? (
              <span role="alert" className="flex-1 text-danger">
                {problem}
              </span>
            ) : saved ? (
              <span role="status" className="flex-1 text-success">
                Saved.
              </span>
            ) : (
              <span className="flex-1">
                {owner ? 'Up to 100 characters.' : 'Ask an owner to rename the team.'}
              </span>
            )}
            {owner ? (
              <Button type="submit" variant="primary" disabled={!changed} loading={saving}>
                {saving ? 'Saving…' : 'Save'}
              </Button>
            ) : null}
          </CardFooter>
        </form>
      </Card>

      <Card aria-labelledby="slug-title">
        <CardBody>
          <CardTitle id="slug-title">Team slug</CardTitle>
          <p className="m-0 leading-normal text-fg-muted">
            Used by CI in <code>team: {team}</code> and in dashboard links. It can’t change, so
            publishing setups never break.
          </p>
          <span className="font-mono text-[12px] break-all text-fg-muted">
            {slugHost}/t/<span className="text-fg">{team}</span>
          </span>
        </CardBody>
      </Card>

      <Card aria-labelledby="leave-title">
        <CardBody className="flex-row flex-wrap items-center">
          <div className="flex min-w-[240px] flex-1 flex-col gap-1">
            <CardTitle id="leave-title" className="text-[14px] leading-5">
              Leave {name}
            </CardTitle>
            <p className="m-0 leading-normal text-fg-muted">
              {soleOwner
                ? 'You’re the only owner. Make someone else an owner before you leave.'
                : 'You lose access to its collections and tokens. An owner can invite you again.'}
            </p>
          </div>
          {soleOwner ? (
            <ButtonLink href={routes.teamTab(team, 'members')}>Go to members</ButtonLink>
          ) : (
            <Button onClick={() => setLeaving(true)}>Leave team</Button>
          )}
        </CardBody>
      </Card>

      {owner ? (
        <Card aria-labelledby="delete-title" className="border-danger-border">
          <CardBody className="flex-row flex-wrap items-center">
            <div className="flex min-w-[240px] flex-1 flex-col gap-1">
              <CardTitle id="delete-title" className="text-[14px] leading-5">
                Delete {name}
              </CardTitle>
              <p className="m-0 leading-normal text-fg-muted">
                Deletes {plural(collections, 'collection')} with every version, all share links,
                tokens and invites.{subscription ? ' Cancels the Team subscription today.' : ''}
              </p>
            </div>
            <Button variant="danger" onClick={() => setDeleting(true)}>
              Delete team
            </Button>
          </CardBody>
          <CardFooter className="border-danger-border">
            Only owners can delete a team. Customers keep their local copies.
          </CardFooter>
        </Card>
      ) : null}

      <LeaveTeamDialog
        open={leaving}
        onClose={() => setLeaving(false)}
        teamName={name}
        email={me.email}
        onLeave={onLeave}
      />

      <ConfirmDialog
        open={deleting}
        onClose={() => setDeleting(false)}
        title={`Delete the ${name} team`}
        description="This can’t be undone."
        notice={{
          tone: 'err',
          children: `Its ${plural(collections, 'collection')} and ${plural(tokens, 'token')} stop working. CI publishes fail with “not found”.${subscription ? ' The subscription is cancelled now, with no refund for the rest of the billing period.' : ''}`,
        }}
        confirmText={team}
        confirmLabel="Delete team"
        pendingLabel="Deleting…"
        onConfirm={async () => {
          const result = await onDelete();
          if (!result.ok) {
            throw new Error(result.message);
          }
        }}
      />
    </>
  );
}
