// What every admin list shares for paging: the cursor trail in the URL and its checks.
// Safe to import from client components.

/** The server's longest `q`. */
export const MAX_QUERY = 200;
/** The first page in a trail: it has no cursor. */
export const FIRST_PAGE = '-';
/** The most pages back a trail remembers, so a URL can't grow without end. */
export const MAX_TRAIL = 50;
const CURSOR = /^[A-Za-z0-9_-]{1,200}$/;

export type RawParams = Record<string, string | string[] | undefined>;

export const one = (v: string | string[] | undefined) => (typeof v === 'string' ? v : undefined);

/** `q`, trimmed and cut to what the server takes. */
export function parseSearch(v: string | string[] | undefined): string {
  return [...(one(v) ?? '').trim()].slice(0, MAX_QUERY).join('');
}

/** A cursor that looks like one, else nothing. */
export function parseCursor(v: string | string[] | undefined): string | undefined {
  const cursor = one(v);
  return cursor && CURSOR.test(cursor) ? cursor : undefined;
}

/** The `trail` param: dot-separated cursors, bad entries dropped, at most `MAX_TRAIL`. */
export function parseTrail(v: string | string[] | undefined): string[] {
  return (one(v) ?? '')
    .split('.')
    .filter((c) => CURSOR.test(c))
    .slice(0, MAX_TRAIL);
}

export function serialiseTrail(trail: string[]): string {
  return trail.join('.');
}

/** Sets `cursor` and `trail` on a list's params. */
export function pagingParams(p: URLSearchParams, page: { cursor?: string; trail: string[] }) {
  if (page.cursor) {
    p.set('cursor', page.cursor);
  }
  if (page.trail.length > 0) {
    p.set('trail', serialiseTrail(page.trail));
  }
  return p;
}

export type PageRef = { cursor: string | undefined; trail: string[] };

/** Where Previous goes: the last page in the trail, which has no cursor if it was the first. */
export function previousPage(trail: string[]): PageRef {
  const last = trail[trail.length - 1];
  return { cursor: last === FIRST_PAGE ? undefined : last, trail: trail.slice(0, -1) };
}

/** Where Next goes: the API's cursor, with this page added to the trail. */
export function nextPage(
  trail: string[],
  current: string | undefined,
  nextCursor: string,
): PageRef {
  return { cursor: nextCursor, trail: [...trail, current ?? FIRST_PAGE].slice(-MAX_TRAIL) };
}
