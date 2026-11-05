'use server';

import { revalidatePath } from 'next/cache';
import type { ActionResult } from '@/lib/action-result';
import { api } from '@/lib/api';
import type { ErrorCode } from '@/lib/errors';
import { routes } from '@/lib/routes';
import { attempt, INVALID, NO_TEAM, validTeam } from '@/lib/team-action';
import { isExpiry, isUuid, parseTokenName } from '@/lib/team-input';

const enc = encodeURIComponent;

/** The `crt_` value lives only in this result: it goes to the page that asked and nowhere else. */
export type CreateTokenResult =
  | {
      ok: true;
      token: { id: string; name: string; value: string; expiresAt: string | null };
    }
  | { ok: false; message: string; code?: ErrorCode };

type Created = { id: string; name: string; token: string; expires_at: string | null };

/** `POST …/tokens`. `null` is no expiry. */
export async function createToken(
  team: string,
  name: string,
  expiresInDays: number | null,
): Promise<CreateTokenResult> {
  if (!validTeam(team)) {
    return NO_TEAM;
  }
  const clean = parseTokenName(name);
  if (clean === null || !isExpiry(expiresInDays)) {
    return INVALID;
  }
  const result = await attempt(() =>
    api<Created>(`/v1/teams/${enc(team)}/tokens`, {
      method: 'POST',
      json:
        expiresInDays === null ? { name: clean } : { name: clean, expires_in_days: expiresInDays },
    }),
  );
  if (!result.ok) {
    return result;
  }
  revalidatePath(routes.teamTab(team, 'tokens'));
  const { id, token, expires_at } = result.value;
  return {
    ok: true,
    token: { id, name: result.value.name, value: token, expiresAt: expires_at },
  };
}

/** `DELETE …/tokens/{id}`. */
export async function revokeToken(team: string, id: string): Promise<ActionResult> {
  if (!validTeam(team)) {
    return NO_TEAM;
  }
  if (!isUuid(id)) {
    return INVALID;
  }
  const result = await attempt(() =>
    api(`/v1/teams/${enc(team)}/tokens/${enc(id)}`, { method: 'DELETE' }),
  );
  if (!result.ok) {
    return result;
  }
  revalidatePath(routes.teamTab(team, 'tokens'));
  return { ok: true };
}
