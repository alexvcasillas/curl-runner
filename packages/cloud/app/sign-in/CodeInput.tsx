'use client';

import { useRef } from 'react';
import { cx } from '@/lib/cx';

const LENGTH = 6;

export type CodeInputProps = {
  value: string;
  onChange: (value: string) => void;
  invalid?: boolean;
  disabled?: boolean;
  /** Form field name for the joined code. */
  name?: string;
};

/**
 * Six boxes for the emailed code. Typing moves along, backspace moves back, and
 * pasting or autofilling the whole code fills every box.
 */
export function CodeInput({ value, onChange, invalid, disabled, name = 'code' }: CodeInputProps) {
  const boxes = useRef<(HTMLInputElement | null)[]>([]);
  const focus = (i: number) => boxes.current[Math.max(0, Math.min(LENGTH - 1, i))]?.focus();

  function put(i: number, raw: string) {
    const digits = raw.replace(/\D/g, '');
    if (!digits) {
      onChange(value.slice(0, i) + value.slice(i + 1));
      return;
    }
    // Overwrite from this box on, and never leave a gap before it.
    const at = Math.min(i, value.length);
    onChange((value.slice(0, at) + digits + value.slice(at + digits.length)).slice(0, LENGTH));
    focus(at + digits.length);
  }

  return (
    <fieldset className="m-0 flex min-w-0 flex-col gap-1.5 border-0 p-0">
      <legend className="mb-1.5 p-0 text-[12px] text-fg-muted">Code</legend>
      <div className="flex justify-between gap-2">
        {Array.from({ length: LENGTH }, (_, i) => (
          <input
            key={i}
            ref={(el) => {
              boxes.current[i] = el;
            }}
            value={value[i] ?? ''}
            disabled={disabled}
            inputMode="numeric"
            autoComplete={i === 0 ? 'one-time-code' : 'off'}
            maxLength={i === 0 ? LENGTH : 1}
            aria-label={`Digit ${i + 1} of ${LENGTH}`}
            aria-invalid={invalid ? true : undefined}
            onChange={(e) => put(i, e.target.value)}
            onFocus={(e) => e.target.select()}
            onPaste={(e) => {
              e.preventDefault();
              put(i, e.clipboardData.getData('text'));
            }}
            onKeyDown={(e) => {
              if (e.key === 'Backspace' && !value[i] && i > 0) {
                e.preventDefault();
                onChange(value.slice(0, i - 1) + value.slice(i));
                focus(i - 1);
              } else if (e.key === 'ArrowLeft') {
                focus(i - 1);
              } else if (e.key === 'ArrowRight') {
                focus(i + 1);
              }
            }}
            className={cx(
              'box-border h-14 w-12 min-w-0 rounded-[8px] border bg-bg text-center font-mono text-[22px] text-fg outline-none focus-visible:border-blue disabled:cursor-not-allowed disabled:text-fg-disabled max-sm:h-12 max-sm:w-10',
              invalid ? 'border-danger' : 'border-control',
            )}
          />
        ))}
      </div>
      <input type="hidden" name={name} value={value} />
    </fieldset>
  );
}
