import 'server-only';
import { cache } from 'react';
import { api } from './api';
import type { Info, Me } from './types';

/** `GET /v1/me`, once per request. */
export const getMe = cache(() => api<Me>('/v1/me'));

/** `GET /v1/info`, once per request. */
export const getInfo = cache(() => api<Info>('/v1/info', { auth: 'optional' }));

/**
 * `GET /v1/info` as nobody, once per request. The public `/s/` and `/invites/`
 * pages use it so a banner meant for signed-in people never shows to a viewer
 * who happens to carry a session: only `everyone` banners come back.
 */
export const getPublicInfo = cache(() => api<Info>('/v1/info', { auth: 'none' }));
