const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

function day(d: Date): string {
  return `${MONTHS[d.getUTCMonth()]} ${d.getUTCDate()}`;
}

function utcDay(d: Date): number {
  return Math.floor(d.getTime() / DAY);
}

/**
 * "5 minutes ago", "Yesterday", "Nov 2" or "Oct 30, 2025". Calendar days are
 * UTC, so a page renders the same wherever the server runs.
 */
export function relativeTime(iso: string, now: Date = new Date()): string {
  const then = new Date(iso);
  const ago = now.getTime() - then.getTime();
  if (ago < MINUTE) {
    return 'Just now';
  }
  if (ago < HOUR) {
    const m = Math.floor(ago / MINUTE);
    return `${m} ${m === 1 ? 'minute' : 'minutes'} ago`;
  }
  if (ago < DAY && utcDay(then) === utcDay(now)) {
    const h = Math.floor(ago / HOUR);
    return `${h} ${h === 1 ? 'hour' : 'hours'} ago`;
  }
  if (utcDay(now) - utcDay(then) === 1) {
    return 'Yesterday';
  }
  return then.getUTCFullYear() === now.getUTCFullYear()
    ? day(then)
    : `${day(then)}, ${then.getUTCFullYear()}`;
}

/** "Nov 5, 2026, 13:02 UTC", for a tooltip. */
export function absoluteTime(iso: string): string {
  const d = new Date(iso);
  const hh = String(d.getUTCHours()).padStart(2, '0');
  const mm = String(d.getUTCMinutes()).padStart(2, '0');
  return `${day(d)}, ${d.getUTCFullYear()}, ${hh}:${mm} UTC`;
}

/** "Nov 19", for a date this year or next that the page already frames. */
export function shortDate(iso: string): string {
  return day(new Date(iso));
}

/** "Feb 1, 2027". */
export function formatDate(iso: string): string {
  const d = new Date(iso);
  return `${day(d)}, ${d.getUTCFullYear()}`;
}

/** "Nov 2" for a date this year, "Oct 30, 2025" for any other. Calendar days are UTC. */
export function dateThisYear(iso: string, now: Date = new Date()): string {
  return new Date(iso).getUTCFullYear() === now.getUTCFullYear() ? shortDate(iso) : formatDate(iso);
}

/** A token's expiry: "Never", "In 9 days" (flagged when it is two weeks off or less) or a date. */
export function tokenExpiry(
  iso: string | null,
  now: Date = new Date(),
): { label: string; soon: boolean } {
  if (iso === null) {
    return { label: 'Never', soon: false };
  }
  const left = new Date(iso).getTime() - now.getTime();
  const days = Math.max(1, Math.ceil(left / DAY));
  if (days <= 14) {
    return { label: `In ${days} ${days === 1 ? 'day' : 'days'}`, soon: true };
  }
  return { label: formatDate(iso), soon: false };
}

/**
 * "Expires in 5 days", "Expires in 5 hours" or "Expires soon". Days round to the nearest,
 * so a 7-day invite reads "7 days" for its first half day.
 */
export function expiresIn(iso: string, now: Date = new Date()): string {
  const left = new Date(iso).getTime() - now.getTime();
  if (left >= DAY) {
    const d = Math.round(left / DAY);
    return `Expires in ${d} ${d === 1 ? 'day' : 'days'}`;
  }
  if (left >= HOUR) {
    const h = Math.floor(left / HOUR);
    return `Expires in ${h} ${h === 1 ? 'hour' : 'hours'}`;
  }
  return 'Expires soon';
}

/** `relativeTime` for the middle of a sentence: "yesterday", "just now", "Oct 30". */
export function relativeTimeInline(iso: string, now: Date = new Date()): string {
  return relativeTime(iso, now).replace(/^(Just now|Yesterday)/, (w) => w.toLowerCase());
}
