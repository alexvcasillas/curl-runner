import type { Metadata } from 'next';
import { pageMetadata, SITE_NAME, SITE_URL } from '@/lib/site';

export const DEFAULT_API_URL = 'https://api.curl-runner.com';
export const GALLERY_REVALIDATE = 300;
export const CLOUD_DOCS_PATH = '/docs/features/cloud';

const SLUG = /^[a-z0-9][a-z0-9-]{0,62}$/;

export interface GalleryTeam {
  slug: string;
  name: string;
}

export interface GalleryCollection {
  team: GalleryTeam;
  slug: string;
  name: string;
  description: string;
  requests: number;
  version: number;
  updated_at: string;
  listed_at: string;
  link: string;
}

export interface GalleryList {
  collections: GalleryCollection[];
  next_cursor: string | null;
  total: number;
  teams: number;
}

export interface GalleryInput {
  name: string;
  description: string;
  secret: boolean;
}

export interface GalleryFile {
  path: string;
  text: string;
  requests?: { method: string; name: string }[];
}

export interface GalleryDetail {
  team: GalleryTeam;
  slug: string;
  name: string;
  description: string;
  version: number;
  updated_at: string;
  link: string;
  inputs: GalleryInput[];
  files: GalleryFile[];
}

export type Fetched<T> = { status: 'ok'; data: T } | { status: 'not_found' } | { status: 'down' };

/** Server-side API base, overridable for previews and tests. */
export function apiUrl(): string {
  const base = process.env.CURL_RUNNER_API_URL?.trim() || DEFAULT_API_URL;
  return base.replace(/\/+$/, '');
}

export function isSlug(value: string): boolean {
  return SLUG.test(value);
}

async function getJson<T>(path: string): Promise<Fetched<T>> {
  try {
    const res = await fetch(`${apiUrl()}${path}`, {
      headers: { accept: 'application/json' },
      next: { revalidate: GALLERY_REVALIDATE },
    });
    if (res.status === 404) {
      return { status: 'not_found' };
    }
    if (!res.ok) {
      return { status: 'down' };
    }
    return { status: 'ok', data: (await res.json()) as T };
  } catch {
    return { status: 'down' };
  }
}

export interface ListQuery {
  q?: string;
  team?: string;
  cursor?: string;
}

export async function fetchCollections(query: ListQuery = {}): Promise<Fetched<GalleryList>> {
  const params = new URLSearchParams();
  for (const key of ['q', 'team', 'cursor'] as const) {
    const value = query[key];
    if (value) {
      params.set(key, value);
    }
  }
  const qs = params.toString();
  const res = await getJson<GalleryList>(`/v1/public/collections${qs ? `?${qs}` : ''}`);
  if (res.status === 'ok' && !Array.isArray(res.data?.collections)) {
    return { status: 'down' };
  }
  return res;
}

/** An invalid slug is a 404 without an API call. */
export async function fetchCollection(team: string, slug: string): Promise<Fetched<GalleryDetail>> {
  if (!isSlug(team) || !isSlug(slug)) {
    return { status: 'not_found' };
  }
  const res = await getJson<GalleryDetail>(`/v1/public/collections/${team}/${slug}`);
  if (res.status === 'ok' && !Array.isArray(res.data?.files)) {
    return { status: 'down' };
  }
  return res;
}

/** Every listed collection, page by page. Never throws: a failure ends with what was read. */
export async function fetchAllCollections(maxPages = 20): Promise<GalleryCollection[]> {
  const all: GalleryCollection[] = [];
  let cursor: string | undefined;
  for (let i = 0; i < maxPages; i++) {
    const res = await fetchCollections({ cursor });
    if (res.status !== 'ok') {
      break;
    }
    all.push(...res.data.collections);
    if (!res.data.next_cursor) {
      break;
    }
    cursor = res.data.next_cursor;
  }
  return all;
}

export function relativeTime(iso: string, now: number = Date.now()): string {
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) {
    return '';
  }
  const seconds = Math.max(0, Math.round((now - then) / 1000));
  const units: [number, string][] = [
    [60 * 60 * 24 * 365, 'year'],
    [60 * 60 * 24 * 30, 'month'],
    [60 * 60 * 24, 'day'],
    [60 * 60, 'hour'],
    [60, 'minute'],
  ];
  for (const [size, unit] of units) {
    if (seconds >= size) {
      const n = Math.floor(seconds / size);
      return `${n} ${unit}${n === 1 ? '' : 's'} ago`;
    }
  }
  return 'just now';
}

export function plural(n: number, word: string): string {
  return `${n} ${word}${n === 1 ? '' : 's'}`;
}

export function initial(name: string): string {
  return (name.trim()[0] ?? '?').toUpperCase();
}

export interface RequestRow {
  file: string;
  /** Index of the request inside its file. */
  index: number;
  method: string;
  name: string;
}

export interface RequestGroup {
  /** The files' top directory, or null for files in the root. */
  folder: string | null;
  rows: RequestRow[];
}

export function folderOf(path: string): string | null {
  const i = path.indexOf('/');
  return i > 0 ? path.slice(0, i) : null;
}

/** Request rows grouped by folder, root files first. Files with no requests (config) are skipped. */
export function groupRequests(files: GalleryFile[]): RequestGroup[] {
  const groups = new Map<string | null, RequestRow[]>();
  for (const file of files) {
    const folder = folderOf(file.path);
    (file.requests ?? []).forEach((req, index) => {
      const rows = groups.get(folder) ?? [];
      rows.push({ file: file.path, index, method: req.method, name: req.name });
      groups.set(folder, rows);
    });
  }
  const ordered = [...groups.entries()].map(([folder, rows]) => ({ folder, rows }));
  return [...ordered.filter((g) => g.folder === null), ...ordered.filter((g) => g.folder !== null)];
}

export function folderCount(groups: RequestGroup[]): number {
  return groups.filter((g) => g.folder !== null).length;
}

export const METHOD_COLORS: Record<string, string> = {
  GET: '#52A8FF',
  POST: '#62C073',
  PATCH: '#F5A524',
  DELETE: '#FF6166',
};

/** Colour for a method; muted for anything else. */
export function methodColor(method: string): string {
  return METHOD_COLORS[method.toUpperCase()] ?? '#8A8A8A';
}

/** The studio deep link, built like the Cloud share landing page does. */
export function studioLink(team: string, slug: string, token: string): string {
  const enc = encodeURIComponent;
  return `curl-runner://open/${enc(team)}/${enc(slug)}?share=${enc(token)}&server=${enc(apiUrl())}`;
}

/** The token in a share link: its last path segment. */
export function shareToken(link: string): string {
  const path = link.split(/[?#]/)[0].replace(/\/+$/, '');
  return path.slice(path.lastIndexOf('/') + 1);
}

/** Share links are https URLs without whitespace or shell metacharacters; anything else is not shown. */
export function safeShareLink(link: string): string | null {
  return /^https:\/\/[A-Za-z0-9._~:/?#@!$&()*+,;=%-]+$/.test(link) ? link : null;
}

export function pullCommand(link: string): string {
  return `curl-runner pull ${link}`;
}

export interface Selection {
  row: RequestRow | null;
  file: GalleryFile | null;
}

/** The selected request: `file` (and `req`) from the query, else the first request. */
export function selectRequest(
  files: GalleryFile[],
  groups: RequestGroup[],
  file?: string,
  req?: string,
): Selection {
  const rows = groups.flatMap((g) => g.rows);
  const parsed = Number.parseInt(req ?? '0', 10);
  const index = Number.isNaN(parsed) ? 0 : parsed;
  const row =
    rows.find((r) => r.file === file && r.index === index) ??
    rows.find((r) => r.file === file) ??
    rows[0] ??
    null;
  return { row, file: row ? (files.find((f) => f.path === row.file) ?? null) : null };
}

export function requestHref(base: string, row: RequestRow): string {
  const params = new URLSearchParams({ file: row.file });
  if (row.index > 0) {
    params.set('req', String(row.index));
  }
  return `${base}?${params.toString()}`;
}

export function collectionHref(team: string, slug: string): string {
  return `/collections/${team}/${slug}`;
}

export function listMetadata(): Metadata {
  return pageMetadata({
    title: 'Collections',
    description:
      'Request collections that teams publish with curl-runner. Read the YAML, open one in the studio, or pull it with the CLI.',
    path: '/collections',
  });
}

export function teamMetadata(team: GalleryTeam, count: number): Metadata {
  return pageMetadata({
    title: `${team.name} collections`,
    description: `${plural(count, 'public collection')} from ${team.name}, published with curl-runner.`,
    path: `/collections/${team.slug}`,
  });
}

export function detailMetadata(detail: GalleryDetail): Metadata {
  const title = `${detail.name} by ${detail.team.name} — ${SITE_NAME} collections`;
  const description = detail.description || `${detail.name} by ${detail.team.name}`;
  const url = `${SITE_URL}${collectionHref(detail.team.slug, detail.slug)}`;
  const images = ['/opengraph-image'];
  return {
    title: { absolute: title },
    description,
    alternates: { canonical: url },
    openGraph: { type: 'website', siteName: SITE_NAME, title, description, url, images },
    twitter: { card: 'summary_large_image', title, description, images },
  };
}
