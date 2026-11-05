'use server';

import { revalidatePath } from 'next/cache';
import { redirect } from 'next/navigation';
import type { ActionResult } from '@/lib/action-result';
import { api } from '@/lib/api';
import { routes } from '@/lib/routes';
import { attempt, INVALID, NO_TEAM, validTeam } from '@/lib/team-action';
import { parseTeamName } from '@/lib/team-input';

const enc = encodeURIComponent;

/** `PATCH /v1/teams/{team}`. Owners only; the server checks. */
export async function renameTeam(team: string, name: string): Promise<ActionResult> {
  if (!validTeam(team)) {
    return NO_TEAM;
  }
  const clean = parseTeamName(name);
  if (clean === null) {
    return INVALID;
  }
  const result = await attempt(() =>
    api(`/v1/teams/${enc(team)}`, { method: 'PATCH', json: { name: clean } }),
  );
  if (!result.ok) {
    return result;
  }
  revalidatePath('/', 'layout');
  return { ok: true };
}

/**
 * `DELETE /v1/teams/{team}`. The server cancels the Stripe subscription first.
 * If Stripe can't be reached it answers 503 and keeps the team, and the page
 * shows its message. Otherwise the team is gone and the person goes home.
 */
export async function deleteTeam(team: string): Promise<ActionResult> {
  if (!validTeam(team)) {
    return NO_TEAM;
  }
  const result = await attempt(() => api(`/v1/teams/${enc(team)}`, { method: 'DELETE' }));
  if (!result.ok) {
    return result;
  }
  revalidatePath('/', 'layout');
  redirect(routes.home);
}
