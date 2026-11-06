'use server';

import { revalidatePath } from 'next/cache';
import type { ActionResult } from '@/lib/action-result';
import { adminRoutes } from '@/lib/admin';
import { requireAdmin } from '@/lib/admin-data';
import { api } from '@/lib/api';
import { isSlug } from '@/lib/params';
import { attempt, INVALID } from '@/lib/team-action';

const enc = encodeURIComponent;

/** The server's limit, checked here too so a long reason never leaves the dashboard. */
const MAX_REASON = 500;

const NO_COLLECTION = {
  ...INVALID,
  code: 'not_found' as const,
  message: 'That collection doesn’t exist.',
};

function valid(team: unknown, slug: unknown): boolean {
  return typeof team === 'string' && typeof slug === 'string' && isSlug(team) && isSlug(slug);
}

/**
 * `POST …/collections/{team}/{slug}/disable-link`. Anyone opening the link gets
 * a 404 and the team can't turn it back on. The team's owners get the reason by
 * email. A second disable is the API's 409.
 */
export async function disableLink(
  team: string,
  slug: string,
  reason: string,
): Promise<ActionResult> {
  const denied = await requireAdmin('teams');
  if (denied) {
    return denied;
  }
  if (!valid(team, slug)) {
    return NO_COLLECTION;
  }
  const clean = typeof reason === 'string' ? reason.trim() : '';
  if (clean === '') {
    return { ...INVALID, message: 'Add a reason. The team’s owners will read it.' };
  }
  if ([...clean].length > MAX_REASON) {
    return { ...INVALID, message: 'The reason is too long.' };
  }
  const result = await attempt(
    () =>
      api(`/v1/admin/collections/${enc(team)}/${enc(slug)}/disable-link`, {
        method: 'POST',
        json: { reason: clean },
      }),
    { conflict: 'That link is already disabled.' },
  );
  if (!result.ok) {
    return result;
  }
  revalidatePath(adminRoutes.collections);
  return { ok: true };
}

/** `POST …/enable-link`: the old link works again. Not disabled is the API's 409. */
export async function enableLink(team: string, slug: string): Promise<ActionResult> {
  const denied = await requireAdmin('teams');
  if (denied) {
    return denied;
  }
  if (!valid(team, slug)) {
    return NO_COLLECTION;
  }
  const result = await attempt(
    () => api(`/v1/admin/collections/${enc(team)}/${enc(slug)}/enable-link`, { method: 'POST' }),
    { conflict: 'That link isn’t disabled.' },
  );
  if (!result.ok) {
    return result;
  }
  revalidatePath(adminRoutes.collections);
  return { ok: true };
}
