// What the settings and admins pages share: wire shapes and wording.
// Safe to import from client components.

import type { Perm } from './admin';

export type SignupMode = 'open' | 'domains' | 'closed';
export type PlanName = 'free' | 'team';
export type BannerTone = 'info' | 'warning';
export type BannerAudience = 'everyone' | 'signed_in';

export type EnvVar = { key: string; value: string | null; secret: boolean };

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
  { value: 'closed', title: 'Closed', desc: 'Only admins. Private preview.' },
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

/**
 * What to show for one environment variable. A secret's value never reaches the
 * page; whether it is set is read off what the server does with it. A secret
 * the server gives no sign of says "Hidden", not a guess.
 */
export function envDisplay(v: EnvVar, ctx: EnvContext): { text: string; mono: boolean } {
  if (!v.secret) {
    return v.value === null ? { text: 'Not set', mono: false } : { text: v.value, mono: true };
  }
  const set = (on: boolean) => ({ text: on ? 'Set' : 'Not set', mono: false });
  switch (v.key) {
    case 'GITHUB_CLIENT_ID':
    case 'GITHUB_CLIENT_SECRET':
      return {
        text: ctx.signIn.includes('github') ? 'Set · GitHub sign-in on' : 'Not set',
        mono: false,
      };
    case 'GOOGLE_CLIENT_ID':
    case 'GOOGLE_CLIENT_SECRET':
      return {
        text: ctx.signIn.includes('google') ? 'Set · Google sign-in on' : 'Not set',
        mono: false,
      };
    case 'STRIPE_SECRET_KEY':
    case 'STRIPE_WEBHOOK_SECRET':
      return set(ctx.billing);
    case 'OPERATOR_EMAILS':
      return {
        text: `${ctx.owners} owner ${ctx.owners === 1 ? 'admin' : 'admins'} who can’t be removed here`,
        mono: false,
      };
    case 'SIGNUP':
    case 'SIGNUP_DOMAINS':
    case 'DEFAULT_PLAN':
      return { text: 'Only seeds the settings above', mono: false };
    default:
      return { text: 'Hidden', mono: false };
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
