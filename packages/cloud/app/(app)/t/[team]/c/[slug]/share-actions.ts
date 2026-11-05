'use server';

import { revalidatePath } from 'next/cache';
import { api } from '@/lib/api';
import { ApiError, messageFor } from '@/lib/errors';
import { isSlug } from '@/lib/params';
import { routes } from '@/lib/routes';
import type { AccessMode, Share } from '@/lib/types';

export type ShareResult = { ok: true; share: Share } | { ok: false; message: string };

export type SharePatch = {
  access?: AccessMode;
  /** `null` is no expiry. */
  expires_in_days?: number | null;
  reset_link?: boolean;
};

const enc = encodeURIComponent;

async function run(
  team: string,
  slug: string,
  call: (base: string) => Promise<Share>,
): Promise<ShareResult> {
  if (!isSlug(team) || !isSlug(slug)) {
    return { ok: false, message: 'That collection doesn’t exist.' };
  }
  try {
    const share = await call(`/v1/teams/${enc(team)}/collections/${enc(slug)}/share`);
    revalidatePath(routes.sharing(team, slug));
    revalidatePath(routes.collection(team, slug));
    return { ok: true, share };
  } catch (e) {
    if (e instanceof ApiError) {
      return { ok: false, message: messageFor(e) };
    }
    throw e;
  }
}

/** `PATCH …/share`: the access mode, the expiry or a new link. Owners and members alike. */
export async function patchShare(
  team: string,
  slug: string,
  patch: SharePatch,
): Promise<ShareResult> {
  return run(team, slug, (base) => api<Share>(base, { method: 'PATCH', json: patch }));
}

/** `POST …/share/entries`, then the list read back so it matches the server. */
export async function addEntry(
  team: string,
  slug: string,
  entry: string,
  notify: boolean,
): Promise<ShareResult> {
  return run(team, slug, async (base) => {
    await api(`${base}/entries`, { method: 'POST', json: { entry: entry.trim(), notify } });
    return api<Share>(base);
  });
}

/** `DELETE …/share/entries/{entry}`. The entry can be `*@customer.com`, so it is encoded. */
export async function removeEntry(team: string, slug: string, entry: string): Promise<ShareResult> {
  return run(team, slug, async (base) => {
    await api(`${base}/entries/${enc(entry)}`, { method: 'DELETE' });
    return api<Share>(base);
  });
}
