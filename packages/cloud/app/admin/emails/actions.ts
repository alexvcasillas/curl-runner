'use server';

import { revalidatePath } from 'next/cache';
import type { ActionResult } from '@/lib/action-result';
import { adminRoutes } from '@/lib/admin';
import { requireAdmin } from '@/lib/admin-data';
import {
  type Audience,
  type Broadcast,
  type BroadcastResult,
  type DraftInput,
  isAudience,
  MAX_BODY,
  MAX_SUBJECT,
  type Preview,
} from '@/lib/admin-emails';
import { api } from '@/lib/api';
import { attempt, INVALID } from '@/lib/team-action';
import { isUuid } from '@/lib/team-input';

const length = (s: string) => [...s].length;

const NO_BROADCAST = {
  ...INVALID,
  code: 'not_found' as const,
  message: 'That broadcast doesn’t exist.',
};

/** A sending or sent broadcast can't change. */
const LOCKED = 'This broadcast is already going out, so it can’t change.';

function refresh() {
  revalidatePath(adminRoutes.emails);
}

/** A draft's fields, checked, or the copy to show. Absent fields stay absent. */
function checkedDraft(
  input: Partial<DraftInput> | undefined,
): { ok: true; body: Partial<DraftInput> } | Extract<ActionResult, { ok: false }> {
  if (input === undefined || input === null) {
    return { ok: true, body: {} };
  }
  if (typeof input !== 'object') {
    return INVALID;
  }
  const body: Partial<DraftInput> = {};
  if (input.subject !== undefined) {
    if (typeof input.subject !== 'string') {
      return INVALID;
    }
    const subject = input.subject.trim();
    if (length(subject) > MAX_SUBJECT) {
      return { ...INVALID, message: 'The subject can be at most 200 characters.' };
    }
    body.subject = subject;
  }
  if (input.body_md !== undefined) {
    if (typeof input.body_md !== 'string') {
      return INVALID;
    }
    if (length(input.body_md) > MAX_BODY) {
      return { ...INVALID, message: 'The message can be at most 20000 characters.' };
    }
    body.body_md = input.body_md;
  }
  if (input.audience !== undefined) {
    if (!isAudience(input.audience)) {
      return INVALID;
    }
    body.audience = input.audience;
  }
  return { ok: true, body };
}

/** `POST /v1/admin/broadcasts`: a new draft. Every field may start empty. */
export async function createBroadcast(input?: Partial<DraftInput>): Promise<BroadcastResult> {
  const denied = await requireAdmin('emails');
  if (denied) {
    return denied;
  }
  const draft = checkedDraft(input);
  if (!draft.ok) {
    return draft;
  }
  const result = await attempt(() =>
    api<Broadcast>('/v1/admin/broadcasts', { method: 'POST', json: draft.body }),
  );
  if (!result.ok) {
    return result;
  }
  refresh();
  return { ok: true, broadcast: result.value };
}

/** `PATCH …/{id}`: autosave. A sending or sent broadcast is the API's 409. */
export async function saveBroadcast(id: string, input: DraftInput): Promise<BroadcastResult> {
  const denied = await requireAdmin('emails');
  if (denied) {
    return denied;
  }
  if (!isUuid(id)) {
    return NO_BROADCAST;
  }
  const draft = checkedDraft(input);
  if (!draft.ok) {
    return draft;
  }
  const result = await attempt(
    () => api<Broadcast>(`/v1/admin/broadcasts/${id}`, { method: 'PATCH', json: draft.body }),
    { conflict: LOCKED },
  );
  if (!result.ok) {
    return result;
  }
  return { ok: true, broadcast: result.value };
}

/** `POST …/{id}/test`: one copy to the admin, with their own variables. Ten an hour. */
export async function sendTest(
  id: string,
): Promise<{ ok: true; to: string } | Extract<ActionResult, { ok: false }>> {
  const denied = await requireAdmin('emails');
  if (denied) {
    return denied;
  }
  if (!isUuid(id)) {
    return NO_BROADCAST;
  }
  const result = await attempt(() =>
    api<{ to: string }>(`/v1/admin/broadcasts/${id}/test`, { method: 'POST' }),
  );
  if (!result.ok) {
    return result;
  }
  refresh();
  return { ok: true, to: result.value.to };
}

/**
 * `POST …/{id}/send`. Without `at` it goes now. With one (any ISO instant, in
 * the future) it is scheduled; the API stores UTC.
 */
export async function sendBroadcast(id: string, at?: string | null): Promise<BroadcastResult> {
  const denied = await requireAdmin('emails');
  if (denied) {
    return denied;
  }
  if (!isUuid(id)) {
    return NO_BROADCAST;
  }
  let when: string | null = null;
  if (at !== undefined && at !== null) {
    const date = typeof at === 'string' ? new Date(at) : null;
    if (!date || Number.isNaN(date.getTime())) {
      return { ...INVALID, message: 'Pick a date and a time.' };
    }
    if (date.getTime() <= Date.now()) {
      return { ...INVALID, message: 'Pick a time in the future.' };
    }
    when = date.toISOString();
  }
  const result = await attempt(
    () =>
      api<Broadcast>(`/v1/admin/broadcasts/${id}/send`, {
        method: 'POST',
        json: when ? { at: when } : {},
      }),
    { conflict: LOCKED },
  );
  if (!result.ok) {
    return result;
  }
  refresh();
  return { ok: true, broadcast: result.value };
}

/**
 * `GET …/{id}/preview?as_user=`: the subject, HTML and text one person would get.
 * The server renders the Markdown and escapes it; the page only shows the result,
 * in a sandboxed frame. Any admin can read it.
 */
export async function previewBroadcast(
  id: string,
  asUser: string,
): Promise<{ ok: true; preview: Preview } | Extract<ActionResult, { ok: false }>> {
  const denied = await requireAdmin();
  if (denied) {
    return denied;
  }
  if (!isUuid(id)) {
    return NO_BROADCAST;
  }
  const who = typeof asUser === 'string' ? asUser.trim() : '';
  if ([...who].length > 254) {
    return { ...INVALID, message: 'That isn’t a user id or an email.' };
  }
  const qs = who ? `?as_user=${encodeURIComponent(who)}` : '';
  const result = await attempt(() => api<Preview>(`/v1/admin/broadcasts/${id}/preview${qs}`));
  if (!result.ok) {
    return result;
  }
  return { ok: true, preview: result.value };
}

/** `GET /v1/admin/audience-count`: the people it would reach now, after opt-outs. */
export async function audienceCount(
  audience: Audience,
): Promise<{ ok: true; count: number } | Extract<ActionResult, { ok: false }>> {
  const denied = await requireAdmin();
  if (denied) {
    return denied;
  }
  if (!isAudience(audience)) {
    return INVALID;
  }
  const result = await attempt(() =>
    api<{ count: number }>(`/v1/admin/audience-count?audience=${audience}`),
  );
  if (!result.ok) {
    return result;
  }
  return { ok: true, count: result.value.count };
}
