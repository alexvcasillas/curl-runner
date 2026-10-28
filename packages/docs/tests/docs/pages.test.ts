import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
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

// In the order of lib/docs-config.ts (Upgrade sits under CLI Reference there).
const FEATURES = [
  'response-storage',
  'conditional-execution',
  'file-uploads',
  'parallel-execution',
  'response-validation',
  'snapshots',
  'response-diffing',
  'retry-mechanism',
  'watch-mode',
  'dry-run',
  'http2',
  'connection-pooling',
  'output-formats',
  'ci-integration',
  'performance-profiling',
  'yaml-wizard',
  'validate',
  'convert',
  'env-files',
  'upgrade',
].map((slug) => `features/${slug}`);

const REFERENCE_AND_EXAMPLES = [
  'api-reference/request-object',
  'api-reference/response-object',
  'api-reference/global-settings',
  'api-reference/validation-rules',
  'examples/basic',
  'examples/collection',
  'examples/advanced',
];

const MOVED_PAGES = [...GETTING_STARTED, ...FEATURES, ...REFERENCE_AND_EXAMPLES];

type TreeNode = { url?: string; children?: unknown[]; index?: unknown };

// Pages of the sidebar tree in display order (folders expand in place).
function flatten(input: unknown): TreeNode[] {
  const node = input as TreeNode;
  return [
    ...(node.url ? [node] : []),
    ...(node.index ? flatten(node.index) : []),
    ...(node.children ?? []).flatMap(flatten),
  ];
}

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
    for (const path of GETTING_STARTED.filter((p) => !p.includes('/'))) {
      expect(meta.pages, path).toContain(path);
      expect(meta.pages.some((e) => e.startsWith('[') && e.includes(`](/docs/${path})`))).toBe(
        false,
      );
    }
  });
});

describe('features docs', () => {
  it('has no link entries left for features pages in the root meta', () => {
    const meta = JSON.parse(readFileSync(join(CONTENT_DIR, 'meta.json'), 'utf8')) as {
      pages: string[];
    };
    expect(meta.pages.filter((e) => e.includes('](/docs/features/'))).toEqual([]);
  });

  it('lists the features pages in docs-config order in features/meta.json', () => {
    const meta = JSON.parse(readFileSync(join(CONTENT_DIR, 'features/meta.json'), 'utf8')) as {
      pages: string[];
    };
    const expected = FEATURES.map((p) => p.replace('features/', '')).filter((p) => p !== 'upgrade');
    expect(meta.pages).toEqual(expected);
  });

  it('shows Upgrade in the sidebar under CLI Reference, after Environment Variables', () => {
    const urls = flatten(source.pageTree).map((n) => n.url);
    expect(urls).toContain('/docs/features/upgrade');
    expect(urls.indexOf('/docs/features/upgrade')).toBeGreaterThan(
      urls.indexOf('/docs/environment-variables'),
    );
  });

  it('puts the features pages between Security and CLI Commands in the sidebar', () => {
    const urls = flatten(source.pageTree).map((n) => n.url);
    const first = urls.indexOf('/docs/features/response-storage');
    const last = urls.indexOf('/docs/features/env-files');
    expect(urls.indexOf('/docs/security')).toBeLessThan(first);
    expect(last).toBeLessThan(urls.indexOf('/docs/cli-commands'));
    // docs-config order, with Upgrade living under CLI Reference instead
    expect(urls.slice(first, last + 1)).toEqual(
      FEATURES.filter((p) => p !== 'features/upgrade').map((p) => `/docs/${p}`),
    );
  });
});

describe('reference and examples docs', () => {
  it('serves every page from MDX: index + 39 moved pages = 40', () => {
    // 1 index + 12 getting-started/config/cli + 20 features + 7 reference and examples
    expect(source.getPages()).toHaveLength(40);
  });

  it('has removed the old React docs tree and its sidebar config', () => {
    expect(existsSync(join(__dirname, '../../app/(docs)'))).toBe(false);
    expect(existsSync(join(__dirname, '../../lib/docs-config.ts'))).toBe(false);
  });

  it('has no link entries left in the root meta: every entry is a page, folder or separator', () => {
    const meta = JSON.parse(readFileSync(join(CONTENT_DIR, 'meta.json'), 'utf8')) as {
      pages: string[];
    };
    expect(meta.pages.filter((e) => e.startsWith('[') || e.includes('](/docs/'))).toEqual([]);
  });

  it('lists the examples, then the API reference, as the last sidebar entries', () => {
    const urls = flatten(source.pageTree).map((n) => n.url);
    expect(urls.slice(-7)).toEqual([
      '/docs/examples/basic',
      '/docs/examples/collection',
      '/docs/examples/advanced',
      '/docs/api-reference/request-object',
      '/docs/api-reference/response-object',
      '/docs/api-reference/global-settings',
      '/docs/api-reference/validation-rules',
    ]);
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
      // The upgrade page documents the command curl-runner itself runs for bun installs.
      expect(text.replaceAll('bun install -g @curl-runner/cli@latest', '')).not.toMatch(
        /bun install/,
      );
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
