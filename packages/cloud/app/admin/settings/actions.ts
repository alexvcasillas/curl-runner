'use server';

import { revalidatePath } from 'next/cache';
import type { ActionResult } from '@/lib/action-result';
import { adminRoutes } from '@/lib/admin';
import { requireAdmin } from '@/lib/admin-data';
import {
  type BannerAudience,
  type BannerTone,
  DOMAIN,
  isWebUrl,
  MAX_BANNER,
  MAX_LINK,
  type PlanName,
  type SignupMode,
} from '@/lib/admin-settings';
import { api } from '@/lib/api';
import { attempt, INVALID } from '@/lib/team-action';

const MAX_DOMAINS = 100;
const length = (s: string) => [...s].length;

/** A settings write changes what every page shows (the banner, who can sign up), so refresh the layout too. */
function refresh() {
  revalidatePath(adminRoutes.settings, undefined);
  revalidatePath('/', 'layout');
}

async function put(section: string, json: unknown): Promise<ActionResult> {
  const result = await attempt(() => api(`/v1/admin/settings/${section}`, { method: 'PUT', json }));
  if (!result.ok) {
    return result;
  }
  refresh();
  return { ok: true };
}

const bad = (message: string) => ({ ...INVALID, message });

/** `PUT …/signup`. Domains are trimmed, lower-cased and deduplicated; the API checks each one. */
export async function saveSignup(mode: SignupMode, domains: string[]): Promise<ActionResult> {
  const denied = await requireAdmin('settings');
  if (denied) {
    return denied;
  }
  if (!['open', 'domains', 'closed'].includes(mode) || !Array.isArray(domains)) {
    return INVALID;
  }
  const clean = [
    ...new Set(
      domains
        .filter((d) => typeof d === 'string')
        .map((d) => d.trim().toLowerCase())
        .filter(Boolean),
    ),
  ];
  const badDomain = clean.find((d) => !DOMAIN.test(d));
  if (badDomain) {
    return bad(`${badDomain} is not a valid domain.`);
  }
  if (clean.length > MAX_DOMAINS) {
    return bad(`List at most ${MAX_DOMAINS} domains.`);
  }
  return put('signup', { mode, domains: clean });
}

export type PlansInput = {
  defaultPlan: PlanName;
  memberLimit: number;
  restricted: boolean;
  graceDays: number;
};

/** `PUT …/plans`. Lowering a limit never removes anyone: the API only refuses new invites. */
export async function savePlans(input: PlansInput): Promise<ActionResult> {
  const denied = await requireAdmin('settings');
  if (denied) {
    return denied;
  }
  const { defaultPlan, memberLimit, restricted, graceDays } = input ?? ({} as PlansInput);
  if (!['free', 'team'].includes(defaultPlan)) {
    return INVALID;
  }
  if (!Number.isInteger(memberLimit) || memberLimit < 1 || memberLimit > 100) {
    return bad('The member limit must be a whole number from 1 to 100.');
  }
  if (!Number.isInteger(graceDays) || graceDays < 0 || graceDays > 365) {
    return bad('The grace period must be a whole number of days from 0 to 365.');
  }
  return put('plans', {
    default_plan: defaultPlan,
    free_member_limit: memberLimit,
    free_restricted: restricted === true,
    grace_days: graceDays,
  });
}

export type BannerInput = {
  message: string;
  tone: BannerTone;
  audience: BannerAudience;
  link: string;
};

/** `PUT …/banner`. An empty message clears it; saving bumps the version, so everyone sees it again. */
export async function saveBanner(input: BannerInput): Promise<ActionResult> {
  const denied = await requireAdmin('settings');
  if (denied) {
    return denied;
  }
  const { message, tone, audience, link } = input ?? ({} as BannerInput);
  if (
    typeof message !== 'string' ||
    typeof link !== 'string' ||
    !['info', 'warning'].includes(tone) ||
    !['everyone', 'signed_in'].includes(audience)
  ) {
    return INVALID;
  }
  const text = message.trim();
  const url = link.trim();
  if (length(text) > MAX_BANNER) {
    return bad(`The message must be at most ${MAX_BANNER} characters.`);
  }
  if (url && !isWebUrl(url)) {
    return bad('The link must be an http or https URL.');
  }
  if (length(url) > MAX_LINK) {
    return bad(`The link must be at most ${MAX_LINK} characters.`);
  }
  return put('banner', { message: text || null, tone, audience, link: url || null });
}

/** `PUT …/publishing`. */
export async function setPublishing(paused: boolean): Promise<ActionResult> {
  const denied = await requireAdmin('settings');
  if (denied) {
    return denied;
  }
  return put('publishing', { paused: paused === true });
}

export type TestEmailResult = { ok: true; to: string } | Extract<ActionResult, { ok: false }>;

/** `POST …/test-email`: one email, to the admin who asks. The API limits how often. */
export async function sendTestEmail(): Promise<TestEmailResult> {
  const denied = await requireAdmin('settings');
  if (denied) {
    return denied;
  }
  const result = await attempt(() =>
    api<{ to: string }>('/v1/admin/settings/test-email', { method: 'POST' }),
  );
  if (!result.ok) {
    return result;
  }
  return { ok: true, to: result.value.to };
}
