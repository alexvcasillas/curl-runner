import 'server-only';
import { cache } from 'react';
import { api } from './api';
import { ApiError } from './errors';
import { getSession } from './session';
import type { InvitePreview, Me, SharedInfo } from './types';

/** The email of whoever is signed in, or null. A stale cookie is signed out. */
export const getViewer = cache(async (): Promise<string | null> => {
  if (!(await getSession())) {
    return null;
  }
  try {
    return (await api<Me>('/v1/me', { auth: 'optional' })).user.email;
  } catch (e) {
    if (e instanceof ApiError && e.status < 500) {
      return null;
    }
    throw e;
  }
});

export type Landing =
  | { kind: 'open'; info: SharedInfo }
  | { kind: 'sign-in' }
  | { kind: 'denied'; message: string }
  | { kind: 'dead' };

/**
 * `GET /v1/shared/{token}`, once per request: `generateMetadata` and the page
 * share it. Anything but 200, 401, 403 and 404 is the error page's to show.
 */
export const getLanding = cache(async (token: string): Promise<Landing> => {
  try {
    const info = await api<SharedInfo>(`/v1/shared/${encodeURIComponent(token)}`, {
      auth: 'optional',
    });
    return { kind: 'open', info };
  } catch (e) {
    if (e instanceof ApiError) {
      if (e.status === 401) {
        return { kind: 'sign-in' };
      }
      if (e.status === 403) {
        return { kind: 'denied', message: e.message };
      }
      if (e.status === 404) {
        return { kind: 'dead' };
      }
    }
    throw e;
  }
});

/** `GET /v1/invites/{token}`: needs no session. Null for an expired or used invite. */
export const getInvite = cache(async (token: string): Promise<InvitePreview | null> => {
  try {
    return await api<InvitePreview>(`/v1/invites/${encodeURIComponent(token)}`, { auth: 'none' });
  } catch (e) {
    if (e instanceof ApiError && e.status === 404) {
      return null;
    }
    throw e;
  }
});
