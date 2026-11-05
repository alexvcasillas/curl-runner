import 'server-only';
import { cache } from 'react';
import { api } from './api';
import { ApiError, messageFor } from './errors';
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
    if (e instanceof ApiError && e.code !== 'unavailable') {
      return null;
    }
    throw e;
  }
});

export type Landing =
  | { kind: 'open'; info: SharedInfo }
  | { kind: 'sign-in' }
  | { kind: 'denied'; message: string }
  | { kind: 'limited'; message: string }
  | { kind: 'dead' };

/**
 * `GET /v1/shared/{token}` as `auth` says. The API answers 401 to any invalid
 * bearer before it looks at the token, so a dead cookie hides a Public link.
 */
function fetchShared(token: string, auth: 'optional' | 'none') {
  return api<SharedInfo>(`/v1/shared/${encodeURIComponent(token)}`, { auth });
}

/**
 * `GET /v1/shared/{token}`, once per request: `generateMetadata` and the page
 * share it. A 401 with a session cookie is tried again without it, so a stale
 * cookie can't hide a Public link or a dead one. Anything but open, sign-in,
 * denied, rate limited and not found is the error page's to show.
 */
export const getLanding = cache(async (token: string): Promise<Landing> => {
  const hadSession = (await getSession()) !== null;
  try {
    try {
      return { kind: 'open', info: await fetchShared(token, 'optional') };
    } catch (e) {
      if (hadSession && e instanceof ApiError && e.code === 'unauthorized') {
        return { kind: 'open', info: await fetchShared(token, 'none') };
      }
      throw e;
    }
  } catch (e) {
    if (e instanceof ApiError) {
      switch (e.code) {
        case 'unauthorized':
          return { kind: 'sign-in' };
        case 'forbidden':
          return { kind: 'denied', message: e.message };
        case 'not_found':
          return { kind: 'dead' };
        case 'rate_limited':
          return { kind: 'limited', message: messageFor(e) };
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
    if (e instanceof ApiError && e.code === 'not_found') {
      return null;
    }
    throw e;
  }
});
