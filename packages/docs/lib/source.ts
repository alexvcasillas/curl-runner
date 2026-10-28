import { loader } from 'fumadocs-core/source';
import { docs } from '@/.source/server';

export const SITE_URL = 'https://www.curl-runner.com';

export const source = loader({
  baseUrl: '/docs',
  source: docs.toFumadocsSource(),
});

export type DocsPage = NonNullable<ReturnType<typeof source.getPage>>;

/** The public Markdown URL of a docs page: `/docs` → `/docs.md`, `/docs/a/b` → `/docs/a/b.md`. */
export function getMarkdownUrl(page: Pick<DocsPage, 'url'>): string {
  return `${SITE_URL}${page.url}.md`;
}

/** A docs page as Markdown for LLMs: title, canonical URL, description and processed body. */
export async function getLLMText(page: DocsPage): Promise<string> {
  const { title, description } = page.data;
  const body = await page.data.getText('processed');
  const parts = [`# ${title}`, `URL: ${SITE_URL}${page.url}`];
  if (description) {
    parts.push(description);
  }
  parts.push(body.trim());
  return `${parts.join('\n\n')}\n`;
}

/** The generated Open Graph image of a docs page (app/og/docs/[...slug]/route.tsx). */
export function getDocsOgUrl(page: Pick<DocsPage, 'slugs'>): string {
  return `/og/docs/${[...page.slugs, 'image.png'].join('/')}`;
}
