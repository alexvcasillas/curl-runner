import 'server-only';
import { notFound } from 'next/navigation';
import { cache } from 'react';
import { api } from './api';
import { getMe } from './app-data';
import { ApiError } from './errors';
import type {
  BundleFile,
  CollectionSummary,
  MeTeam,
  Share,
  VersionDetail,
  VersionSummary,
} from './types';

const enc = encodeURIComponent;
const base = (team: string) => `/v1/teams/${enc(team)}/collections`;
const one = (team: string, slug: string) => `${base(team)}/${enc(slug)}`;

/** `GET …/collections`, once per request. */
export const getCollections = cache((team: string) => api<CollectionSummary[]>(base(team)));

/** `GET …/versions`, newest first. */
export const getVersions = cache((team: string, slug: string) =>
  api<VersionSummary[]>(`${one(team, slug)}/versions`),
);

export const getVersion = cache((team: string, slug: string, n: number) =>
  api<VersionDetail>(`${one(team, slug)}/versions/${n}`),
);

export const getFiles = cache((team: string, slug: string, n: number) =>
  api<BundleFile[]>(`${one(team, slug)}/versions/${n}/files`),
);

/** `GET …/share`. Null when it can't be read, so a page can leave the link out. */
export const getShare = cache(async (team: string, slug: string): Promise<Share | null> => {
  try {
    return await api<Share>(`${one(team, slug)}/share`);
  } catch (e) {
    if (e instanceof ApiError) {
      return null;
    }
    throw e;
  }
});

/** The team from the URL, if the person is in it. Anything else is a 404. */
export async function requireTeam(team: string): Promise<MeTeam> {
  const me = await getMe();
  const current = me.teams.find((t) => t.slug === team);
  if (!current) {
    notFound();
  }
  return current;
}

export async function requireCollection(team: string, slug: string): Promise<CollectionSummary> {
  const collection = (await getCollections(team)).find((c) => c.slug === slug);
  if (!collection) {
    notFound();
  }
  return collection;
}
