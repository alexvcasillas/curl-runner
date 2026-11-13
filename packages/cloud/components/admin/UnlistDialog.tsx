'use client';

import { useEffect, useId, useState } from 'react';
import type { ActionResult } from '@/lib/action-result';
import type { CollectionRow } from '@/lib/admin-collections';
import { Button } from '../Button';
import { Dialog } from '../Dialog';
import { inputClasses } from '../field-styles';
import { Notice } from '../Notice';

const MAX_REASON = 500;

/** Takes a collection out of the public gallery. The share link keeps working. */
export function UnlistDialog({
  collection,
  onClose,
  onSubmit,
  onDone,
}: {
  /** The collection to unlist. Null keeps the dialog closed. */
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
      setError('Add a reason.');
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
      title={collection ? `Unlist ${collection.name}?` : ''}
      description="It leaves the public gallery. The share link keeps working. The team can list it again."
      footer={
        <>
          <Button onClick={onClose} disabled={pending}>
            Cancel
          </Button>
          <Button type="submit" form={formId} variant="danger-fill" loading={pending}>
            {pending ? 'Unlisting…' : 'Unlist'}
          </Button>
        </>
      }
    >
      <form id={formId} onSubmit={submit} noValidate className="flex flex-col gap-3.5">
        <div className="flex flex-col gap-1.5 text-[12px]">
          <label htmlFor={reasonId} className="text-fg-muted">
            Reason
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
