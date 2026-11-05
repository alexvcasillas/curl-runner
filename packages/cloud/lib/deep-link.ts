/**
 * The link that opens a shared collection in the studio:
 * `curl-runner://open/{team}/{collection}?share={token}&server={api url}`.
 * The studio reads the path and `share`; `server` tells it which API to ask.
 */
export function studioLink(
  apiUrl: string,
  teamSlug: string,
  collectionSlug: string,
  token: string,
): string {
  const enc = encodeURIComponent;
  return `curl-runner://open/${enc(teamSlug)}/${enc(collectionSlug)}?share=${enc(token)}&server=${enc(apiUrl)}`;
}

/** The token in a share link: its last path segment. */
export function shareToken(link: string): string {
  const path = link.split(/[?#]/)[0].replace(/\/+$/, '');
  return path.slice(path.lastIndexOf('/') + 1);
}
