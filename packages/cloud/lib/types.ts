// Wire shapes the shell needs, from `GET /v1/me` and `GET /v1/info`.

export type Plan = 'free' | 'team';
export type Role = 'owner' | 'member';

export type MeTeam = { slug: string; name: string; role: Role; plan: Plan };

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
