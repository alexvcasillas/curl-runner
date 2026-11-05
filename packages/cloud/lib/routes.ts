// Every in-app URL, in one place.

export type TeamTab = 'collections' | 'tokens' | 'members' | 'billing' | 'settings';

export const routes = {
  home: '/',
  signIn: '/sign-in',
  signOut: '/sign-out',
  shared: '/shared',
  account: '/account',
  newTeam: '/new',
  admin: '/admin',
  team: (slug: string) => `/t/${encodeURIComponent(slug)}`,
  teamTab: (slug: string, tab: TeamTab) =>
    tab === 'collections'
      ? `/t/${encodeURIComponent(slug)}`
      : `/t/${encodeURIComponent(slug)}/${tab}`,
};

export const external = {
  docs: 'https://www.curl-runner.com/docs',
  download: 'https://www.curl-runner.com/downloads',
};
