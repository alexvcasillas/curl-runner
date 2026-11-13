// Every in-app URL, in one place.

export type TeamTab = 'collections' | 'tokens' | 'members' | 'billing' | 'settings';

export const routes = {
  home: '/',
  signIn: '/sign-in',
  signOut: '/sign-out',
  shared: '/shared',
  account: '/account',
  newTeam: '/new-team',
  admin: '/admin',
  team: (slug: string) => `/t/${encodeURIComponent(slug)}`,
  teamTab: (slug: string, tab: TeamTab) =>
    tab === 'collections'
      ? `/t/${encodeURIComponent(slug)}`
      : `/t/${encodeURIComponent(slug)}/${tab}`,
  collection: (team: string, slug: string, tab?: 'inputs' | 'settings') =>
    `/t/${encodeURIComponent(team)}/c/${encodeURIComponent(slug)}${tab ? `?tab=${tab}` : ''}`,
  sharing: (team: string, slug: string) =>
    `/t/${encodeURIComponent(team)}/c/${encodeURIComponent(slug)}/sharing`,
  version: (team: string, slug: string, n: number, compare?: number) =>
    `/t/${encodeURIComponent(team)}/c/${encodeURIComponent(slug)}/versions/${n}${compare === undefined ? '' : `?compare=${compare}`}`,
  download: (team: string, slug: string, n: number) =>
    `/t/${encodeURIComponent(team)}/c/${encodeURIComponent(slug)}/versions/${n}/download`,
};

/** The public gallery page of a listed collection. */
export function galleryUrl(team: string, slug: string): string {
  return `https://www.curl-runner.com/collections/${encodeURIComponent(team)}/${encodeURIComponent(slug)}`;
}

export const external = {
  docs: 'https://www.curl-runner.com/docs',
  download: 'https://www.curl-runner.com/downloads',
  /** The Cloud page of the docs. */
  cloudDocs: 'https://www.curl-runner.com/docs/features/cloud',
  selfHosting: 'https://www.curl-runner.com/docs/self-hosting',
};

const TEAM_TABS: readonly TeamTab[] = ['collections', 'tokens', 'members', 'billing', 'settings'];

/** The team and tab a path belongs to. Collection pages (`/t/acme/c/api`) sit under Collections. */
export function parseLocation(path: string): { team: string | null; tab: TeamTab | null } {
  const [pathname] = path.split(/[?#]/);
  const m = /^\/t\/([^/]+)(?:\/([^/]+))?/.exec(pathname);
  if (!m) {
    return { team: null, tab: null };
  }
  let team = m[1];
  try {
    team = decodeURIComponent(team);
  } catch {
    // Keep the raw segment.
  }
  const seg = m[2] as TeamTab | undefined;
  return {
    team,
    tab: seg && seg !== 'collections' && TEAM_TABS.includes(seg) ? seg : 'collections',
  };
}
