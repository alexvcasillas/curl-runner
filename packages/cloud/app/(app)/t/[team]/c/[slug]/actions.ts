'use server';

import { revalidatePath } from 'next/cache';
import { redirect } from 'next/navigation';
import { api } from '@/lib/api';
import { ApiError, messageFor } from '@/lib/errors';
import { routes } from '@/lib/routes';
import type { Source } from '@/lib/types';

export type ActionResult = { ok: true } | { ok: false; message: string };

const path = (team: string, slug: string) =>
  `/v1/teams/${encodeURIComponent(team)}/collections/${encodeURIComponent(slug)}`;

/** `PATCH …/collections/{slug} {source}`. Any member can switch. */
export async function switchSource(
  team: string,
  slug: string,
  source: Source,
): Promise<ActionResult> {
  if (source !== 'git' && source !== 'cloud') {
    return { ok: false, message: 'Choose git or Cloud.' };
  }
  try {
    await api(path(team, slug), { method: 'PATCH', json: { source } });
  } catch (e) {
    if (e instanceof ApiError) {
      return { ok: false, message: messageFor(e) };
    }
    throw e;
  }
  revalidatePath(routes.team(team));
  revalidatePath(`/t/${encodeURIComponent(team)}/c/${encodeURIComponent(slug)}`);
  return { ok: true };
}

/** `DELETE …/collections/{slug}`. Owners only. Success goes back to the list. */
export async function deleteCollection(team: string, slug: string): Promise<ActionResult> {
  try {
    await api(path(team, slug), { method: 'DELETE' });
  } catch (e) {
    if (e instanceof ApiError) {
      return { ok: false, message: messageFor(e) };
    }
    throw e;
  }
  revalidatePath(routes.team(team));
  redirect(routes.team(team));
}
