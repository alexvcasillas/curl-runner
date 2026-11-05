'use server';

import { revalidatePath } from 'next/cache';
import { redirect } from 'next/navigation';
import type { ActionResult } from '@/lib/action-result';
import { api } from '@/lib/api';
import { routes } from '@/lib/routes';
import { clearSession } from '@/lib/session';
import { attempt, INVALID } from '@/lib/team-action';
import { isUuid } from '@/lib/team-input';
import type { SessionInfo } from '@/lib/types';

/**
 * `DELETE /v1/sessions/{id}`. Revoking the session this browser is using signs
 * it out: the cookie goes and the person lands on sign-in. Which one is current
 * comes from the API, never from the page.
 */
export async function revokeSession(id: string): Promise<ActionResult> {
  if (!isUuid(id)) {
    return INVALID;
  }
  const listed = await attempt(() => api<SessionInfo[]>('/v1/sessions'));
  if (!listed.ok) {
    return listed;
  }
  const current = listed.value.find((s) => s.id === id)?.current === true;
  const result = await attempt(() =>
    api(`/v1/sessions/${encodeURIComponent(id)}`, { method: 'DELETE' }),
  );
  if (!result.ok) {
    return result;
  }
  if (current) {
    await clearSession();
    redirect(routes.signIn);
  }
  revalidatePath(routes.account);
  return { ok: true };
}
