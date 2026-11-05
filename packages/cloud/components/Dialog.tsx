'use client';

import { useEffect, useId, useRef } from 'react';

function focusBack(opener: React.RefObject<HTMLElement | null>) {
  const el = opener.current;
  opener.current = null;
  if (el?.isConnected) {
    el.focus();
  }
}

import { cx } from '@/lib/cx';

export type DialogProps = {
  open: boolean;
  /** Escape, a click on the backdrop, or Cancel. */
  onClose: () => void;
  /** Names the thing it changes: "Delete Payments API". */
  title: React.ReactNode;
  /** What happens, in one or two sentences. */
  description?: React.ReactNode;
  children?: React.ReactNode;
  /** The buttons, safe choice first. */
  footer?: React.ReactNode;
  /** `alertdialog` for confirmations. */
  role?: 'dialog' | 'alertdialog';
  className?: string;
};

/**
 * A modal on the native `<dialog>`: the browser traps focus and makes the page
 * inert. Escape closes it and focus goes back to whatever opened it.
 */
export function Dialog({
  open,
  onClose,
  title,
  description,
  children,
  footer,
  role = 'dialog',
  className,
}: DialogProps) {
  const ref = useRef<HTMLDialogElement>(null);
  const opener = useRef<HTMLElement | null>(null);
  const titleId = useId();
  const descId = useId();

  useEffect(() => {
    const dialog = ref.current;
    if (!dialog) {
      return;
    }
    if (open && !dialog.open) {
      opener.current = document.activeElement as HTMLElement | null;
      if (typeof dialog.showModal === 'function') {
        dialog.showModal();
      } else {
        dialog.setAttribute('open', '');
      }
      // showModal() focuses the first control; do it ourselves where it doesn't.
      if (!dialog.contains(document.activeElement)) {
        dialog
          .querySelector<HTMLElement>('input, select, textarea, button, a[href], [tabindex]')
          ?.focus();
      }
    } else if (!open && dialog.open) {
      if (typeof dialog.close === 'function') {
        dialog.close();
      } else {
        dialog.removeAttribute('open');
        focusBack(opener);
      }
    }
  }, [open]);

  return (
    <dialog
      ref={ref}
      role={role === 'alertdialog' ? 'alertdialog' : undefined}
      aria-labelledby={titleId}
      aria-describedby={description ? descId : undefined}
      onCancel={(e) => {
        // Escape: let the parent decide, so `open` stays the source of truth.
        e.preventDefault();
        onClose();
      }}
      onKeyDown={(e) => {
        // Also handled here because not every engine sends `cancel` on Escape.
        if (e.key === 'Escape') {
          e.preventDefault();
          onClose();
        }
      }}
      onClose={() => focusBack(opener)}
      onClick={(e) => {
        if (e.target === e.currentTarget) {
          onClose();
        }
      }}
      className={cx(
        'm-auto box-border max-h-[calc(100dvh-32px)] w-[calc(100%-32px)] max-w-[440px] overflow-auto rounded-[12px] border border-control bg-raised p-0 text-fg shadow-dialog open:animate-rise',
        className,
      )}
    >
      <div className={cx('px-5 pt-5', !children && 'pb-5')}>
        <h2 id={titleId} className="m-0 text-[16px] leading-6 font-semibold">
          {title}
        </h2>
        {description ? (
          <p id={descId} className="mt-1 mb-0 text-fg-muted">
            {description}
          </p>
        ) : null}
      </div>
      {children ? <div className="flex flex-col gap-3.5 px-5 pt-4 pb-5">{children}</div> : null}
      {footer ? (
        <div className="flex flex-wrap items-center justify-end gap-2 border-t border-control bg-bg px-5 py-3">
          {footer}
        </div>
      ) : null}
    </dialog>
  );
}
