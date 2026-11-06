'use server';

import { revalidatePath } from 'next/cache';
import { redirect } from 'next/navigation';
import type { ActionResult } from '@/lib/action-result';
import { adminRoutes } from '@/lib/admin';
import { api } from '@/lib/api';
import { INVALID_FORM } from '@/lib/errors';
import { attempt, INVALID } from '@/lib/team-action';
import { isUuid } from '@/lib/team-input';

const enc = encodeURIComponent;

/** The server's limits, checked here too so a bad form never leaves the dashboard. */
const MAX_REASON = 500;
const MAX_SUBJECT = 200;
const MAX_BODY = 10_000;
const MAX_BULK = 200;

const NO_USER: Extract<ActionResult, { ok: false }> = {
  ok: false,
  message: 'That user doesn’t exist.',
  code: 'not_found',
};

const length = (s: string) => [...s].length;

function refresh(id?: string) {
  revalidatePath(adminRoutes.users);
  if (id) {
    revalidatePath(adminRoutes.user(id));
  }
}

/** The reason, trimmed, or null when it is too long. Empty is fine. */
function reasonOf(raw: unknown): string | null {
  if (raw === undefined || raw === null) {
    return '';
  }
  if (typeof raw !== 'string') {
    return null;
  }
  const reason = raw.trim();
  return length(reason) <= MAX_REASON ? reason : null;
}

const TOO_LONG = { ...INVALID, message: 'The reason is too long.' };

/** `POST …/{id}/suspend`. Owners, you and other admins come back as the API's 409 or 403. */
export async function suspendUser(
  id: string,
  reason: string,
  notify: boolean,
): Promise<ActionResult> {
  if (!isUuid(id)) {
    return NO_USER;
  }
  const clean = reasonOf(reason);
  if (clean === null) {
    return TOO_LONG;
  }
  const result = await attempt(() =>
    api(`/v1/admin/users/${id}/suspend`, {
      method: 'POST',
      json: { reason: clean, notify: notify === true },
    }),
  );
  if (!result.ok) {
    return result;
  }
  refresh(id);
  return { ok: true };
}

/** `POST …/{id}/unsuspend`. */
export async function unsuspendUser(id: string): Promise<ActionResult> {
  if (!isUuid(id)) {
    return NO_USER;
  }
  const result = await attempt(() => api(`/v1/admin/users/${id}/unsuspend`, { method: 'POST' }));
  if (!result.ok) {
    return result;
  }
  refresh(id);
  return { ok: true };
}

/** `POST …/{id}/sign-out-everywhere`. */
export async function signOutEverywhere(id: string): Promise<ActionResult> {
  if (!isUuid(id)) {
    return NO_USER;
  }
  const result = await attempt(() =>
    api(`/v1/admin/users/${id}/sign-out-everywhere`, { method: 'POST' }),
  );
  if (!result.ok) {
    return result;
  }
  refresh(id);
  return { ok: true };
}

/** `DELETE …/{id}/sessions/{sid}`. */
export async function revokeSession(id: string, sessionId: string): Promise<ActionResult> {
  if (!isUuid(id) || !isUuid(sessionId)) {
    return INVALID;
  }
  const result = await attempt(() =>
    api(`/v1/admin/users/${id}/sessions/${sessionId}`, { method: 'DELETE' }),
  );
  if (!result.ok) {
    return result;
  }
  refresh(id);
  return { ok: true };
}

/** Provider names the server stores: lower-case words. `email` is the account itself. */
const PROVIDER = /^[a-z0-9_-]{1,32}$/;

/** `DELETE …/{id}/identities/{provider}`. */
export async function unlinkIdentity(id: string, provider: string): Promise<ActionResult> {
  if (!isUuid(id)) {
    return NO_USER;
  }
  if (typeof provider !== 'string' || !PROVIDER.test(provider) || provider === 'email') {
    return INVALID;
  }
  const result = await attempt(() =>
    api(`/v1/admin/users/${id}/identities/${enc(provider)}`, { method: 'DELETE' }),
  );
  if (!result.ok) {
    return result;
  }
  refresh(id);
  return { ok: true };
}

/**
 * `DELETE …/{id}`. A user who is the only owner of a team gets a 409 that names
 * it. From the detail page a delete goes back to the list.
 */
export async function deleteUser(
  id: string,
  options?: { redirectTo?: 'list' },
): Promise<ActionResult> {
  if (!isUuid(id)) {
    return NO_USER;
  }
  const result = await attempt(() => api(`/v1/admin/users/${id}`, { method: 'DELETE' }));
  if (!result.ok) {
    return result;
  }
  refresh();
  if (options?.redirectTo === 'list') {
    redirect(adminRoutes.users);
  }
  return { ok: true };
}

/** Delete from the detail page: the same call, then back to the list. */
export async function deleteUserAndReturn(id: string): Promise<ActionResult> {
  return deleteUser(id, { redirectTo: 'list' });
}

/** `POST …/{id}/email`. The body is Markdown. */
export async function emailUser(id: string, subject: string, body: string): Promise<ActionResult> {
  if (!isUuid(id)) {
    return NO_USER;
  }
  const s = typeof subject === 'string' ? subject.trim() : '';
  const b = typeof body === 'string' ? body.trim() : '';
  if (s === '' || length(s) > MAX_SUBJECT) {
    return { ...INVALID, message: 'The subject must be 1 to 200 characters.' };
  }
  if (b === '' || length(b) > MAX_BODY) {
    return { ...INVALID, message: 'The message must be 1 to 10000 characters.' };
  }
  const result = await attempt(() =>
    api(`/v1/admin/users/${id}/email`, { method: 'POST', json: { subject: s, body_md: b } }),
  );
  if (!result.ok) {
    return result;
  }
  refresh(id);
  return { ok: true };
}

export type BulkResult =
  | { ok: true; done: number; skipped: string[] }
  | Extract<ActionResult, { ok: false }>;

/** `POST /v1/admin/users/bulk`: 1 to 200 ids. Anyone the server leaves alone comes back in `skipped`. */
export async function bulkUsers(
  ids: string[],
  action: 'sign_out' | 'suspend',
  reason = '',
): Promise<BulkResult> {
  if (
    !Array.isArray(ids) ||
    ids.length === 0 ||
    ids.length > MAX_BULK ||
    !ids.every(isUuid) ||
    (action !== 'sign_out' && action !== 'suspend')
  ) {
    return { ...INVALID, message: INVALID_FORM };
  }
  const clean = reasonOf(reason);
  if (clean === null) {
    return TOO_LONG;
  }
  const json = action === 'suspend' ? { ids, action, reason: clean } : { ids, action };
  const result = await attempt(() =>
    api<{ done: number; skipped: string[] }>('/v1/admin/users/bulk', { method: 'POST', json }),
  );
  if (!result.ok) {
    return result;
  }
  refresh();
  return { ok: true, done: result.value.done, skipped: result.value.skipped };
}
