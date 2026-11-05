import 'server-only';
import type { ActionResult } from './action-result';
import { ApiError, type ErrorCode, messageFor } from './errors';
import { isSlug } from './params';

export type Failure = Extract<ActionResult, { ok: false }>;

export const INVALID: Failure = {
  ok: false,
  message: 'Something in the form isn’t right. Check it and try again.',
  code: 'invalid_request',
};

export const NO_TEAM: Failure = {
  ok: false,
  message: 'That team doesn’t exist.',
  code: 'not_found',
};

export const validTeam = isSlug;

/** Runs an API call for a server action. An API error becomes copy and a code; anything else throws. */
export async function attempt<T>(
  call: () => Promise<T>,
  overrides: Partial<Record<ErrorCode, string>> = {},
): Promise<{ ok: true; value: T } | { ok: false; message: string; code: ErrorCode }> {
  try {
    return { ok: true, value: await call() };
  } catch (e) {
    if (e instanceof ApiError) {
      return { ok: false, message: messageFor(e, overrides), code: e.code };
    }
    throw e;
  }
}
