import 'server-only';
import { notFound } from 'next/navigation';
import { cache } from 'react';
import type { ActionResult } from './action-result';
import { type AdminMe, type AdminOverview, can, type Perm } from './admin';
import type { TeamDetail, TeamsPage, TeamsQuery } from './admin-teams';
import type { UserDetail, UsersPage, UsersQuery } from './admin-users';
import { api } from './api';
import { ApiError } from './errors';
import { isSlug } from './params';
import { isUuid } from './team-input';

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

export const FORBIDDEN: Extract<ActionResult, { ok: false }> = {
  ok: false,
  code: 'forbidden',
  message: 'You don’t have permission to do that.',
};

/**
 * The guard at the top of every admin write action. A non-admin gets the 404
 * page; an admin without `perm` gets the problem to show, and nothing is sent.
 * The API enforces both too: this is defence in depth.
 */
export async function requireAdmin(perm?: Perm): Promise<typeof FORBIDDEN | null> {
  const me = await getAdminMe();
  return perm && !can(me.permissions, perm) ? FORBIDDEN : null;
}

export const getOverview = cache(() => api<AdminOverview>('/v1/admin/overview'));

/** Users per page on the list. The footer's range follows from it. */
export const USERS_PAGE_SIZE = 50;

export function getUsers(query: UsersQuery): Promise<UsersPage> {
  const p = new URLSearchParams({ limit: String(USERS_PAGE_SIZE) });
  if (query.q) {
    p.set('q', query.q);
  }
  if (query.filter !== 'all') {
    p.set('filter', query.filter);
  }
  if (query.cursor) {
    p.set('cursor', query.cursor);
  }
  return api<UsersPage>(`/v1/admin/users?${p}`);
}

/** One user. An id that isn't a uuid or isn't there is the 404 page. */
export async function getUser(id: string): Promise<UserDetail> {
  if (!isUuid(id)) {
    notFound();
  }
  try {
    return await api<UserDetail>(`/v1/admin/users/${id}`);
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      notFound();
    }
    throw err;
  }
}

/** Teams per page on the list. The footer's range follows from it. */
export const TEAMS_PAGE_SIZE = 50;

export function getTeams(query: TeamsQuery): Promise<TeamsPage> {
  const p = new URLSearchParams({ limit: String(TEAMS_PAGE_SIZE) });
  if (query.q) {
    p.set('q', query.q);
  }
  if (query.filter !== 'all') {
    p.set('filter', query.filter);
  }
  if (query.cursor) {
    p.set('cursor', query.cursor);
  }
  return api<TeamsPage>(`/v1/admin/teams?${p}`);
}

/** One team. A slug that isn't one or isn't there is the 404 page. */
export async function getTeamDetail(slug: string): Promise<TeamDetail> {
  if (!isSlug(slug)) {
    notFound();
  }
  try {
    return await api<TeamDetail>(`/v1/admin/teams/${encodeURIComponent(slug)}`);
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      notFound();
    }
    throw err;
  }
}
