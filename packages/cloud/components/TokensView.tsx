'use client';

import { useState } from 'react';
import type { CreateTokenResult } from '@/app/(app)/t/[team]/tokens/actions';
import type { ActionResult } from '@/lib/action-result';
import { formatDate } from '@/lib/time';
import { Button } from './Button';
import { Card } from './Card';
import { ConfirmDialog } from './ConfirmDialog';
import { CopyField } from './CopyField';
import { Dialog } from './Dialog';
import { EmptyState } from './EmptyState';
import { Field } from './Field';
import { PlusIcon } from './icons';
import { Notice } from './Notice';
import { PageHeader } from './PageHeader';
import { Pill } from './Pill';
import { Segmented } from './Segmented';
import { Table, TableCell, TableHead, TableHeader, TableRow } from './Table';

export type TokenRow = {
  id: string;
  name: string;
  /** The email of whoever made it, or null. */
  by: string | null;
  created: string;
  expires: string;
  /** Two weeks off or less: flagged. */
  expiresSoon: boolean;
  /** "2 hours ago", or null when it was never used. */
  lastUsed: string | null;
};

type Expiry = 'never' | '30' | '90' | '365';
const EXPIRY: { value: Expiry; label: string; days: number | null }[] = [
  { value: 'never', label: 'Never', days: null },
  { value: '30', label: '30 days', days: 30 },
  { value: '90', label: '90 days', days: 90 },
  { value: '365', label: '1 year', days: 365 },
];

// The Actions expression, written out. It is literal text, not a template.
// biome-ignore lint/suspicious/noTemplateCurlyInString: see above.
const SECRET_EXPR = '${{ secrets.CURL_RUNNER_TOKEN }}';

const DAY = 24 * 60 * 60 * 1000;

function expiryHint(expiry: Expiry): string {
  const days = EXPIRY.find((e) => e.value === expiry)?.days ?? null;
  return days === null
    ? 'It works until someone revokes it.'
    : `It stops working on ${formatDate(new Date(Date.now() + days * DAY).toISOString())}.`;
}

type Created = Extract<CreateTokenResult, { ok: true }>['token'];

/** The publish tokens: the list, Create token and Revoke. The new value shows once, from state only. */
export function TokensView({
  team,
  teamName,
  tokens,
  onCreate,
  onRevoke,
}: {
  team: string;
  teamName: string;
  tokens: TokenRow[];
  onCreate: (name: string, expiresInDays: number | null) => Promise<CreateTokenResult>;
  onRevoke: (id: string) => Promise<ActionResult>;
}) {
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState('github-actions');
  const [expiry, setExpiry] = useState<Expiry>('90');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // The `crt_` value. It lives here and nowhere else: not in a URL, a cookie or a cache.
  const [created, setCreated] = useState<Created | null>(null);
  const [revoking, setRevoking] = useState<TokenRow | null>(null);

  function openCreate() {
    setName('github-actions');
    setExpiry('90');
    setError(null);
    setCreating(true);
  }

  async function create(e: React.FormEvent) {
    e.preventDefault();
    if (pending) {
      return;
    }
    setPending(true);
    setError(null);
    try {
      const days = EXPIRY.find((x) => x.value === expiry)?.days ?? null;
      const result = await onCreate(name, days);
      if (result.ok) {
        setCreated(result.token);
        setCreating(false);
      } else {
        setError(result.message);
      }
    } catch {
      setError('That didn’t work. Try again.');
    } finally {
      setPending(false);
    }
  }

  const createButton = (
    <Button variant="primary" onClick={openCreate}>
      <PlusIcon size={14} />
      Create token
    </Button>
  );

  return (
    <>
      <PageHeader title="Publish tokens" actions={createButton}>
        CI uses these to publish to {teamName}. A token can publish to any of {teamName}’s
        collections and nothing else.
      </PageHeader>

      {tokens.length === 0 ? (
        <EmptyState
          title="No publish tokens yet"
          actions={
            <Button variant="primary" onClick={openCreate}>
              Create token
            </Button>
          }
        >
          Create one for each place that publishes, like GitHub Actions or a release script. You can
          revoke one without touching the others.
        </EmptyState>
      ) : (
        <Table
          label="Tokens"
          minWidth={820}
          footer="Tokens are stored hashed. If one leaks, revoke it and create a new one. Revoking takes effect on the next publish."
        >
          <TableHeader>
            <TableHead>Name</TableHead>
            <TableHead width={180}>Created by</TableHead>
            <TableHead width={110}>Created</TableHead>
            <TableHead width={140}>Expires</TableHead>
            <TableHead width={120}>Last used</TableHead>
            <TableHead width={72} />
          </TableHeader>
          {tokens.map((t) => (
            <TableRow key={t.id}>
              <TableCell className="flex flex-col">
                <span className="font-medium [overflow-wrap:anywhere]">{t.name}</span>
                <span className="font-mono text-[12px] text-fg-subtle">crt_••••</span>
              </TableCell>
              <TableCell width={180} className="truncate text-fg-muted">
                {t.by ?? 'Unknown'}
              </TableCell>
              <TableCell width={110} className="text-fg-muted">
                {t.created}
              </TableCell>
              <TableCell width={140}>
                {t.expiresSoon ? <Pill tone="warn">{t.expires}</Pill> : t.expires}
              </TableCell>
              <TableCell width={120} className="text-fg-muted">
                {t.lastUsed ?? 'Never used'}
              </TableCell>
              <TableCell width={72} className="flex justify-end">
                <Button size="sm" aria-label={`Revoke ${t.name}`} onClick={() => setRevoking(t)}>
                  Revoke
                </Button>
              </TableCell>
            </TableRow>
          ))}
        </Table>
      )}

      <Card aria-label="Publish from your machine">
        <div className="px-6 py-5 max-sm:px-4">
          <h2 className="m-0 text-[14px] leading-5 font-semibold">
            Publishing from your own machine?
          </h2>
          <p className="mt-1 mb-0 leading-normal text-fg-muted">
            You don’t need a token. Run <code>curl-runner login</code>, then{' '}
            <code>curl-runner publish</code> publishes as you.
          </p>
        </div>
      </Card>

      <Dialog
        open={creating}
        onClose={() => !pending && setCreating(false)}
        title="Create a publish token"
        description="Name it after where it runs, so you know which one to revoke."
        footer={
          <>
            <Button onClick={() => setCreating(false)} disabled={pending}>
              Cancel
            </Button>
            <Button type="submit" form="create-token" variant="primary" loading={pending}>
              {pending ? 'Creating…' : 'Create token'}
            </Button>
          </>
        }
      >
        {creating ? (
          <form id="create-token" onSubmit={create} className="flex flex-col gap-4">
            <Field
              label="Name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              maxLength={100}
              autoComplete="off"
              spellCheck={false}
            />
            <div className="flex flex-col gap-1.5 text-[12px]">
              <span className="text-fg-muted">Expires</span>
              <Segmented
                label="Expires"
                options={EXPIRY.map(({ value, label }) => ({ value, label }))}
                value={expiry}
                onChange={setExpiry}
                className="self-start"
              />
              <span className="text-fg-subtle">{expiryHint(expiry)}</span>
            </div>
            {error ? (
              <Notice tone="err" role="alert">
                {error}
              </Notice>
            ) : null}
          </form>
        ) : null}
      </Dialog>

      <Dialog
        open={created !== null}
        onClose={() => setCreated(null)}
        title="Token created"
        description="Copy it now. You won’t see it again."
        className="max-w-[620px]"
        footer={
          <Button variant="primary" onClick={() => setCreated(null)}>
            Done
          </Button>
        }
      >
        {created ? (
          <div className="flex flex-col gap-4">
            <CopyField value={created.value} label="token" variant="secret" />
            <div className="flex flex-col gap-2">
              <span className="text-[12px] text-fg-muted">
                Save it as <code>CURL_RUNNER_TOKEN</code> in your repository secrets, then add this
                step:
              </span>
              <pre className="m-0 overflow-x-auto rounded-[6px] border border-hairline bg-bg px-3.5 py-3 font-mono text-[12px] leading-[18px] whitespace-pre text-fg">
                <span>- uses:</span>{' '}
                <span className="text-success">alexvcasillas/curl-runner/actions/publish@main</span>
                {'\n  '}
                <span>with:</span>
                {'\n    '}
                <span>token:</span> <span className="text-success">{SECRET_EXPR}</span>
                {'\n    '}
                <span>team:</span> <span className="text-success">{team}</span>
              </pre>
            </div>
            <Notice tone="warn">
              Anyone with this token can publish to {teamName}. Keep it in CI secrets, never in the
              repo.
            </Notice>
          </div>
        ) : null}
      </Dialog>

      <ConfirmDialog
        open={revoking !== null}
        onClose={() => setRevoking(null)}
        title={`Revoke ${revoking?.name ?? ''}?`}
        description="The next publish that uses it fails with “Sign in or use a valid token.”"
        notice={{
          tone: 'warn',
          children: revoking?.lastUsed ? (
            <>
              Last used {revoking.lastUsed}. Create a new token and update{' '}
              <code>CURL_RUNNER_TOKEN</code> first, so CI keeps publishing.
            </>
          ) : (
            'It hasn’t been used, so no publish depends on it.'
          ),
        }}
        confirmLabel="Revoke token"
        pendingLabel="Revoking…"
        onConfirm={async () => {
          if (!revoking) {
            return;
          }
          const result = await onRevoke(revoking.id);
          if (!result.ok) {
            throw new Error(result.message);
          }
          setRevoking(null);
        }}
      />
    </>
  );
}
