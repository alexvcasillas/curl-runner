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
