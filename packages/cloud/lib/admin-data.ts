import 'server-only';
import { notFound } from 'next/navigation';
import { cache } from 'react';
import type { AdminMe, AdminOverview } from './admin';
import { api } from './api';
import { ApiError } from './errors';

/**
 * `GET /v1/admin/me`, once per request. A non-admin gets the API's 404, which
 * becomes the dashboard's own 404 page: nothing says the panel exists.
 */
export const getAdminMe = cache(async (): Promise<AdminMe> => {
  try {
    return await api<AdminMe>('/v1/admin/me');
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      notFound();
    }
    throw err;
  }
});

export const getOverview = cache(() => api<AdminOverview>('/v1/admin/overview'));
