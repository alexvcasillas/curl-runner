'use client';

import { useEffect, useId, useRef, useState } from 'react';
import { cx } from '@/lib/cx';
import { absoluteTime } from '@/lib/time';
import type { AccessMode, Plan, Share } from '@/lib/types';
import { Avatar } from './Avatar';
import { Button } from './Button';
import { Card, CardBody, CardFooter, CardTitle } from './Card';
import { Checkbox } from './Checkbox';
import { ConfirmDialog } from './ConfirmDialog';
import { Field } from './Field';
import { LockIcon } from './icons';
import { Notice } from './Notice';
import { Pill } from './Pill';
import { RadioCard } from './RadioCard';
import { RestrictedUpsell } from './RestrictedUpsell';
import { Select } from './Select';

type Result = { ok: true; share: Share } | { ok: false; message: string };
type Patch = { access?: AccessMode; expires_in_days?: number | null; reset_link?: boolean };

const DAY = 86_400_000;

type Expiry = 'never' | '7' | '30' | 'custom';

/** The select's value for an expiry the server holds. Anything else is `custom`. */
export function expiryChoice(expiresAt: string | null, now: string): Expiry {
  if (!expiresAt) {
    return 'never';
  }
  const days = (new Date(expiresAt).getTime() - new Date(now).getTime()) / DAY;
  if (Math.abs(days - 7) <= 1) {
    return '7';
  }
  if (Math.abs(days - 30) <= 1) {
    return '30';
  }
  return 'custom';
}

const MODES: { id: AccessMode; title: string }[] = [
  { id: 'off', title: 'Off' },
  { id: 'public', title: 'Public' },
  { id: 'restricted', title: 'Restricted' },
];

function describe(id: AccessMode, teamName: string): string {
  return id === 'off'
    ? `Only members of ${teamName}.`
    : id === 'public'
      ? 'Anyone with the link. No account needed.'
      : 'Only people on the list, after they sign in.';
}

function linkHelp(access: AccessMode, name: string): string {
  return access === 'off'
    ? 'Turn on Public or Restricted to share it. The link is kept, so it works again when you do.'
    : access === 'public'
      ? `Anyone who has it can open ${name} and get every new version.`
      : `People on the list sign in once, then open ${name} and get every new version.`;
}

function kindLabel(entry: Share['entries'][number]): string {
  return entry.kind === 'domain'
    ? `Domain · anyone at ${entry.entry.replace(/^\*@/, '')}`
    : 'Email';
}

/** Access mode, the share link and the access list for one collection. */
export function SharePanel({
  team,
  teamName,
  name,
  plan,
  billing,
  share: initial,
  now,
  onPatch,
  onAdd,
  onRemove,
}: {
  team: string;
  teamName: string;
  name: string;
  plan: Plan;
  /** Stripe is configured on this server. */
  billing: boolean;
  share: Share;
  /** The server's clock at render, so the expiry choice matches on both sides. */
  now: string;
  onPatch: (patch: Patch) => Promise<Result>;
  onAdd: (entry: string, notify: boolean) => Promise<Result>;
  onRemove: (entry: string) => Promise<Result>;
}) {
  const [share, setShare] = useState(initial);
  const [expiry, setExpiry] = useState<Expiry>(expiryChoice(initial.expires_at, now));
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmRestrict, setConfirmRestrict] = useState(false);
  const [confirmReset, setConfirmReset] = useState(false);
  const [copy, setCopy] = useState<'idle' | 'copied' | 'failed'>('idle');
  const [entry, setEntry] = useState('');
  const [notify, setNotify] = useState(true);
  const [adding, setAdding] = useState(false);
  const [addError, setAddError] = useState<string | null>(null);
  const [removing, setRemoving] = useState<Share['entries'][number] | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const removeButtons = useRef(new Map<string, HTMLButtonElement>());
  const linkId = useId();
  const peopleId = useId();
  const whoId = useId();
  const linkTitleId = useId();

  useEffect(() => () => clearTimeout(timer.current), []);

  const { access } = share;
  const off = access === 'off';
  const free = plan === 'free';

  /** Runs a PATCH. Returns whether it worked. */
  async function patch(body: Patch): Promise<boolean> {
    setBusy(true);
    setError(null);
    try {
      const result = await onPatch(body);
      if (result.ok) {
        setShare(result.share);
        setExpiry(
          'expires_in_days' in body
            ? body.expires_in_days === null
              ? 'never'
              : (String(body.expires_in_days) as Expiry)
            : expiryChoice(result.share.expires_at, new Date().toISOString()),
        );
        return true;
      }
      setError(result.message);
    } catch {
      setError('That didn’t work. Try again.');
    } finally {
      setBusy(false);
    }
    return false;
  }

  function pick(mode: AccessMode) {
    if (mode === access) {
      return;
    }
    setConfirmReset(false);
    if (mode === 'restricted' && access === 'public') {
      setError(null);
      setConfirmRestrict(true);
      return;
    }
    setConfirmRestrict(false);
    void patch({ access: mode });
  }

  async function makeRestricted() {
    if (await patch({ access: 'restricted' })) {
      setConfirmRestrict(false);
    }
  }

  async function reset() {
    if (await patch({ reset_link: true })) {
      setConfirmReset(false);
    }
  }

  async function copyLink() {
    clearTimeout(timer.current);
    try {
      await navigator.clipboard.writeText(share.link);
      setCopy('copied');
    } catch {
      setCopy('failed');
    }
    timer.current = setTimeout(() => setCopy('idle'), 2000);
  }

  async function add(e: React.FormEvent) {
    e.preventDefault();
    const value = entry.trim();
    if (!value || adding) {
      return;
    }
    setAdding(true);
    setAddError(null);
    try {
      const result = await onAdd(value, notify);
      if (result.ok) {
        setShare(result.share);
        setEntry('');
      } else {
        setAddError(result.message);
      }
    } catch {
      setAddError('That didn’t work. Try again.');
    } finally {
      setAdding(false);
    }
  }

  async function remove() {
    if (!removing) {
      return;
    }
    const result = await onRemove(removing.entry);
    if (!result.ok) {
      throw new Error(result.message);
    }
    setShare(result.share);
    setRemoving(null);
  }

  const covered =
    removing?.kind === 'email'
      ? share.entries.find(
          (e) => e.kind === 'domain' && removing.entry.endsWith(e.entry.replace(/^\*/, '')),
        )
      : undefined;

  return (
    <div className="flex min-w-0 flex-col gap-5">
      <Card aria-labelledby={whoId}>
        <CardBody className="gap-3.5">
          <div>
            <CardTitle id={whoId}>Who can open it</CardTitle>
            <p className="m-0 text-fg-muted leading-normal">
              Members of {teamName} can always open it. This decides who else can.
            </p>
          </div>
          <fieldset className="m-0 flex min-w-0 flex-col gap-2 border-0 p-0" disabled={busy}>
            <legend className="sr-only">Who can open it</legend>
            {MODES.map(({ id, title }) => {
              const locked = free && id === 'restricted';
              return (
                <RadioCard
                  key={id}
                  name="access"
                  value={id}
                  checked={access === id}
                  disabled={locked && access !== id}
                  onChange={() => pick(id)}
                  title={
                    <span className="flex items-center gap-2">
                      {title}
                      {locked ? <Pill size="sm">Team plan</Pill> : null}
                    </span>
                  }
                  description={describe(id, teamName)}
                />
              );
            })}
          </fieldset>
          {free ? <RestrictedUpsell team={team} teamName={teamName} billing={billing} /> : null}
          {confirmRestrict ? (
            <Notice tone="warn" role="alert">
              <div className="flex flex-wrap items-center gap-3">
                <span className="min-w-[240px] flex-1">
                  The link will need sign-in. Anyone already using it who isn’t on the list stops
                  getting updates. Add them before you switch.
                </span>
                <span className="flex gap-2">
                  <Button size="sm" onClick={() => setConfirmRestrict(false)} disabled={busy}>
                    Keep public
                  </Button>
                  <Button size="sm" variant="primary" onClick={makeRestricted} loading={busy}>
                    {busy ? 'Switching…' : 'Make restricted'}
                  </Button>
                </span>
              </div>
            </Notice>
          ) : null}
          {error ? (
            <Notice tone="err" role="alert">
              {error}
            </Notice>
          ) : null}
        </CardBody>
      </Card>

      <Card aria-labelledby={linkTitleId}>
        <CardBody className="gap-3.5">
          <div>
            <CardTitle id={linkTitleId}>Share link</CardTitle>
            <p className="m-0 text-fg-muted leading-normal">{linkHelp(access, name)}</p>
          </div>
          <div className="flex flex-wrap gap-2">
            <label htmlFor={linkId} className="sr-only">
              Share link
            </label>
            <input
              id={linkId}
              readOnly
              value={share.link}
              onFocus={(e) => e.currentTarget.select()}
              className={cx(
                'box-border h-8 min-w-[260px] flex-1 rounded-[6px] border border-control bg-bg px-2.5 font-mono text-[12px] outline-none focus-visible:border-blue max-sm:min-w-0',
                off ? 'text-fg-disabled' : 'text-fg',
              )}
            />
            <Button onClick={copyLink} disabled={off}>
              {copy === 'copied' ? 'Copied' : copy === 'failed' ? 'Couldn’t copy' : 'Copy link'}
            </Button>
          </div>
          <span role="status" className="sr-only">
            {copy === 'copied' ? 'Link copied' : copy === 'failed' ? 'Couldn’t copy the link.' : ''}
          </span>
          <div className="flex flex-wrap items-end gap-4">
            <Select
              label="Expires"
              value={expiry}
              disabled={busy}
              onChange={(e) => {
                const next = e.target.value as Expiry;
                if (next === 'custom') {
                  return;
                }
                void patch({ expires_in_days: next === 'never' ? null : Number(next) });
              }}
              options={[
                { value: 'never', label: 'Never' },
                { value: '7', label: 'In 7 days' },
                { value: '30', label: 'In 30 days' },
                ...(expiry === 'custom' && share.expires_at
                  ? [{ value: 'custom', label: `Until ${absoluteTime(share.expires_at)}` }]
                  : []),
              ]}
            />
            <span className="flex-1" />
            <Button
              size="sm"
              onClick={() => {
                setConfirmRestrict(false);
                setConfirmReset(true);
              }}
              disabled={off || busy}
            >
              Reset link
            </Button>
          </div>
          {share.expires_at ? (
            <p className="m-0 text-[12px] text-fg-subtle">
              {new Date(share.expires_at).getTime() < new Date(now).getTime()
                ? 'Expired'
                : 'Expires'}{' '}
              {absoluteTime(share.expires_at)}.
            </p>
          ) : null}
          {confirmReset ? (
            <Notice tone="err" role="alert">
              <div className="flex flex-wrap items-center gap-3">
                <span className="min-w-[240px] flex-1">
                  The old link stops working right away, for everyone. People who already added{' '}
                  {name} need the new link to get updates.
                </span>
                <span className="flex gap-2">
                  <Button size="sm" onClick={() => setConfirmReset(false)} disabled={busy}>
                    Cancel
                  </Button>
                  <Button size="sm" variant="danger" onClick={reset} loading={busy}>
                    {busy ? 'Resetting…' : 'Reset link'}
                  </Button>
                </span>
              </div>
            </Notice>
          ) : null}
        </CardBody>
        <CardFooter>
          <LockIcon size={13} />
          <span className="min-w-0 flex-1">
            Treat the link like a password. It isn’t indexed by search engines, and it opens in the
            studio or with <span className="font-mono text-fg">curl-runner pull</span>.
          </span>
        </CardFooter>
      </Card>

      {access === 'restricted' ? (
        <Card aria-labelledby={peopleId}>
          <CardBody className="gap-3.5">
            <div>
              <CardTitle id={peopleId}>People with access</CardTitle>
              <p className="m-0 text-fg-muted leading-normal">
                Add an email, or a whole domain like{' '}
                <span className="font-mono text-fg">*@customer.com</span>. They sign in with that
                email to open it.
              </p>
            </div>
            <form onSubmit={add} className="flex flex-col gap-3">
              <div className="flex flex-wrap items-end gap-2">
                <Field
                  label="Email or domain"
                  value={entry}
                  onChange={(e) => setEntry(e.target.value)}
                  placeholder="ada@customer.com or *@customer.com"
                  autoComplete="off"
                  autoCapitalize="off"
                  spellCheck={false}
                  error={addError}
                  className="min-w-[240px] flex-1 max-sm:min-w-0"
                />
                <Button
                  type="submit"
                  variant="primary"
                  loading={adding}
                  disabled={!entry.trim()}
                  className={addError ? 'mb-[22px]' : undefined}
                >
                  Add
                </Button>
              </div>
              <Checkbox
                checked={notify}
                onChange={(e) => setNotify(e.target.checked)}
                label="Email them the link. Domains never get an email."
              />
            </form>
          </CardBody>
          {share.entries.length === 0 ? (
            <p className="m-0 border-t border-hairline px-6 py-4 text-[12px] text-fg-muted max-sm:px-4">
              Nobody on the list yet.
            </p>
          ) : (
            <ul className="m-0 list-none p-0">
              {share.entries.map((e) => (
                <li
                  key={e.entry}
                  className="box-border flex min-h-14 flex-wrap items-center gap-3 border-t border-hairline px-6 py-2 max-sm:px-4"
                >
                  <Avatar
                    name={e.kind === 'domain' ? '@' : e.entry}
                    size={28}
                    className={e.kind === 'domain' ? 'font-mono' : undefined}
                  />
                  <span className="flex min-w-[180px] flex-1 flex-col max-sm:min-w-0">
                    <span className="font-mono text-[12px] [overflow-wrap:anywhere]">
                      {e.entry}
                    </span>
                    <span className="text-[12px] text-fg-subtle">{kindLabel(e)}</span>
                  </span>
                  <Pill tone={e.opened ? 'ok' : 'neutral'}>
                    {e.opened ? 'Opened' : 'Invited · not opened'}
                  </Pill>
                  <Button
                    size="sm"
                    variant="ghost"
                    aria-label={`Remove ${e.entry}`}
                    ref={(el) => {
                      if (el) {
                        removeButtons.current.set(e.entry, el);
                      } else {
                        removeButtons.current.delete(e.entry);
                      }
                    }}
                    onClick={() => setRemoving(e)}
                  >
                    Remove
                  </Button>
                </li>
              ))}
            </ul>
          )}
          <CardFooter>
            Removing someone stops their next update. Their local copy stays on their machine.
          </CardFooter>
        </Card>
      ) : null}

      <ConfirmDialog
        open={removing !== null}
        onClose={() => setRemoving(null)}
        onConfirm={remove}
        title={`Remove ${removing?.entry ?? ''}?`}
        description={`They can’t open ${name} or get new versions after this.`}
        confirmLabel="Remove access"
        pendingLabel="Removing…"
      >
        {removing ? (
          <div className="flex items-center gap-3">
            <Avatar name={removing.kind === 'domain' ? '@' : removing.entry} size={28} />
            <span className="flex min-w-0 flex-1 flex-col">
              <span className="font-mono text-[12px] [overflow-wrap:anywhere]">
                {removing.entry}
              </span>
              <span className="text-[12px] text-fg-muted">
                {removing.opened ? 'Opened' : 'Invited · not opened'}
              </span>
            </span>
          </div>
        ) : null}
        <p className="m-0 text-[12px] text-fg-subtle">
          Their local copy stays on their machine.
          {covered
            ? ` They still have access through ${covered.entry}, which is on the list too.`
            : ''}
        </p>
      </ConfirmDialog>
    </div>
  );
}
