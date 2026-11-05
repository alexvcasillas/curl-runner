'use client';

import { useEffect, useId, useState } from 'react';
import { Button } from './Button';
import type { ButtonVariant } from './button-styles';
import { Dialog } from './Dialog';
import { inputClasses } from './field-styles';
import { Notice } from './Notice';
import type { Tone } from './tones';

export type ConfirmDialogProps = {
  open: boolean;
  onClose: () => void;
  /** Runs on confirm. While its promise is pending the button shows `pendingLabel`. */
  onConfirm: () => void | Promise<void>;
  title: React.ReactNode;
  description?: React.ReactNode;
  /** A callout about the people affected. */
  notice?: { tone: Tone; children: React.ReactNode };
  children?: React.ReactNode;
  /** Says what happens: "Revoke token". */
  confirmLabel: string;
  /** The verb in progress: "Revoking…". */
  pendingLabel?: string;
  cancelLabel?: string;
  /** Red outline by default. Typing a slug to confirm uses the red fill. */
  variant?: Extract<ButtonVariant, 'danger' | 'danger-fill' | 'primary'>;
  /** Asks the person to type this (a slug) before the button unlocks. */
  confirmText?: string;
  /** An error to show, such as the API's message. */
  error?: string | null;
};

/** Names the thing it changes and what happens to the people affected. Safe choice first. */
export function ConfirmDialog({
  open,
  onClose,
  onConfirm,
  title,
  description,
  notice,
  children,
  confirmLabel,
  pendingLabel,
  cancelLabel = 'Cancel',
  variant,
  confirmText,
  error,
}: ConfirmDialogProps) {
  const [typed, setTyped] = useState('');
  const [pending, setPending] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const inputId = useId();
  const formId = `${inputId}-form`;

  useEffect(() => {
    if (open) {
      setTyped('');
      setFailure(null);
      setPending(false);
    }
  }, [open]);

  const locked = confirmText !== undefined && typed !== confirmText;
  const shownError = error ?? failure;

  async function submit(e?: React.FormEvent) {
    e?.preventDefault();
    if (locked || pending) {
      return;
    }
    setPending(true);
    setFailure(null);
    try {
      await onConfirm();
    } catch (err) {
      setFailure(
        err instanceof Error && err.message ? err.message : 'That didn’t work. Try again.',
      );
    } finally {
      setPending(false);
    }
  }

  const hasBody = Boolean(notice || children || confirmText !== undefined || shownError);

  return (
    <Dialog
      open={open}
      onClose={() => {
        if (!pending) {
          onClose();
        }
      }}
      role="alertdialog"
      title={title}
      description={description}
      footer={
        <>
          <Button onClick={onClose} disabled={pending}>
            {cancelLabel}
          </Button>
          <Button
            type={hasBody ? 'submit' : 'button'}
            form={hasBody ? formId : undefined}
            onClick={hasBody ? undefined : () => submit()}
            variant={variant ?? (confirmText !== undefined ? 'danger-fill' : 'danger')}
            disabled={locked}
            loading={pending}
          >
            {pending && pendingLabel ? pendingLabel : confirmLabel}
          </Button>
        </>
      }
    >
      {hasBody ? (
        <form id={formId} onSubmit={submit} className="flex flex-col gap-3.5">
          {notice ? <Notice tone={notice.tone}>{notice.children}</Notice> : null}
          {children}
          {confirmText !== undefined ? (
            <div className="flex flex-col gap-1.5">
              <label htmlFor={inputId} className="text-[12px] text-fg-muted">
                Type <code>{confirmText}</code> to confirm
              </label>
              <input
                id={inputId}
                value={typed}
                onChange={(e) => setTyped(e.target.value)}
                autoComplete="off"
                autoCapitalize="off"
                spellCheck={false}
                className={inputClasses(false, 'font-mono text-[12px]')}
              />
            </div>
          ) : null}
          {shownError ? (
            <Notice tone="err" role="alert">
              {shownError}
            </Notice>
          ) : null}
        </form>
      ) : null}
    </Dialog>
  );
}
