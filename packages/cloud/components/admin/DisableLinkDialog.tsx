'use client';

import { useEffect, useId, useState } from 'react';
import type { ActionResult } from '@/lib/action-result';
import type { CollectionRow } from '@/lib/admin-collections';
import { plural } from '@/lib/admin-emails';
import { Button } from '../Button';
import { Dialog } from '../Dialog';
import { inputClasses } from '../field-styles';
import { Notice } from '../Notice';

const MAX_REASON = 500;

/** Takes a share link down. The reason goes to the team's owners by email. */
export function DisableLinkDialog({
  collection,
  onClose,
  onSubmit,
  onDone,
}: {
  /** The collection to take down. Null keeps the dialog closed. */
  collection: CollectionRow | null;
  onClose: () => void;
  onSubmit: (reason: string) => Promise<ActionResult>;
  /** Called once it worked, after the dialog closes. */
  onDone: (collection: CollectionRow) => void;
}) {
  const [reason, setReason] = useState('');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const formId = useId();
  const reasonId = `${formId}-reason`;
  const open = collection !== null;

  useEffect(() => {
    if (open) {
      setReason('');
      setPending(false);
      setError(null);
    }
  }, [open]);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (pending || !collection) {
      return;
    }
    if (!reason.trim()) {
      setError('Add a reason. The team’s owners will read it.');
      return;
    }
    setPending(true);
    setError(null);
    try {
      const result = await onSubmit(reason);
      if (result.ok) {
        onClose();
        onDone(collection);
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
      title={collection ? `Disable the share link of ${collection.name}?` : ''}
      description="Anyone opening it sees “This link no longer works”. The team keeps the collection and its versions, and can’t turn the link back on."
      footer={
        <>
          <Button onClick={onClose} disabled={pending}>
            Cancel
          </Button>
          <Button type="submit" form={formId} variant="danger-fill" loading={pending}>
            {pending ? 'Disabling…' : 'Disable link'}
          </Button>
        </>
      }
    >
      <form id={formId} onSubmit={submit} noValidate className="flex flex-col gap-3.5">
        {collection && collection.opens_30d > 0 ? (
          <Notice tone="warn">
            {plural(collection.opens_30d, 'signed-in open', 'signed-in opens')} in the last 30 days.
          </Notice>
        ) : null}
        <div className="flex flex-col gap-1.5 text-[12px]">
          <label htmlFor={reasonId} className="text-fg-muted">
            Reason, shown to the team’s owners
          </label>
          <input
            id={reasonId}
            value={reason}
            maxLength={MAX_REASON}
            autoComplete="off"
            onChange={(e) => setReason(e.target.value)}
            className={inputClasses(false)}
          />
        </div>
        {error ? (
          <Notice tone="err" role="alert">
            {error}
          </Notice>
        ) : null}
      </form>
    </Dialog>
  );
}
