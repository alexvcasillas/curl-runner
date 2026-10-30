import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { prerenderToNodeStream } from 'react-dom/static.node';
import { describe, expect, it, vi } from 'vitest';

vi.mock('@/lib/git-dates', () => ({ lastCommitDates: () => new Map() }));

import FeaturePage, { generateMetadata } from '@/app/(marketing)/features/[slug]/page';
import Home from '@/app/(marketing)/page';
import sitemap from '@/app/sitemap';
import { StudioScreenshot } from '@/components/site/Screenshot';
import { SoftwareApplicationSchema } from '@/components/structured-data';
import { getFeatureBySlug } from '@/lib/features-data';
import { SITE_URL, source } from '@/lib/source';
import {
  STUDIO_BUILD_COMMAND,
  STUDIO_HERO,
  STUDIO_SCREENSHOT_NAMES,
  type StudioScreenshotName,
  studioScreenshot,
  studioScreenshots,
} from '@/lib/studio';

const PUBLIC_DIR = join(__dirname, '../../public');
const CONTENT_DIR = join(__dirname, '../../content/docs');

async function render(element: React.ReactElement): Promise<string> {
  const { prelude } = await prerenderToNodeStream(element);
  const chunks: Buffer[] = [];
  for await (const chunk of prelude) {
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks).toString('utf8');
}

/** The studio section of the home page, from its eyebrow to the end of the section. */
function studioSection(html: string): string {
  const start = html.indexOf('aria-labelledby="studio-heading"');
  expect(start).toBeGreaterThan(-1);
  return html.slice(start, html.indexOf('</section>', start));
}

/** PNG and WebP (VP8/VP8L/VP8X) dimensions from the file header. */
function imageSize(file: string): { width: number; height: number } {
  const b = readFileSync(file);
  if (b.toString('ascii', 1, 4) === 'PNG') {
    return { width: b.readUInt32BE(16), height: b.readUInt32BE(20) };
  }
  expect(b.toString('ascii', 8, 12)).toBe('WEBP');
  const chunk = b.toString('ascii', 12, 16);
  if (chunk === 'VP8X') {
    return { width: b.readUIntLE(24, 3) + 1, height: b.readUIntLE(27, 3) + 1 };
  }
  if (chunk === 'VP8L') {
    const bits = b.readUInt32LE(21);
    return { width: (bits & 0x3fff) + 1, height: ((bits >> 14) & 0x3fff) + 1 };
  }
  return { width: b.readUInt16LE(26) & 0x3fff, height: b.readUInt16LE(28) & 0x3fff };
}

describe('home studio section', () => {
  it('labels the studio a preview and links to its page and the 2.2.0 changelog', async () => {
    const section = studioSection(await render(<Home />));
    expect(section).toContain('Studio · Preview');
    expect(section).toContain('href="/features/studio"');
    expect(section).toContain('href="/changelog#2.2.0"');
    expect(section).toContain(STUDIO_BUILD_COMMAND);
    expect(section).toContain('not distributed yet');
  });

  it('serves the animated WebP as is, lazy, sized, with a still for reduced motion', async () => {
    const section = studioSection(await render(<Home />));
    // A plain <img>: next/image would freeze the animation.
    expect(section).not.toContain('/_next/image');
    expect(section).toMatch(
      /<source media="\(prefers-reduced-motion: reduce\)" srcSet="\/studio\/studio-collection-run-still\.webp" type="image\/webp"\/>/,
    );
    const img = section.match(/<img [^>]*>/)?.[0] ?? '';
    expect(img).toContain(`src="${STUDIO_HERO.src}"`);
    expect(img).toContain('width="1440"');
    expect(img).toContain('height="900"');
    expect(img).toContain('loading="lazy"');
    expect(img).toContain('alt="curl-runner studio running');
  });

  it('keeps the still the same shape as the animation, so nothing shifts', () => {
    const anim = imageSize(join(PUBLIC_DIR, STUDIO_HERO.src));
    const still = imageSize(join(PUBLIC_DIR, STUDIO_HERO.still));
    expect(anim).toEqual({ width: STUDIO_HERO.width, height: STUDIO_HERO.height });
    expect(still.width / still.height).toBeCloseTo(anim.width / anim.height, 3);
  });
});

describe('studio feature page', () => {
  it('is a preview feature with every screenshot', () => {
    const studio = getFeatureBySlug('studio');
    expect(studio?.preview).toBe(true);
    expect(studio?.screenshots).toEqual(studioScreenshots());
    expect(studio?.screenshots?.length).toBeGreaterThanOrEqual(4);
    expect(studio?.screenshots?.length).toBeLessThanOrEqual(6);
  });

  it('says it is a preview and how to build it, never that there is a download', async () => {
    const html = await render(<FeaturePage params={Promise.resolve({ slug: 'studio' })} />);
    expect(html).toContain('Studio · Preview');
    expect(html).toContain('A preview, built from source');
    expect(html).toContain(STUDIO_BUILD_COMMAND);
    expect(html).toContain('href="/changelog#2.2.0"');
    expect(html).toContain('href="/docs/features/studio"');
    // The header links Downloads site-wide; the page itself must not offer an install.
    const article = html.slice(html.indexOf('<article'), html.indexOf('</article>'));
    expect(article).not.toContain('href="/downloads"');
    expect(article).not.toMatch(/Install curl-runner|Download/);
  });

  it('renders each screenshot through next/image with its alt text and caption', async () => {
    const html = await render(<FeaturePage params={Promise.resolve({ slug: 'studio' })} />);
    for (const shot of studioScreenshots()) {
      const escaped = shot.alt.replaceAll('"', '&quot;');
      expect(html).toContain(`alt="${escaped}"`);
      expect(html).toContain(`/_next/image?url=${encodeURIComponent(shot.src)}`);
      expect(html).toContain(shot.title);
    }
  });

  it('marks the studio as a preview in the features sidebar', async () => {
    const html = await render(
      <FeaturePage params={Promise.resolve({ slug: 'retry-mechanism' })} />,
    );
    expect(html).toMatch(/href="\/features\/studio"[^>]*>Studio<span[^>]*>preview<\/span>/);
  });

  it('has its own title, description and canonical URL', async () => {
    const meta = await generateMetadata({ params: Promise.resolve({ slug: 'studio' }) });
    expect(meta.title).toEqual({
      absolute: 'curl-runner studio: a macOS app for YAML API tests (preview) — curl-runner',
    });
    expect(meta.alternates?.canonical).toBe(`${SITE_URL}/features/studio`);
    expect(meta.openGraph).toMatchObject({ url: `${SITE_URL}/features/studio` });
  });

  it('is in the sitemap', () => {
    expect(sitemap().map((e) => e.url)).toContain(`${SITE_URL}/features/studio`);
    expect(sitemap().map((e) => e.url)).toContain(`${SITE_URL}/docs/features/studio`);
  });

  it('stays out of the CLI’s JSON-LD feature list', async () => {
    const html = await render(<SoftwareApplicationSchema />);
    const data = JSON.parse(html.match(/>(\{.*\})</)?.[1] ?? '{}');
    expect(data.featureList).toContain('Retry Mechanism');
    expect(data.featureList).not.toContain('Studio');
  });
});

describe('studio images', () => {
  it('has every referenced image in public/, at the size the markup declares', () => {
    for (const shot of studioScreenshots()) {
      const file = join(PUBLIC_DIR, shot.src);
      expect(existsSync(file), shot.src).toBe(true);
      expect(imageSize(file), shot.src).toEqual({ width: shot.width, height: shot.height });
      expect(shot.width, shot.src).toBeLessThanOrEqual(2400);
      expect(shot.alt.length, shot.src).toBeGreaterThan(60);
    }
    expect(existsSync(join(PUBLIC_DIR, STUDIO_HERO.src))).toBe(true);
    expect(existsSync(join(PUBLIC_DIR, STUDIO_HERO.still))).toBe(true);
  });

  it('ships no image nothing references', () => {
    const referenced = new Set([
      ...studioScreenshots().map((s) => s.src),
      STUDIO_HERO.src,
      STUDIO_HERO.still,
    ]);
    for (const name of readdirSync(join(PUBLIC_DIR, 'studio'))) {
      expect(referenced.has(`/studio/${name}`), name).toBe(true);
    }
  });
});

describe('studio docs', () => {
  const mdx = (path: string) => readFileSync(join(CONTENT_DIR, `${path}.mdx`), 'utf8');

  it('has a docs page that says it is a preview and how to build it', () => {
    expect(source.getPage(['features', 'studio'])?.data.title).toBe('Studio (preview)');
    const text = mdx('features/studio');
    expect(text).toContain(STUDIO_BUILD_COMMAND);
    expect(text).toContain('/changelog#2.2.0');
    expect(text).toMatch(/no signed build, Homebrew formula or download/);
  });

  it('captions docs screenshots as the studio preview, linking to its page', async () => {
    const html = await render(
      <StudioScreenshot name="environments">The Staging environment.</StudioScreenshot>,
    );
    expect(html).toContain('<figure');
    expect(html).toContain(`alt="${studioScreenshot('environments').alt}"`);
    expect(html.replaceAll('<!-- -->', '')).toMatch(
      /The Staging environment\. Shown in <a [^>]*href="\/docs\/features\/studio"[^>]*>curl-runner studio<\/a>, a macOS preview\./,
    );
    const own = await render(
      <StudioScreenshot name="environments" onStudioPage>
        The Staging environment.
      </StudioScreenshot>,
    );
    expect(own).not.toContain('Shown in');
  });

  it('only uses screenshots that exist, on the pages that document them', () => {
    const expected: Record<string, string[]> = {
      'features/validate': ['editor-quick-fix'],
      'features/response-validation': ['failed-request'],
      'features/retry-mechanism': ['failed-request'],
      'features/env-files': ['environments'],
      'features/studio': [...STUDIO_SCREENSHOT_NAMES],
    };
    for (const [page, names] of Object.entries(expected)) {
      const used = [...mdx(page).matchAll(/<StudioScreenshot name="([^"]+)"/g)].map((m) => m[1]);
      expect(used.toSorted(), page).toEqual(names.toSorted());
      for (const name of used) {
        expect(studioScreenshot(name as StudioScreenshotName), name).toBeDefined();
      }
    }
  });
});
