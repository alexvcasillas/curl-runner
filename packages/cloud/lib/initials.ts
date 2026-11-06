function firstAndLast(words: string[]): string {
  return words.length > 1 ? words[0][0] + words[words.length - 1][0] : (words[0]?.[0] ?? '');
}

/**
 * A person's initials: first and last word of the name ("Ada Lovelace" → AL),
 * or for an email, its local part and domain ("jane@globex.example" → JG).
 */
export function initials(nameOrEmail: string): string {
  const value = nameOrEmail.trim();
  const at = value.indexOf('@');
  let out: string;
  if (at > 0) {
    const local = value
      .slice(0, at)
      .split(/[._+-]+/)
      .filter(Boolean);
    out = local.length > 1 ? firstAndLast(local) : (local[0]?.[0] ?? '') + (value[at + 1] ?? '');
  } else {
    out = firstAndLast(value.split(/\s+/).filter(Boolean));
  }
  return out ? out.toUpperCase() : '?';
}

/** A team's one letter. */
export function teamLetter(name: string): string {
  return name.trim()[0]?.toUpperCase() ?? '?';
}
