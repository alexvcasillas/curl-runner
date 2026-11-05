/** The server's rule for team and collection slugs. */
export const SLUG = /^[a-z0-9][a-z0-9-]{0,62}$/;

export function isSlug(value: string): boolean {
  return SLUG.test(value);
}
