import { normalizeEntry } from './access-entry';
import type { Role } from './types';

/** The server's longest email, and the longest token or team name. */
const MAX_EMAIL = 254;
const MAX_NAME = 100;

/** An invite email: an address, not a `*@domain` entry. Trimmed and lower-cased like the server. */
export function normalizeEmail(raw: unknown): string | null {
  if (typeof raw !== 'string') {
    return null;
  }
  const entry = normalizeEntry(raw);
  if (entry === null || entry.startsWith('*@') || entry.length > MAX_EMAIL) {
    return null;
  }
  return entry;
}

export function isRole(value: unknown): value is Role {
  return value === 'owner' || value === 'member';
}

/** The expiries the create dialog offers: days, or null for never. */
export const EXPIRY_DAYS = [null, 30, 90, 365] as const;

export function isExpiry(value: unknown): value is (typeof EXPIRY_DAYS)[number] {
  return (EXPIRY_DAYS as readonly unknown[]).includes(value);
}

export function isUuid(value: unknown): value is string {
  return (
    typeof value === 'string' &&
    /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)
  );
}

function parseName(raw: unknown): string | null {
  if (typeof raw !== 'string') {
    return null;
  }
  const name = raw.trim();
  return name && [...name].length <= MAX_NAME ? name : null;
}

export const parseTokenName = parseName;
export const parseTeamName = parseName;
