'use server';

import { revalidatePath } from 'next/cache';
import { redirect } from 'next/navigation';
import type { ActionResult } from '@/lib/action-result';
import { api } from '@/lib/api';
import { getMe } from '@/lib/app-data';
import { routes } from '@/lib/routes';
import { attempt, INVALID, NO_TEAM, validTeam } from '@/lib/team-action';
import { isRole, isUuid, normalizeEmail } from '@/lib/team-input';
import type { Role } from '@/lib/types';

const enc = encodeURIComponent;
const members = (team: string) => routes.teamTab(team, 'members');

export type InviteResult = { ok: true; email: string } | Extract<ActionResult, { ok: false }>;

/** `POST …/invites`. Owners only; the server checks. */
export async function inviteMember(team: string, email: string, role: Role): Promise<InviteResult> {
  if (!validTeam(team)) {
    return NO_TEAM;
  }
  const clean = normalizeEmail(email);
  if (clean === null || !isRole(role)) {
    return { ...INVALID, message: 'Enter a valid email address.' };
  }
  const result = await attempt(() =>
    api(`/v1/teams/${enc(team)}/invites`, { method: 'POST', json: { email: clean, role } }),
  );
  if (!result.ok) {
    return result;
  }
  revalidatePath(members(team));
  return { ok: true, email: clean };
}

/** `DELETE …/invites/{id}`. */
export async function cancelInvite(team: string, id: string): Promise<ActionResult> {
  if (!validTeam(team)) {
    return NO_TEAM;
  }
  if (!isUuid(id)) {
    return INVALID;
  }
  const result = await attempt(() =>
    api(`/v1/teams/${enc(team)}/invites/${enc(id)}`, { method: 'DELETE' }),
  );
  if (!result.ok) {
    return result;
  }
  revalidatePath(members(team));
  return { ok: true };
}

/** `PATCH …/members/{user_id}`. */
export async function changeRole(team: string, userId: string, role: Role): Promise<ActionResult> {
  if (!validTeam(team)) {
    return NO_TEAM;
  }
  if (!isUuid(userId) || !isRole(role)) {
    return INVALID;
  }
  const result = await attempt(() =>
    api(`/v1/teams/${enc(team)}/members/${enc(userId)}`, { method: 'PATCH', json: { role } }),
  );
  if (!result.ok) {
    return result;
  }
  revalidatePath(members(team));
  revalidatePath('/', 'layout');
  return { ok: true };
}

/** `DELETE …/members/{user_id}`: an owner removing someone. */
export async function removeMember(team: string, userId: string): Promise<ActionResult> {
  if (!validTeam(team)) {
    return NO_TEAM;
  }
  if (!isUuid(userId)) {
    return INVALID;
  }
  const result = await attempt(() =>
    api(`/v1/teams/${enc(team)}/members/${enc(userId)}`, { method: 'DELETE' }),
  );
  if (!result.ok) {
    return result;
  }
  revalidatePath(members(team));
  return { ok: true };
}

/**
 * Leaving is removing yourself. The id comes from the session, never from the
 * page. The last owner gets a 409 and stays. Anyone else goes home.
 */
export async function leaveTeam(team: string): Promise<ActionResult> {
  if (!validTeam(team)) {
    return NO_TEAM;
  }
  const me = await getMe();
  const result = await attempt(() =>
    api(`/v1/teams/${enc(team)}/members/${enc(me.user.id)}`, { method: 'DELETE' }),
  );
  if (!result.ok) {
    return result;
  }
  revalidatePath('/', 'layout');
  redirect(routes.home);
}
