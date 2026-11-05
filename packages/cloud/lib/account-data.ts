import 'server-only';
import { cache } from 'react';
import { api } from './api';
import type { Identity, SessionInfo, SharedItem } from './types';

/** `GET /v1/me/identities`: the email first, then each linked provider. */
export const getIdentities = cache(() => api<Identity[]>('/v1/me/identities'));

/** `GET /v1/sessions`: the caller's live sessions, newest first. */
export const getSessions = cache(() => api<SessionInfo[]>('/v1/sessions'));

/** `GET /v1/me/shared`: collections shared with the caller by other teams. */
export const getShared = cache(() => api<SharedItem[]>('/v1/me/shared'));
