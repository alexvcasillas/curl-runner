import 'server-only';
import { cache } from 'react';
import { api } from './api';
import type { Info, Me } from './types';

/** `GET /v1/me`, once per request. */
export const getMe = cache(() => api<Me>('/v1/me'));

/** `GET /v1/info`, once per request. */
export const getInfo = cache(() => api<Info>('/v1/info', { auth: 'optional' }));
