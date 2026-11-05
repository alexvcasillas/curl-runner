'use server';

import { redirect } from 'next/navigation';
import type { ActionResult } from '@/lib/action-result';
import { api } from '@/lib/api';
import { ApiError } from '@/lib/errors';
import { getInvite } from '@/lib/public-data';
import { routes } from '@/lib/routes';
import { attempt } from '@/lib/team-action';
import { isInviteToken } from '@/lib/tokens';

type Joined = { slug: string; name: string; role: string };

/**
 * `POST /v1/invites/{token}/accept`, then the team. Already a member (409) goes to
 * the team too. A used or expired invite (404) goes back to its dead page. A full
 * Free team (402) comes back as a code for the page to explain.
 */
export async function acceptInvite(token: string): Promise<ActionResult> {
  if (!isInviteToken(token)) {
    return { ok: false, code: 'not_found', message: 'This invite doesn’t exist.' };
  }
  const result = await attempt(() =>
    // `optional` so a 401 comes back here, where the way back is known.
    api<Joined>(`/v1/invites/${encodeURIComponent(token)}/accept`, {
      method: 'POST',
      auth: 'optional',
    }),
  );
  if (result.ok) {
    redirect(routes.team(result.value.slug));
  }
  if (result.code === 'unauthorized') {
    // The session expired between render and click.
    redirect(`/sign-in/expired?return_to=${encodeURIComponent(`/invites/${token}`)}`);
  }
  if (result.code === 'conflict') {
    // The invite is still open, so the preview names the team the person is in.
    const invite = await getInvite(token).catch((e: unknown) => {
      if (e instanceof ApiError) {
        return null;
      }
      throw e;
    });
    redirect(invite ? routes.team(invite.team_slug) : routes.home);
  }
  if (result.code === 'not_found') {
    redirect(`/invites/${token}`);
  }
  return result;
}
