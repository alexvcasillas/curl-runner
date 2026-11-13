import { cx } from '@/lib/cx';

/**
 * An on/off setting that applies at once. A button with `role="switch"`, so
 * Space and Enter toggle it. The hit area is 44px tall, the track is 20px.
 */
export function Switch({
  checked,
  onChange,
  disabled,
  id,
  'aria-labelledby': labelledBy,
  'aria-describedby': describedBy,
  'aria-label': label,
}: {
  checked: boolean;
  onChange: (next: boolean) => void;
  disabled?: boolean;
  id?: string;
  'aria-labelledby'?: string;
  'aria-describedby'?: string;
  'aria-label'?: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      id={id}
      aria-checked={checked}
      aria-label={label}
      aria-labelledby={labelledBy}
      aria-describedby={describedBy}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className="group box-border flex h-11 w-14 shrink-0 cursor-pointer items-center justify-center border-0 bg-transparent p-0 outline-none disabled:cursor-not-allowed"
    >
      <span
        aria-hidden="true"
        className={cx(
          'relative box-border block h-5 w-9 rounded-full border transition-colors group-focus-visible:outline-2 group-focus-visible:outline-offset-2 group-focus-visible:outline-blue group-disabled:opacity-50',
          checked ? 'border-primary bg-primary' : 'border-control bg-surface-300',
        )}
      >
        <span
          className={cx(
            'absolute top-px left-px block size-4 rounded-full transition-transform',
            checked ? 'translate-x-4 bg-on-primary' : 'bg-fg-muted',
          )}
        />
      </span>
    </button>
  );
}
