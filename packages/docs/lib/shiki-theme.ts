import type { ThemeRegistration } from 'shiki';

/** The Monochrome code theme shared by the marketing pages, the changelog and the docs. */
export const YAML_THEME: ThemeRegistration = {
  name: 'curl-runner-monochrome',
  type: 'dark',
  fg: '#EDEDED',
  bg: 'transparent',
  settings: [
    { settings: { foreground: '#EDEDED', background: 'transparent' } },
    { scope: ['entity.name.tag'], settings: { foreground: '#22D3EE' } },
    { scope: ['string.quoted'], settings: { foreground: '#F5A97F' } },
    { scope: ['comment'], settings: { foreground: '#8A8A8A' } },
  ],
};
