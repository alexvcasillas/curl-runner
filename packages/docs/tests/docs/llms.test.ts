import { describe, expect, it } from 'vitest';
import { GET as getPageMarkdown } from '@/app/llms.mdx/[[...slug]]/route';
import { GET as getLlmsTxt } from '@/app/llms.txt/route';
import { source } from '@/lib/source';

const SITE = 'https://www.curl-runner.com';

function markdownUrl(url: string): string {
  return `${SITE}${url}.md`;
}

async function getMarkdown(slug: string[]): Promise<Response> {
  return getPageMarkdown(new Request(`${SITE}/llms.mdx/${slug.join('/')}`), {
    params: Promise.resolve({ slug }),
  });
}

describe('/llms.txt', () => {
  it('is plain text listing /docs.md and the .md URL of every sidebar docs page', async () => {
    const res = await getLlmsTxt();
    expect(res.status).toBe(200);
    expect(res.headers.get('content-type')).toMatch(/^text\/plain/);

    const body = await res.text();
    expect(body).toMatch(/^# curl-runner/);
    expect(body).toContain(`${SITE}/docs.md`);

    // The coming-soon placeholder is deliberately not in the sidebar, so it is not indexed.
    const pages = source.getPages().filter((p) => p.url !== '/docs/tutorials/coming-soon');
    expect(pages.length).toBeGreaterThan(0);
    for (const page of pages) {
      expect(body).toContain(markdownUrl(page.url));
      expect(body).toContain(page.data.title);
    }
  });
});

describe('/llms.mdx (served at /docs.md and /docs/<page>.md)', () => {
  it('renders the index page as Markdown with its title and canonical URL', async () => {
    const index = source.getPage([]);
    expect(index).toBeDefined();

    const res = await getMarkdown([]);
    expect(res.status).toBe(200);
    expect(res.headers.get('content-type')).toMatch(/^text\/markdown/);

    const body = await res.text();
    expect(body.startsWith('# ')).toBe(true);
    expect(body.split('\n')[0]).toBe(`# ${index?.data.title}`);
    expect(body).toContain(`${SITE}/docs`);
    // processed Markdown, not raw MDX: no frontmatter block
    expect(body).not.toMatch(/^---$/m);
  });

  it('returns 404 for a slug with no MDX page', async () => {
    await expect(getMarkdown(['no-such-page'])).rejects.toThrow(/NEXT_HTTP_ERROR_FALLBACK;404/);
  });
});
