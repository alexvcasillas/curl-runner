// What the settings and admins pages share: wire shapes and wording.
// Safe to import from client components.

import type { Perm } from './admin';

export type SignupMode = 'open' | 'domains' | 'closed';
export type PlanName = 'free' | 'team';
export type BannerTone = 'info' | 'warning';
export type BannerAudience = 'everyone' | 'signed_in';

export type EnvVar = { key: string; value: string | null; secret: boolean; set: boolean };

/** `GET /v1/admin/settings`: the row, plus the environment listing. */
export type SettingsData = {
  signup_mode: SignupMode;
  signup_domains: string[];
  default_plan: PlanName;
  free_member_limit: number;
  free_restricted: boolean;
  grace_days: number;
  banner_message: string | null;
  banner_tone: BannerTone;
  banner_audience: BannerAudience;
  banner_link: string | null;
  banner_version: number;
  publishing_paused: boolean;
  updated_at: string | null;
  updated_by_email: string | null;
  env: EnvVar[];
};

export const SIGNUP_MODES: { value: SignupMode; title: string; desc: string }[] = [
  {
    value: 'open',
    title: 'Open',
    desc: 'Anyone who signs in can create teams. The default at launch.',
  },
  {
    value: 'domains',
    title: 'Allowed domains',
    desc: 'Only people with an email at these domains.',
  },
  { value: 'closed', title: 'Closed', desc: 'Only admins can create teams.' },
];

export const MAX_BANNER = 280;
export const MAX_LINK = 500;
export const PAUSED_COPY = 'Publishing is paused for maintenance. Try again in a few minutes.';

/** What the page knows about the environment beyond the listing. */
export type EnvContext = {
  signIn: ('email' | 'github' | 'google')[];
  billing: boolean;
  /** Owners listed on the Admins page: `OPERATOR_EMAILS` plus the first operator. */
  owners: number;
};

/** Only a web link is ever shown or sent: no `javascript:` or `data:` URLs. */
export function isWebUrl(value: string): boolean {
  try {
    const u = new URL(value);
    return u.protocol === 'https:' || u.protocol === 'http:';
  } catch {
    return false;
  }
}

/** A lower-case hostname with at least one dot, as the API wants a sign-up domain. */
export const DOMAIN =
  /^(?=.{1,253}$)([a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z][a-z0-9-]{0,61}[a-z0-9]$/;

/**
 * What to show for one environment variable. A secret's value never reaches the
 * page: the server says only whether it is set. Notes such as "GitHub sign-in
 * on" come from what the page already knows.
 */
export function envDisplay(v: EnvVar, ctx: EnvContext): { text: string; mono: boolean } {
  if (!v.secret) {
    return v.value === null ? { text: 'Not set', mono: false } : { text: v.value, mono: true };
  }
  const state = v.set ? 'Set' : 'Not set';
  switch (v.key) {
    case 'GITHUB_CLIENT_ID':
    case 'GITHUB_CLIENT_SECRET':
      return {
        text: v.set && ctx.signIn.includes('github') ? 'Set · GitHub sign-in on' : state,
        mono: false,
      };
    case 'GOOGLE_CLIENT_ID':
    case 'GOOGLE_CLIENT_SECRET':
      return {
        text: v.set && ctx.signIn.includes('google') ? 'Set · Google sign-in on' : state,
        mono: false,
      };
    case 'OPERATOR_EMAILS':
      return {
        text: `${state} · ${ctx.owners} owner ${ctx.owners === 1 ? 'admin' : 'admins'} who can’t be removed here`,
        mono: false,
      };
    case 'SIGNUP':
    case 'SIGNUP_DOMAINS':
    case 'DEFAULT_PLAN':
      return { text: `${state} · only seeds the settings above`, mono: false };
    default:
      return { text: state, mono: false };
  }
}

// ---- GET /v1/admin/admins ----

export type AdminRow = {
  /** `null` for an owner who has not signed in yet. */
  user_id: string | null;
  email: string;
  name: string | null;
  role: 'owner' | 'admin';
  permissions: Perm[];
  last_active: string | null;
  added_by: { user_id: string; email: string; name: string | null } | null;
  added_at: string | null;
};
