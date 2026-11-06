'use server';

import { revalidatePath } from 'next/cache';
import type { ActionResult } from '@/lib/action-result';
import { adminRoutes, isPerm, type Perm } from '@/lib/admin';
import { requireAdmin } from '@/lib/admin-data';
import { api } from '@/lib/api';
import { attempt, INVALID } from '@/lib/team-action';
import { isUuid } from '@/lib/team-input';

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

function validPerms(perms: unknown): perms is Perm[] {
  return Array.isArray(perms) && perms.every(isPerm);
}

/**
 * `POST /v1/admin/admins`: an account that already exists, plus what it may
 * change. No email goes out and there is nothing pending. An owner is the
 * API's 409, an email with no account its 404.
 */
export async function addAdmin(email: string, permissions: Perm[]): Promise<ActionResult> {
  const denied = await requireAdmin('admins');
  if (denied) {
    return denied;
  }
  const to = typeof email === 'string' ? email.trim().toLowerCase() : '';
  if (!EMAIL.test(to) || !validPerms(permissions)) {
    return { ...INVALID, message: 'Enter the email of an account that has signed in.' };
  }
  const result = await attempt(() =>
    api('/v1/admin/admins', { method: 'POST', json: { email: to, permissions } }),
  );
  if (!result.ok) {
    return result;
  }
  revalidatePath(adminRoutes.admins);
  return { ok: true };
}

/** `PATCH /v1/admin/admins/{id}`. The admin sees it on their next request. */
export async function updateAdmin(userId: string, permissions: Perm[]): Promise<ActionResult> {
  const denied = await requireAdmin('admins');
  if (denied) {
    return denied;
  }
  if (!isUuid(userId) || !validPerms(permissions)) {
    return INVALID;
  }
  const result = await attempt(() =>
    api(`/v1/admin/admins/${userId}`, { method: 'PATCH', json: { permissions } }),
  );
  if (!result.ok) {
    return result;
  }
  revalidatePath(adminRoutes.admins);
  return { ok: true };
}

/** `DELETE /v1/admin/admins/{id}`. The person keeps their account and their teams. */
export async function removeAdmin(userId: string): Promise<ActionResult> {
  const denied = await requireAdmin('admins');
  if (denied) {
    return denied;
  }
  if (!isUuid(userId)) {
    return INVALID;
  }
  const result = await attempt(() => api(`/v1/admin/admins/${userId}`, { method: 'DELETE' }));
  if (!result.ok) {
    return result;
  }
  revalidatePath(adminRoutes.admins);
  return { ok: true };
}
