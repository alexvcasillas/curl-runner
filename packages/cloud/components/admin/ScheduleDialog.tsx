'use client';

import { useEffect, useId, useState } from 'react';
import type { ActionResult } from '@/lib/action-result';
import { peopleLabel, scheduledLabel, scheduleInstant } from '@/lib/admin-emails';
import { Button } from '../Button';
import { Dialog } from '../Dialog';
import { inputClasses } from '../field-styles';
import { Notice } from '../Notice';

/** "Europe/Madrid (UTC+01:00)". Read in the browser: the server doesn't know the admin's zone. */
function zoneLabel(at: Date): string {
  let name = 'your time zone';
  try {
    name = Intl.DateTimeFormat().resolvedOptions().timeZone || name;
  } catch {
    // keep the fallback
  }
  const offset = -at.getTimezoneOffset();
  const sign = offset < 0 ? '-' : '+';
  const abs = Math.abs(offset);
  const hh = String(Math.floor(abs / 60)).padStart(2, '0');
  const mm = String(abs % 60).padStart(2, '0');
  return `${name} (UTC${sign}${hh}:${mm})`;
}

/** `YYYY-MM-DDTHH:mm` in local time, for the input's `min`. */
function localInput(d: Date): string {
  const p = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
}

/**
 * Picks when a broadcast goes out. The time is read in the browser's zone and
 * sent as the UTC instant the server stores; a past time is refused here too.
 */
export function ScheduleDialog({
  open,
  onClose,
  rescheduling,
  count,
  current,
  onSubmit,
  onDone,
}: {
  open: boolean;
  onClose: () => void;
  rescheduling: boolean;
  /** People it reaches today. */
  count: number | null;
  /** The time it is already scheduled for, to start from. */
  current: string | null;
  onSubmit: (iso: string) => Promise<ActionResult>;
  onDone: (iso: string) => void;
}) {
  const [value, setValue] = useState('');
  const [zone, setZone] = useState('');
  const [min, setMin] = useState('');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const formId = useId();
  const inputId = `${formId}-at`;

  useEffect(() => {
    if (open) {
      const now = new Date();
      setValue(current ? localInput(new Date(current)) : '');
      setZone(zoneLabel(now));
      setMin(localInput(now));
      setPending(false);
      setError(null);
    }
  }, [open, current]);

  const parsed = value ? scheduleInstant(value, new Date()) : null;

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (pending) {
      return;
    }
    const at = scheduleInstant(value, new Date());
    if (!at.ok) {
      setError(at.message);
      return;
    }
    setPending(true);
    setError(null);
    try {
      const result = await onSubmit(at.iso);
      if (result.ok) {
        onClose();
        onDone(at.iso);
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
      title="Schedule this broadcast"
      description={
        count === null
          ? 'It goes out at the time you pick.'
          : `It goes to ${peopleLabel(count)} as of now. The count is checked again when it goes out.`
      }
      footer={
        <>
          <Button onClick={onClose} disabled={pending}>
            Cancel
          </Button>
          <Button type="submit" form={formId} variant="primary" loading={pending}>
            {pending ? 'Scheduling…' : rescheduling ? 'Reschedule' : 'Schedule'}
          </Button>
        </>
      }
    >
      <form id={formId} onSubmit={submit} noValidate className="flex flex-col gap-3.5">
        <div className="flex flex-col gap-1.5 text-[12px]">
          <label htmlFor={inputId} className="text-fg-muted">
            Send at
          </label>
          <input
            id={inputId}
            type="datetime-local"
            value={value}
            min={min || undefined}
            onChange={(e) => {
              setValue(e.target.value);
              setError(null);
            }}
            className={inputClasses(error !== null)}
          />
          <span className="text-fg-subtle">Time zone: {zone || 'your time zone'}</span>
          <span className="text-fg-subtle">
            Stored and sent in UTC
            {parsed?.ok ? `: ${scheduledLabel(parsed.iso)}` : '.'}
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
