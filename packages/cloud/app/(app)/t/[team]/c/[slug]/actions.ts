'use server';

import { revalidatePath } from 'next/cache';
import { redirect } from 'next/navigation';
import type { ActionResult } from '@/lib/action-result';
import { api } from '@/lib/api';
import { isSlug } from '@/lib/params';
import { routes } from '@/lib/routes';
import { attempt, INVALID, NO_TEAM, validTeam } from '@/lib/team-action';
import type { Source } from '@/lib/types';

const path = (team: string, slug: string) =>
  `/v1/teams/${encodeURIComponent(team)}/collections/${encodeURIComponent(slug)}`;

/** `PATCH …/collections/{slug} {source}`. Any member can switch. */
export async function switchSource(
  team: string,
  slug: string,
  source: Source,
): Promise<ActionResult> {
  if (!validTeam(team)) {
    return NO_TEAM;
  }
  if (!isSlug(slug) || (source !== 'git' && source !== 'cloud')) {
    return INVALID;
  }
  const result = await attempt(() => api(path(team, slug), { method: 'PATCH', json: { source } }));
  if (!result.ok) {
    return result;
  }
  revalidatePath(routes.team(team));
  revalidatePath(`/t/${encodeURIComponent(team)}/c/${encodeURIComponent(slug)}`);
  return { ok: true };
}

/** `DELETE …/collections/{slug}`. Owners only. Success goes back to the list. */
export async function deleteCollection(team: string, slug: string): Promise<ActionResult> {
  if (!validTeam(team)) {
    return NO_TEAM;
  }
  if (!isSlug(slug)) {
    return INVALID;
  }
  const result = await attempt(() => api(path(team, slug), { method: 'DELETE' }));
  if (!result.ok) {
    return result;
  }
  revalidatePath(routes.team(team));
  redirect(routes.team(team));
}
