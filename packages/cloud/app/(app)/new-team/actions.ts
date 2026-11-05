'use server';

import { revalidatePath } from 'next/cache';
import { redirect } from 'next/navigation';
import { api } from '@/lib/api';
import { ApiError, messageFor } from '@/lib/errors';
import { routes } from '@/lib/routes';
import { isValidSlug } from '@/lib/slug';

export type CreateTeamResult =
  | { ok: false; kind: 'closed' }
  | { ok: false; kind: 'taken' | 'invalid' | 'error'; message: string };

function text(formData: FormData, name: string): string {
  const v = formData.get(name);
  return typeof v === 'string' ? v.trim() : '';
}

/** `POST /v1/teams`. Success goes to the new team, so only a problem comes back. */
export async function createTeam(formData: FormData): Promise<CreateTeamResult> {
  const name = text(formData, 'name');
  const slug = text(formData, 'slug');
  if (!name) {
    return { ok: false, kind: 'invalid', message: 'Enter a team name.' };
  }
  if (!isValidSlug(slug)) {
    return {
      ok: false,
      kind: 'invalid',
      message: 'Use lower case letters, numbers and dashes, and start with a letter or number.',
    };
  }
  try {
    await api('/v1/teams', { method: 'POST', json: { slug, name } });
  } catch (e) {
    if (e instanceof ApiError) {
      if (e.code === 'signup_closed') {
        return { ok: false, kind: 'closed' };
      }
      if (e.code === 'conflict') {
        return { ok: false, kind: 'taken', message: messageFor(e) };
      }
      if (e.code === 'invalid_request') {
        return { ok: false, kind: 'invalid', message: messageFor(e) };
      }
      return { ok: false, kind: 'error', message: messageFor(e) };
    }
    throw e;
  }
  // The shell lists teams, so every page needs the new one.
  revalidatePath('/', 'layout');
  redirect(routes.team(slug));
}
