/** The server's team slug rule: `^[a-z0-9][a-z0-9-]{0,62}$`. */
const SLUG = /^[a-z0-9][a-z0-9-]{0,62}$/;

export function isValidSlug(slug: string): boolean {
  return SLUG.test(slug);
}

/** "Acme Labs!" becomes "acme-labs". Empty when the name has no letters or digits. */
export function slugFromName(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+/, '')
    .slice(0, 63)
    .replace(/-+$/, '');
}
