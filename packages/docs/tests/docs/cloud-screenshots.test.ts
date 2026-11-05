import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { CLOUD_SCREENSHOTS } from '@/lib/cloud-screenshots';

const PUBLIC_DIR = join(__dirname, '../../public');
const CLOUD_MDX = readFileSync(join(__dirname, '../../content/docs/features/cloud.mdx'), 'utf8');

function pngSize(file: string) {
  const b = readFileSync(file);
  return { width: b.readUInt32BE(16), height: b.readUInt32BE(20) };
}

describe('cloud screenshots', () => {
  it('has every image in public/, at the size declared', () => {
    for (const [name, shot] of Object.entries(CLOUD_SCREENSHOTS)) {
      const file = join(PUBLIC_DIR, shot.src);
      expect(existsSync(file), name).toBe(true);
      expect(pngSize(file), name).toEqual({ width: shot.width, height: shot.height });
      expect(shot.alt.length, name).toBeGreaterThan(60);
    }
  });

  it('ships no image nothing references, and every one is used in the Cloud page', () => {
    const referenced = new Set(Object.values(CLOUD_SCREENSHOTS).map((s) => s.src));
    for (const file of readdirSync(join(PUBLIC_DIR, 'cloud'))) {
      expect(referenced.has(`/cloud/${file}`), file).toBe(true);
    }
    for (const name of Object.keys(CLOUD_SCREENSHOTS)) {
      expect(CLOUD_MDX, name).toContain(`<CloudScreenshot name="${name}">`);
    }
  });
});
