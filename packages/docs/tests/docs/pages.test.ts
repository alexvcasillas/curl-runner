import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';
import { source } from '@/lib/source';
import nextConfig from '@/next.config';

const CONTENT_DIR = join(__dirname, '../../content/docs');

// Pages moved from React to MDX, grouped by the task that moved them.
const GETTING_STARTED = [
  'installation',
  'quick-start',
  'yaml-structure',
  'variables',
  'global-settings',
  'environment-variables',
  'security',
  'use-cases',
  'tutorials',
  'tutorials/coming-soon',
  'cli-commands',
  'cli-options',
];

const MOVED_PAGES = [...GETTING_STARTED];

function mdxFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? mdxFiles(path) : path.endsWith('.mdx') ? [path] : [];
  });
}

describe('docs pages', () => {
  const urls = source.getPages().map((page) => page.url);

  it.each(MOVED_PAGES)('serves /docs/%s from MDX at the same URL', (path) => {
    expect(urls).toContain(`/docs/${path}`);
  });

  it('gives every MDX page a title and a description', () => {
    for (const page of source.getPages()) {
      expect(page.data.title, page.url).toBeTruthy();
      expect(page.data.description, page.url).toBeTruthy();
    }
  });

  it('lists each moved page in the sidebar tree, not as a link entry', () => {
    const meta = JSON.parse(readFileSync(join(CONTENT_DIR, 'meta.json'), 'utf8')) as {
      pages: string[];
    };
    for (const path of MOVED_PAGES.filter((p) => !p.includes('/'))) {
      expect(meta.pages, path).toContain(path);
      expect(meta.pages.some((e) => e.startsWith('[') && e.includes(`](/docs/${path})`))).toBe(
        false,
      );
    }
  });
});

describe('MDX content for 2.x', () => {
  const files = mdxFiles(CONTENT_DIR);

  it('has files to check', () => {
    expect(files.length).toBeGreaterThan(MOVED_PAGES.length);
  });

  it.each(files.map((f) => [relative(CONTENT_DIR, f), f]))(
    '%s has no Bun-era `validation:` key or `bun install` step',
    (_name, file) => {
      const text = readFileSync(file, 'utf8');
      expect(text).not.toMatch(/^\s*validation:/m);
      // `bun add -g` is fine in the npm-family install list; `bun install` was the old step.
      expect(text).not.toMatch(/bun install/);
    },
  );
});

describe('markdown rewrites', () => {
  it('serves /docs/index.md and /docs.md from the index page', async () => {
    const rewrites = await nextConfig.rewrites?.();
    const list = Array.isArray(rewrites) ? rewrites : (rewrites?.beforeFiles ?? []);
    const dest = (src: string) => list.find((r) => r.source === src)?.destination;
    expect(dest('/docs.md')).toBe('/llms.mdx');
    expect(dest('/docs/index.md')).toBe('/llms.mdx');
    // the generic rule must not shadow the index rule
    const order = list.map((r) => r.source);
    expect(order.indexOf('/docs/index.md')).toBeLessThan(order.indexOf('/docs/:path*.md'));
  });
});
