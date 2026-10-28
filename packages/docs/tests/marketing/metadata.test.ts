import { describe, expect, it } from 'vitest';
import { metadata as alternatives } from '@/app/(marketing)/alternatives/page';
import { metadata as changelog } from '@/app/(marketing)/changelog/page';
import { metadata as downloads } from '@/app/(marketing)/downloads/page';
import { metadata as toolsDatabase } from '@/app/(marketing)/tools-database/page';

const ROUTES = [
  ['/alternatives', alternatives],
  ['/downloads', downloads],
  ['/tools-database', toolsDatabase],
  ['/changelog', changelog],
] as const;

describe('marketing page metadata', () => {
  it.each(ROUTES)('%s has its own title, Open Graph and canonical url', (path, meta) => {
    const title = (meta.title as { absolute: string }).absolute;
    expect(title).toMatch(/ — curl-runner$/);
    expect(title).not.toContain('Documentation');
    expect(meta.openGraph?.title).toBe(title);
    expect(meta.openGraph?.url).toBe(`https://www.curl-runner.com${path}`);
    expect(meta.openGraph?.description).toBe(meta.description);
    expect(meta.twitter?.title).toBe(title);
    expect(meta.alternates?.canonical).toBe(`https://www.curl-runner.com${path}`);
  });
});
