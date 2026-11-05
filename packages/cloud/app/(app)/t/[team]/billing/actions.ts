'use server';

import { redirect } from 'next/navigation';
import type { ActionResult } from '@/lib/action-result';
import { api } from '@/lib/api';
import { type Interval, isInterval, isStripeUrl } from '@/lib/billing';
import { ApiError, messageFor } from '@/lib/errors';
import { INVALID, NO_TEAM, validTeam } from '@/lib/team-action';

const enc = encodeURIComponent;

type Failure = Extract<ActionResult, { ok: false }>;

const NOT_OWNER: Failure = {
  ok: false,
  message: 'Only team owners can change the plan. Ask a team owner.',
  code: 'forbidden',
};

const BAD_URL: Failure = {
  ok: false,
  message: 'Billing sent us somewhere unexpected. Nothing was charged. Try again in a minute.',
  code: 'unavailable',
};

/**
 * Asks the API for a Stripe page and returns its URL, or the copy to show.
 * 403 is a member; 409 and 503 carry the API's own message.
 */
async function stripeUrl(path: string, json?: unknown): Promise<{ url: string } | Failure> {
  try {
    const { url } = await api<{ url: string }>(path, { method: 'POST', json });
    return isStripeUrl(url) ? { url } : BAD_URL;
  } catch (e) {
    if (e instanceof ApiError) {
      if (e.status === 403) {
        return NOT_OWNER;
      }
      return { ok: false, message: messageFor(e), code: e.code };
    }
    throw e;
  }
}

/** `POST …/billing/checkout`, then on to Stripe. Owners only; the server checks. */
export async function startCheckout(team: string, interval: Interval): Promise<ActionResult> {
  if (!validTeam(team)) {
    return NO_TEAM;
  }
  if (!isInterval(interval)) {
    return INVALID;
  }
  const result = await stripeUrl(`/v1/teams/${enc(team)}/billing/checkout`, { interval });
  if ('ok' in result) {
    return result;
  }
  redirect(result.url);
}

/** `POST …/billing/portal`, then on to Stripe's customer portal. */
export async function openPortal(team: string): Promise<ActionResult> {
  if (!validTeam(team)) {
    return NO_TEAM;
  }
  const result = await stripeUrl(`/v1/teams/${enc(team)}/billing/portal`);
  if ('ok' in result) {
    return result;
  }
  redirect(result.url);
}
