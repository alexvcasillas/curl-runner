/** The API's error codes (`{error, message}`), plus `unknown` for a code this build doesn't know. */
export const ERROR_CODES = [
  'invalid_request',
  'invalid_bundle',
  'unauthorized',
  'forbidden',
  'not_found',
  'plan_limit',
  'wrong_source',
  'conflict',
  'rate_limited',
  'code_expired',
  'code_invalid',
  'signup_closed',
  'account_suspended',
  'team_suspended',
  'link_disabled',
  'unavailable',
  'unknown',
] as const;

export type ErrorCode = (typeof ERROR_CODES)[number];

export function toErrorCode(value: unknown): ErrorCode {
  return (ERROR_CODES as readonly unknown[]).includes(value) ? (value as ErrorCode) : 'unknown';
}

/** A non-2xx API response. Safe to import from client components. */
export class ApiError extends Error {
  readonly status: number;
  readonly code: ErrorCode;
  /** Seconds until a rate limit resets, from `Retry-After`. */
  readonly retryAfter?: number;

  constructor(status: number, code: ErrorCode, message: string, retryAfter?: number) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.code = code;
    if (retryAfter !== undefined) {
      this.retryAfter = retryAfter;
    }
  }
}

/** The copy for a form the page would never send. One place, so every action says the same. */
/** What a suspended person reads when they try to sign in, on every sign-in path. */
export const ACCOUNT_SUSPENDED = 'This account is suspended. Contact support.';

export const INVALID_FORM = 'Something in the form isn’t right. Check it and try again.';

const FALLBACK: Record<ErrorCode, string> = {
  invalid_request: INVALID_FORM,
  invalid_bundle: 'The collection isn’t valid. Fix it and publish again.',
  unauthorized: 'You were signed out. Sign in again.',
  forbidden: 'Only team owners can do this.',
  not_found: 'It doesn’t exist, or it belongs to a team you’re not in.',
  plan_limit: 'The Free plan doesn’t cover this. Upgrade to Team.',
  wrong_source: 'This collection is edited somewhere else.',
  conflict: 'That’s already taken. Try another.',
  rate_limited: 'Too many tries. Try again in a few minutes.',
  code_expired: 'That code expired. Ask for a new one.',
  code_invalid: 'That code isn’t right. Check it and try again.',
  signup_closed: 'Sign-up is closed on this server.',
  account_suspended: ACCOUNT_SUSPENDED,
  team_suspended: 'This team is suspended, so it can’t publish. Contact support.',
  link_disabled:
    'This share link was turned off by curl-runner and can’t be turned back on. Contact support.',
  unavailable: 'Cloud isn’t answering right now. Try again in a minute.',
  unknown: 'Something went wrong. Try again in a minute.',
};

/** "12 minutes", "1 minute", "30 seconds". */
export function waitFor(seconds: number): string {
  if (seconds < 60) {
    return `${seconds} ${seconds === 1 ? 'second' : 'seconds'}`;
  }
  const minutes = Math.ceil(seconds / 60);
  return `${minutes} ${minutes === 1 ? 'minute' : 'minutes'}`;
}

/**
 * The copy for an API error: the page's override for its code, then the API's
 * own message, then a default. A rate limit with `Retry-After` names the wait.
 */
export function messageFor(
  e: ApiError,
  overrides: Partial<Record<ErrorCode, string>> = {},
): string {
  // One sentence for a suspended account, whatever wording the API sent.
  const override =
    overrides[e.code] ?? (e.code === 'account_suspended' ? ACCOUNT_SUSPENDED : undefined);
  if (override) {
    return override;
  }
  if (e.code === 'rate_limited' && e.retryAfter !== undefined) {
    return `Too many tries. Try again in ${waitFor(e.retryAfter)}.`;
  }
  return e.message || FALLBACK[e.code];
}
