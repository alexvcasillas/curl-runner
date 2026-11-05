'use client';

import { useEffect, useRef, useState } from 'react';
import { cx } from '@/lib/cx';
import { Button } from './Button';

export type CopyFieldProps = {
  value: string;
  /** What the value is, for the Copy button's name: "token", "share link". */
  label: string;
  /**
   * `secret`: shown once, wraps, bright Copy button.
   * `command`: one line with a `$` prompt and a quiet Copy button.
   */
  variant?: 'secret' | 'command';
  onCopy?: () => void;
  className?: string;
};

async function writeClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

/** A read-only value with a Copy button. */
export function CopyField({
  value,
  label,
  variant = 'command',
  onCopy,
  className,
}: CopyFieldProps) {
  const [copied, setCopied] = useState(false);
  const [failed, setFailed] = useState(false);
  const valueRef = useRef<HTMLSpanElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => () => clearTimeout(timer.current), []);

  async function copy() {
    clearTimeout(timer.current);
    const ok = await writeClipboard(value);
    setCopied(ok);
    setFailed(!ok);
    if (ok) {
      onCopy?.();
    } else if (valueRef.current) {
      // No clipboard access: select the text so the person can copy it.
      const range = document.createRange();
      range.selectNodeContents(valueRef.current);
      const selection = window.getSelection();
      selection?.removeAllRanges();
      selection?.addRange(range);
    }
    timer.current = setTimeout(() => {
      setCopied(false);
      setFailed(false);
    }, 2000);
  }

  const secret = variant === 'secret';
  return (
    <div
      className={cx(
        'box-border flex items-center gap-2 bg-raised',
        secret
          ? 'rounded-[8px] border border-control py-1.5 pr-1.5 pl-3.5'
          : 'min-h-9 rounded-[6px] border border-hairline py-1 pr-1 pl-3 max-sm:min-h-11 max-sm:rounded-[8px] max-sm:py-1.5 max-sm:pr-1.5',
        className,
      )}
    >
      <span
        ref={valueRef}
        className={cx(
          'min-w-0 flex-1 font-mono text-[12px] text-fg',
          secret ? '[overflow-wrap:anywhere]' : 'sm:truncate max-sm:[overflow-wrap:anywhere]',
        )}
      >
        {secret ? null : <span className="text-fg-subtle">$ </span>}
        {value}
      </span>
      <Button
        variant={secret ? 'primary' : 'ghost'}
        size={secret ? 'md' : 'sm'}
        onClick={copy}
        aria-label={copied ? `Copied ${label}` : `Copy ${label}`}
        className={secret ? undefined : 'max-sm:h-8'}
      >
        {copied ? 'Copied' : 'Copy'}
      </Button>
      <span role="status" className="sr-only">
        {copied ? `Copied ${label}` : failed ? 'Couldn’t copy. The text is selected.' : ''}
      </span>
    </div>
  );
}
