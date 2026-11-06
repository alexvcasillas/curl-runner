import 'server-only';
import { notFound } from 'next/navigation';
import { cache } from 'react';
import type { ActionResult } from './action-result';
import { type AdminMe, type AdminOverview, can, type Perm } from './admin';
import type { CollectionStats, CollectionsPage, CollectionsQuery } from './admin-collections';
import type { Audience, AudienceCount, Broadcast, LogPage, LogQuery } from './admin-emails';
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

/** Collections per page on the list. The footer's range follows from it. */
export const COLLECTIONS_PAGE_SIZE = 50;

export function getCollections(query: CollectionsQuery): Promise<CollectionsPage> {
  const p = new URLSearchParams({ limit: String(COLLECTIONS_PAGE_SIZE) });
  if (query.q) {
    p.set('q', query.q);
  }
  if (query.filter !== 'all') {
    p.set('filter', query.filter);
  }
  if (query.cursor) {
    p.set('cursor', query.cursor);
  }
  return api<CollectionsPage>(`/v1/admin/collections?${p}`);
}

/** The list footer's totals: every collection, whatever the filters. */
export function getCollectionStats(): Promise<CollectionStats> {
  return api<CollectionStats>('/v1/admin/collections/stats');
}

/** The latest 100 broadcasts, newest first. */
export async function getBroadcasts(): Promise<Broadcast[]> {
  return (await api<{ items: Broadcast[] }>('/v1/admin/broadcasts')).items;
}

/** One broadcast, or null when the id isn't a uuid or isn't there. */
export async function getBroadcast(id: string): Promise<Broadcast | null> {
  if (!isUuid(id)) {
    return null;
  }
  try {
    return await api<Broadcast>(`/v1/admin/broadcasts/${id}`);
  } catch (err) {
    if (err instanceof ApiError && err.status === 404) {
      return null;
    }
    throw err;
  }
}

export async function getAudienceCount(audience: Audience): Promise<number> {
  const r = await api<AudienceCount>(`/v1/admin/audience-count?audience=${audience}`);
  return r.count;
}

/** Log rows per page. */
export const LOG_PAGE_SIZE = 50;

export function getEmailLog(query: LogQuery): Promise<LogPage> {
  const p = new URLSearchParams({ limit: String(LOG_PAGE_SIZE) });
  if (query.q) {
    p.set('q', query.q);
  }
  if (query.status !== 'all') {
    p.set('status', query.status);
  }
  if (query.kind !== 'all') {
    p.set('kind', query.kind);
  }
  if (query.cursor) {
    p.set('cursor', query.cursor);
  }
  return api<LogPage>(`/v1/admin/emails/log?${p}`);
}
