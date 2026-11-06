'use client';

import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useEffect, useState } from 'react';
import { can, type Perm } from '@/lib/admin';
import { type Broadcast, broadcastHref, broadcastMeta } from '@/lib/admin-emails';
import { cx } from '@/lib/cx';
import { Button } from '../Button';
import { Card, CardBody, CardTitle } from '../Card';
import { EmptyState } from '../EmptyState';
import { Notice } from '../Notice';
import { BroadcastEditor } from './BroadcastEditor';
import type { EmailActions } from './email-actions';

/** The broadcasts tab: the list on the left, the open one's editor and preview beside it. */
export function BroadcastsView({
  items,
  selected,
  initialCount,
  now,
  permissions,
  actions,
}: {
  items: Broadcast[];
  selected: Broadcast | null;
  /** People the selected broadcast's audience reaches today. */
  initialCount: number | null;
  now: string;
  permissions: Perm[];
  actions: EmailActions;
}) {
  const router = useRouter();
  const clock = new Date(now);
  const canEmail = can(permissions, 'emails');
  const [fresh, setFresh] = useState<Record<string, Broadcast>>({});
  const [creating, setCreating] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);

  // A new render from the server is newer than anything held here.
  // biome-ignore lint/correctness/useExhaustiveDependencies: reset on new server data only
  useEffect(() => setFresh({}), [items, selected]);

  const latest = (b: Broadcast) => fresh[b.id] ?? b;
  const open = selected ? latest(selected) : null;

  async function create() {
    setCreating(true);
    setProblem(null);
    try {
      const r = await actions.create();
      if (r.ok) {
        router.push(broadcastHref(r.broadcast.id));
      } else {
        setProblem(r.message);
      }
    } catch {
      setProblem('That didn’t work. Try again.');
    } finally {
      setCreating(false);
    }
  }

  const newButton = (label: string, primary: boolean) => (
    <Button
      size="sm"
      variant={primary ? 'primary' : 'secondary'}
      disabled={!canEmail}
      loading={creating}
      title={canEmail ? undefined : 'You can read this page but not change it.'}
      onClick={create}
    >
      {label}
    </Button>
  );

  return (
    <div className="flex flex-wrap items-start gap-4">
      <Card aria-label="Broadcasts" className="max-w-[300px] flex-[1_1_260px]">
        <div className="flex min-h-11 flex-wrap items-center gap-2 border-b border-hairline px-4">
          <CardTitle className="mr-auto text-[14px]">Broadcasts</CardTitle>
          {newButton('New', true)}
        </div>
        {items.length === 0 ? (
          <p className="m-0 px-4 py-4 text-[12px] text-fg-subtle">Nothing written yet.</p>
        ) : (
          <ul className="m-0 list-none p-0">
            {items.map((b) => {
              const row = latest(b);
              const on = selected?.id === b.id;
              return (
                <li key={b.id} className="border-b border-hairline last:border-b-0">
                  <Link
                    href={broadcastHref(b.id)}
                    aria-current={on ? 'page' : undefined}
                    className={cx(
                      'flex flex-col gap-0.5 px-4 py-2.5 text-fg no-underline hover:bg-surface-100 hover:text-fg',
                      on && 'bg-surface-100',
                    )}
                  >
                    <span className="font-medium [overflow-wrap:anywhere]">
                      {row.subject || 'Untitled draft'}
                    </span>
                    <span className="text-[12px] text-fg-muted">{broadcastMeta(row, clock)}</span>
                  </Link>
                </li>
              );
            })}
          </ul>
        )}
      </Card>

      <div className="flex min-w-0 flex-[999_1_520px] flex-col gap-4">
        {problem ? (
          <Notice tone="err" role="alert">
            {problem}
          </Notice>
        ) : null}
        {open ? (
          <BroadcastEditor
            key={open.id}
            broadcast={open}
            initialCount={initialCount}
            canEdit={canEmail}
            now={now}
            actions={actions}
            onUpdate={(b) => setFresh((f) => ({ ...f, [b.id]: b }))}
          />
        ) : (
          <Card aria-label="Compose">
            <CardBody>
              <EmptyState title="No broadcasts yet" actions={newButton('Write a broadcast', true)}>
                Write to the people on this server. You preview it as any user, send yourself a
                test, then send it or schedule it.
              </EmptyState>
            </CardBody>
          </Card>
        )}
      </div>
    </div>
  );
}
