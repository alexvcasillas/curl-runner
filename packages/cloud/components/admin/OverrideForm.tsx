'use client';

import { useState } from 'react';
import type { GrantResult } from '@/app/admin/teams/actions';
import {
  dayOf,
  endOfDay,
  type OverrideBody,
  type OverrideInput,
  planName,
  type TeamPlan,
} from '@/lib/admin-teams';
import { Button } from '../Button';
import { Checkbox } from '../Checkbox';
import { Field } from '../Field';
import { Notice } from '../Notice';
import { Select } from '../Select';

const MAX_REASON = 500;

/**
 * Grant a plan, or edit the one in force. It sits in the Plan card the way the
 * board draws it. `stripe` is true when the team pays through Stripe: a grant
 * doesn't stop those charges, so the form says so.
 */
export function OverrideForm({
  teamName,
  initial,
  stripe,
  onSubmit,
  onCancel,
  onDone,
}: {
  teamName: string;
  /** The override being edited, or null for a new grant. */
  initial: OverrideBody | null;
  stripe: boolean;
  onSubmit: (input: OverrideInput) => Promise<GrantResult>;
  onCancel: () => void;
  onDone: (result: Extract<GrantResult, { ok: true }>, input: OverrideInput) => void;
}) {
  const editing = initial !== null;
  const [plan, setPlan] = useState<TeamPlan>(initial?.plan ?? 'team');
  const [day, setDay] = useState(initial?.until ? dayOf(initial.until) : '');
  const [noEnd, setNoEnd] = useState(initial ? initial.until === null : false);
  const [reason, setReason] = useState(initial?.reason ?? '');
  const [notify, setNotify] = useState(!editing);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    if (pending) {
      return;
    }
    let until: string | null = null;
    if (!noEnd) {
      if (day === '') {
        setError('Pick an end date, or choose No end date.');
        return;
      }
      until = endOfDay(day);
      if (until === null) {
        setError('That date isn’t valid.');
        return;
      }
      if (new Date(until).getTime() <= Date.now()) {
        setError('The end date must be in the future.');
        return;
      }
    }
    const input: OverrideInput = { plan, until, reason, notify };
    setPending(true);
    setError(null);
    try {
      const result = await onSubmit(input);
      if (result.ok) {
        onDone(result, input);
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
    <form onSubmit={submit} noValidate className="flex flex-col gap-3">
      <div className="flex flex-wrap gap-3">
        <Select
          label="Plan"
          value={plan}
          onChange={(e) => setPlan(e.target.value as TeamPlan)}
          options={[
            { value: 'team', label: 'Team' },
            { value: 'free', label: 'Free' },
          ]}
          className="w-[140px]"
        />
        <Field
          label="Until"
          type="date"
          value={day}
          disabled={noEnd}
          hint="Ends at the end of that day, UTC."
          onChange={(e) => setDay(e.target.value)}
          className="w-[150px]"
        />
        <Checkbox
          label="No end date"
          checked={noEnd}
          onChange={(e) => setNoEnd(e.target.checked)}
          className="self-end pb-1.5"
        />
      </div>
      <Field
        label="Reason, internal only"
        value={reason}
        maxLength={MAX_REASON}
        autoComplete="off"
        onChange={(e) => setReason(e.target.value)}
      />
      <Checkbox
        label={`Email the owners that curl-runner gave ${teamName} the ${planName(plan)} plan`}
        checked={notify}
        onChange={(e) => setNotify(e.target.checked)}
      />
      {stripe ? (
        <Notice tone="warn">
          {teamName} pays through Stripe. Granting doesn’t stop those charges. Cancel the
          subscription in Stripe too if this replaces it.
        </Notice>
      ) : null}
      {error ? (
        <Notice tone="err" role="alert">
          {error}
        </Notice>
      ) : null}
      <div className="flex gap-2">
        <Button onClick={onCancel} disabled={pending}>
          Cancel
        </Button>
        <Button type="submit" variant="primary" loading={pending}>
          {editing ? 'Save' : 'Grant'} {planName(plan)}
        </Button>
      </div>
    </form>
  );
}
