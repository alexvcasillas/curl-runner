'use server';

import { revalidatePath } from 'next/cache';
import { redirect } from 'next/navigation';
import type { ActionResult } from '@/lib/action-result';
import { adminRoutes } from '@/lib/admin';
import { requireAdmin } from '@/lib/admin-data';
import type { OverrideInput } from '@/lib/admin-teams';
import { api } from '@/lib/api';
import { attempt, INVALID, NO_TEAM, validTeam } from '@/lib/team-action';
import { isRole, isUuid } from '@/lib/team-input';

const enc = encodeURIComponent;

/** The server's limits, checked here too so a bad form never leaves the dashboard. */
const MAX_REASON = 500;
const MAX_SUBJECT = 200;
const MAX_BODY = 10_000;

const length = (s: string) => [...s].length;

function refresh(slug?: string) {
  revalidatePath(adminRoutes.teams);
  if (slug) {
    revalidatePath(adminRoutes.team(slug));
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

/** `PUT …/{slug}/override`. `stripeConflict` is true when a live subscription keeps billing. */
export type GrantResult =
  | { ok: true; stripeConflict: boolean }
  | Extract<ActionResult, { ok: false }>;

export async function grantOverride(slug: string, input: OverrideInput): Promise<GrantResult> {
  const denied = await requireAdmin('billing');
  if (denied) {
    return denied;
  }
  if (typeof slug !== 'string' || !validTeam(slug)) {
    return NO_TEAM;
  }
  if (!input || (input.plan !== 'free' && input.plan !== 'team')) {
    return INVALID;
  }
  const reason = reasonOf(input.reason);
  if (reason === null) {
    return TOO_LONG;
  }
  let until: string | null = null;
  if (input.until !== null && input.until !== undefined) {
    const at = typeof input.until === 'string' ? new Date(input.until) : null;
    if (!at || Number.isNaN(at.getTime())) {
      return INVALID;
    }
    if (at.getTime() <= Date.now()) {
      return { ...INVALID, message: 'The end date must be in the future.' };
    }
    until = at.toISOString();
  }
  const result = await attempt(() =>
    api<{ stripe_conflict: boolean }>(`/v1/admin/teams/${enc(slug)}/override`, {
      method: 'PUT',
      json: { plan: input.plan, until, reason, notify: input.notify === true },
    }),
  );
  if (!result.ok) {
    return result;
  }
  refresh(slug);
  return { ok: true, stripeConflict: result.value.stripe_conflict === true };
}

/** `DELETE …/{slug}/override`. A team with none gets the API's 409. */
export async function removeOverride(slug: string): Promise<ActionResult> {
  const denied = await requireAdmin('billing');
  if (denied) {
    return denied;
  }
  if (typeof slug !== 'string' || !validTeam(slug)) {
    return NO_TEAM;
  }
  const result = await attempt(() =>
    api(`/v1/admin/teams/${enc(slug)}/override`, { method: 'DELETE' }),
  );
  if (!result.ok) {
    return result;
  }
  refresh(slug);
  return { ok: true };
}

/** `POST …/{slug}/suspend`. An already suspended team is the API's 409. */
export async function suspendTeam(
  slug: string,
  reason: string,
  notify: boolean,
): Promise<ActionResult> {
  const denied = await requireAdmin('teams');
  if (denied) {
    return denied;
  }
  if (typeof slug !== 'string' || !validTeam(slug)) {
    return NO_TEAM;
  }
  const clean = reasonOf(reason);
  if (clean === null) {
    return TOO_LONG;
  }
  const result = await attempt(() =>
    api(`/v1/admin/teams/${enc(slug)}/suspend`, {
      method: 'POST',
      json: { reason: clean, notify: notify === true },
    }),
  );
  if (!result.ok) {
    return result;
  }
  refresh(slug);
  return { ok: true };
}

/** `POST …/{slug}/unsuspend`. */
export async function unsuspendTeam(slug: string): Promise<ActionResult> {
  const denied = await requireAdmin('teams');
  if (denied) {
    return denied;
  }
  if (typeof slug !== 'string' || !validTeam(slug)) {
    return NO_TEAM;
  }
  const result = await attempt(() =>
    api(`/v1/admin/teams/${enc(slug)}/unsuspend`, { method: 'POST' }),
  );
  if (!result.ok) {
    return result;
  }
  refresh(slug);
  return { ok: true };
}

/** `DELETE …/{slug}`, then back to the list. The API cancels the Stripe subscription too. */
export async function deleteTeamAndReturn(slug: string): Promise<ActionResult> {
  const denied = await requireAdmin('teams');
  if (denied) {
    return denied;
  }
  if (typeof slug !== 'string' || !validTeam(slug)) {
    return NO_TEAM;
  }
  const result = await attempt(() => api(`/v1/admin/teams/${enc(slug)}`, { method: 'DELETE' }));
  if (!result.ok) {
    return result;
  }
  refresh();
  redirect(adminRoutes.teams);
}

/** `PATCH …/{slug}/members/{user_id}`. The last owner can't be demoted: the API's 409. */
export async function setMemberRole(
  slug: string,
  userId: string,
  role: 'owner' | 'member',
): Promise<ActionResult> {
  const denied = await requireAdmin('teams');
  if (denied) {
    return denied;
  }
  if (typeof slug !== 'string' || !validTeam(slug)) {
    return NO_TEAM;
  }
  if (typeof userId !== 'string' || !isUuid(userId) || !isRole(role)) {
    return INVALID;
  }
  const result = await attempt(() =>
    api(`/v1/admin/teams/${enc(slug)}/members/${userId}`, { method: 'PATCH', json: { role } }),
  );
  if (!result.ok) {
    return result;
  }
  refresh(slug);
  return { ok: true };
}

/** `POST …/{slug}/stripe-sync`: read the subscription from Stripe and apply it. */
export async function stripeSync(slug: string): Promise<ActionResult> {
  const denied = await requireAdmin('billing');
  if (denied) {
    return denied;
  }
  if (typeof slug !== 'string' || !validTeam(slug)) {
    return NO_TEAM;
  }
  const result = await attempt(() =>
    api(`/v1/admin/teams/${enc(slug)}/stripe-sync`, { method: 'POST' }),
  );
  if (!result.ok) {
    return result;
  }
  refresh(slug);
  return { ok: true };
}

/** `POST …/{slug}/resend-invoice`. A 409 means Stripe has nothing finished to send. */
export async function resendInvoice(slug: string): Promise<ActionResult> {
  const denied = await requireAdmin('billing');
  if (denied) {
    return denied;
  }
  if (typeof slug !== 'string' || !validTeam(slug)) {
    return NO_TEAM;
  }
  const result = await attempt(
    () => api(`/v1/admin/teams/${enc(slug)}/resend-invoice`, { method: 'POST' }),
    { conflict: 'There’s no finished invoice to resend yet.' },
  );
  if (!result.ok) {
    return result;
  }
  refresh(slug);
  return { ok: true };
}

/** `POST …/{slug}/email-owners`. The body is Markdown. */
export async function emailOwners(
  slug: string,
  subject: string,
  body: string,
): Promise<ActionResult> {
  const denied = await requireAdmin('emails');
  if (denied) {
    return denied;
  }
  if (typeof slug !== 'string' || !validTeam(slug)) {
    return NO_TEAM;
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
    api(`/v1/admin/teams/${enc(slug)}/email-owners`, {
      method: 'POST',
      json: { subject: s, body_md: b },
    }),
  );
  if (!result.ok) {
    return result;
  }
  refresh(slug);
  return { ok: true };
}
