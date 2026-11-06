'use client';

import { useEffect, useId, useState } from 'react';
import type { ActionResult } from '@/lib/action-result';
import { Button } from '../Button';
import { Dialog } from '../Dialog';
import { Field } from '../Field';
import { inputClasses } from '../field-styles';
import { Notice } from '../Notice';

const MAX_SUBJECT = 200;
const MAX_BODY = 10_000;

/** A one-off message to one person. The body is Markdown, sent from the server's own address. */
export function EmailUserDialog({
  open,
  onClose,
  name,
  onSubmit,
  onDone,
}: {
  open: boolean;
  onClose: () => void;
  name: string;
  onSubmit: (subject: string, body: string) => Promise<ActionResult>;
  onDone: () => void;
}) {
  const [subject, setSubject] = useState('');
  const [body, setBody] = useState('');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const formId = useId();
  const bodyId = `${formId}-body`;

  useEffect(() => {
    if (open) {
      setSubject('');
      setBody('');
      setPending(false);
      setError(null);
    }
  }, [open]);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (pending) {
      return;
    }
    if (!subject.trim() || !body.trim()) {
      setError('Add a subject and a message.');
      return;
    }
    setPending(true);
    setError(null);
    try {
      const result = await onSubmit(subject, body);
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
      title={`Email ${name}`}
      description="Sent from the server’s own address. Logged in the activity log."
      footer={
        <>
          <Button onClick={onClose} disabled={pending}>
            Cancel
          </Button>
          <Button type="submit" form={formId} variant="primary" loading={pending}>
            {pending ? 'Sending…' : 'Send email'}
          </Button>
        </>
      }
    >
      <form id={formId} onSubmit={submit} noValidate className="flex flex-col gap-3.5">
        <Field
          label="Subject"
          value={subject}
          maxLength={MAX_SUBJECT}
          autoComplete="off"
          onChange={(e) => setSubject(e.target.value)}
        />
        <div className="flex flex-col gap-1.5 text-[12px]">
          <label htmlFor={bodyId} className="text-fg-muted">
            Message
          </label>
          <textarea
            id={bodyId}
            value={body}
            maxLength={MAX_BODY}
            rows={6}
            onChange={(e) => setBody(e.target.value)}
            className={inputClasses(false, 'h-36 resize-y py-2 leading-5')}
          />
          <span className="text-fg-subtle">
            Markdown works: **bold**, *italic*, [links](https://example.com) and “- ” lists.
          </span>
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
