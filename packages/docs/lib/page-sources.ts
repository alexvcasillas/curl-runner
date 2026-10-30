// Files (relative to this package) each public page's content comes from. They date the
// sitemap's lastmod, which in turn picks the URLs pinged to IndexNow.

/** The CLI changelog: a new release changes the pages that show release data. */
const CLI_CHANGELOG = '../cli/CHANGELOG.md';

/** Marketing routes and their sources. */
export const MARKETING_PAGES: { path: string; sources: string[] }[] = [
  { path: '', sources: ['app/(marketing)/page.tsx', 'lib/home-features.ts'] },
  {
    path: '/alternatives',
    sources: ['app/(marketing)/alternatives/page.tsx', 'lib/alternatives.ts'],
  },
  { path: '/downloads', sources: ['app/(marketing)/downloads/page.tsx', CLI_CHANGELOG] },
  { path: '/tools-database', sources: ['app/(marketing)/tools-database/page.tsx', CLI_CHANGELOG] },
  { path: '/changelog', sources: ['app/(marketing)/changelog/page.tsx', CLI_CHANGELOG] },
];

export const FEATURE_SOURCES = ['app/(marketing)/features/[slug]/page.tsx', 'lib/features-data.ts'];

export const DOCS_CONTENT_DIR = 'content/docs';
