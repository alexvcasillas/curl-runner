'use client';

import { useEffect, useId, useState } from 'react';
import type { ActionResult } from '@/lib/action-result';
import { Button } from '../Button';
import { Checkbox } from '../Checkbox';
import { Dialog } from '../Dialog';
import { inputClasses } from '../field-styles';
import { Notice } from '../Notice';

const MAX_REASON = 500;

export type SuspendDialogProps = {
  open: boolean;
  onClose: () => void;
  title: string;
  description: string;
  /** "Suspend user" or "Suspend 3 users". */
  confirmLabel: string;
  /** The notify box's label. Omit it (bulk) and the dialog has no such box. */
  notifyLabel?: string;
  /** Runs on confirm. A failure keeps the dialog open with the API's message. */
  onSubmit: (reason: string, notify: boolean) => Promise<ActionResult>;
  /** Called once it worked, after the dialog closes. */
  onDone: () => void;
};

/** Why, and whether to tell them. The reason goes to the activity log. */
export function SuspendDialog({
  open,
  onClose,
  title,
  description,
  confirmLabel,
  notifyLabel,
  onSubmit,
  onDone,
}: SuspendDialogProps) {
  const [reason, setReason] = useState('');
  const [notify, setNotify] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const formId = useId();

  useEffect(() => {
    if (open) {
      setReason('');
      setNotify(false);
      setPending(false);
      setError(null);
    }
  }, [open]);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (pending) {
      return;
    }
    setPending(true);
    setError(null);
    try {
      const result = await onSubmit(reason, notifyLabel ? notify : false);
      if (result.ok) {
        onClose();
        onDone();
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
      title={title}
      description={description}
      footer={
        <>
          <Button onClick={onClose} disabled={pending}>
            Cancel
          </Button>
          <Button type="submit" form={formId} variant="danger-fill" loading={pending}>
            {pending ? 'Suspending…' : confirmLabel}
          </Button>
        </>
      }
    >
      <form id={formId} onSubmit={submit} className="flex flex-col gap-3.5">
        <label className="flex flex-col gap-1.5 text-[12px] text-fg-muted">
          Reason, for the activity log
          <textarea
            value={reason}
            maxLength={MAX_REASON}
            rows={3}
            onChange={(e) => setReason(e.target.value)}
            className={inputClasses(false, 'h-[72px] resize-y py-2 leading-5')}
          />
        </label>
        {notifyLabel ? (
          <Checkbox
            label={notifyLabel}
            checked={notify}
            onChange={(e) => setNotify(e.target.checked)}
          />
        ) : null}
        {error ? (
          <Notice tone="err" role="alert">
            {error}
          </Notice>
        ) : null}
      </form>
    </Dialog>
  );
}
