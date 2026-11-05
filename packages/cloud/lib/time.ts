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
