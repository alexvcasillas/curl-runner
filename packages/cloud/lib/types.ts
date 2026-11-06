// Wire shapes the shell needs, from `GET /v1/me` and `GET /v1/info`.

export type Plan = 'free' | 'team';
export type Role = 'owner' | 'member';

export type MeTeam = { slug: string; name: string; role: Role; plan: Plan; suspended: boolean };

export type Me = {
  user: { id: string; email: string; name: string | null };
  teams: MeTeam[];
  /** A server operator: the account menu shows Admin. */
  admin: boolean;
};

export type Info = {
  version: string;
  sign_in: ('email' | 'github' | 'google')[];
  signup: 'open' | 'domains' | 'closed';
  /** Stripe is configured. Off hides Billing and every plan pill. */
  billing: boolean;
  app_url?: string | null;
};

// Wire shapes for collections, from `/v1/teams/{team}/collections` and below.

export type Source = 'git' | 'cloud';
export type AccessMode = 'public' | 'restricted' | 'off';

export type CollectionSummary = {
  slug: string;
  name: string;
  source: Source;
  access: AccessMode;
  /** Null until the first publish or sync. */
  current_version: number | null;
  /** When the current version was published. */
  updated_at: string | null;
  /** Who published it: `token {name}` or an email. Null before the first. */
  publisher: string | null;
};

export type GitMeta = {
  commit: string | null;
  ref: string | null;
  repo_url: string | null;
};

export type VersionSummary = {
  number: number;
  hash: string;
  published_at: string;
  /** `token {name}` or a member's email. */
  publisher: string;
  git: GitMeta | null;
  file_count: number;
};

export type CollectionInput = {
  name: string;
  description: string | null;
  secret: boolean;
  required: boolean;
  locked: boolean;
  default: string | null;
};

export type VersionDetail = VersionSummary & { inputs: CollectionInput[] };

export type BundleFile = { path: string; text: string };

/** `GET …/share`. The link is the only token value a page may render. */
export type Share = {
  access: AccessMode;
  link: string;
  expires_at: string | null;
  entries: { entry: string; kind: string; invited_at: string; opened: boolean }[];
};

// Wire shapes for tokens, members and billing, from `/v1/teams/{team}/…`.

export type Token = {
  id: string;
  name: string;
  /** The email of whoever made it. Null when that account is gone. */
  created_by: string | null;
  created_at: string;
  expires_at: string | null;
  last_used_at: string | null;
};

export type Member = { user_id: string; email: string; name: string | null; role: Role };

export type PendingInvite = { id: string; email: string; role: Role; expires_at: string };

export type TeamDetail = {
  slug: string;
  name: string;
  plan: Plan;
  role: Role;
  members: Member[];
  pending_invites: PendingInvite[];
  /** An admin suspended the team. The reason stays with the admins. */
  suspended: boolean;
  suspended_at: string | null;
};

export type Price = { unit_amount: number; currency: string };
export type Prices = { monthly: Price; annual: Price };

export type BillingSummary = {
  plan: Plan;
  interval: string | null;
  status: string | null;
  seats: number;
  past_due_since: string | null;
  can_manage: boolean;
  cancel_at: string | null;
  current_period_end: string | null;
  /** `override` while an admin's grant is in force, `stripe` when a subscription backs the plan, else `none`. */
  plan_source: string;
  /** When the grant ends. Null with no grant, or a grant with no end date. */
  override_until: string | null;
  /** A live Stripe subscription exists, whatever `plan_source` says: a grant doesn't stop its charges. */
  has_subscription: boolean;
};

// Wire shapes for the account and shared pages, from `/v1/me/…` and `/v1/sessions`.

export type Identity = { provider: string; email: string; created_at: string };

export type SessionInfo = {
  id: string;
  kind: 'web' | 'device' | string;
  label: string | null;
  created_at: string;
  last_used_at: string;
  expires_at: string;
  current: boolean;
};

export type SharedItem = {
  /** The team's display name. */
  team: string;
  team_slug: string;
  /** The collection's slug. */
  collection: string;
  name: string;
  version: number;
  updated_at: string;
  access: AccessMode;
  /** The share link. The token in it is a share token, not a session. */
  link: string;
};

// Wire shapes for the public pages, from `GET /v1/shared/{token}` and `GET /v1/invites/{token}`.

export type SharedInfo = {
  /** The team's display name. */
  team: string;
  team_slug: string;
  collection: string;
  name: string;
  description: string | null;
  version: number;
  updated_at: string;
  request_count: number;
  access: AccessMode;
};

export type InvitePreview = {
  team_name: string;
  team_slug: string;
  inviter_name: string | null;
  role: Role;
  email: string;
  expires_at: string;
};
