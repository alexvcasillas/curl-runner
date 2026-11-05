/**
 * The server's rule for an access-list entry (`parse_entry` in the server's
 * share.rs): trimmed and lower-cased, no whitespace or control characters, one
 * `@`. An email has a local part without `*`. A domain entry is `*@domain.tld`,
 * where the domain has a dot and no empty labels.
 */
export function normalizeEntry(raw: string): string | null {
  const entry = raw.trim().toLowerCase();
  // biome-ignore lint/suspicious/noControlCharactersInRegex: the server rejects them too.
  if (/[\s\u0000-\u001f\u007f-\u009f]/.test(entry)) {
    return null;
  }
  const parts = entry.split('@');
  if (parts.length !== 2) {
    return null;
  }
  const [local, domain] = parts;
  if (!domain) {
    return null;
  }
  if (local === '*') {
    return domain.includes('.') && domain.split('.').every(Boolean) ? entry : null;
  }
  return local && !local.includes('*') ? entry : null;
}

export function isAccessEntry(value: string): boolean {
  return normalizeEntry(value) !== null;
}
