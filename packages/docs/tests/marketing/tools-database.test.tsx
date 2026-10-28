import { prerenderToNodeStream } from 'react-dom/static.node';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import fixture from '../fixtures/release-2.1.0.json';

const getLatestRelease = vi.fn();
vi.mock('@/lib/github-release', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/github-release')>()),
  getLatestRelease: () => getLatestRelease(),
}));

import ToolsDatabasePage from '@/app/(marketing)/tools-database/page';
import { parseRelease } from '@/lib/github-release';

async function render(): Promise<string> {
  const { prelude } = await prerenderToNodeStream(await ToolsDatabasePage());
  const chunks: Buffer[] = [];
  for await (const chunk of prelude) {
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks).toString('utf8');
}

const TERMS = [
  'Category',
  'Latest version',
  'Written in',
  'Runtime needed',
  'Platforms',
  'Install',
  'Test format',
  'Protocols',
  'CI use',
  'License',
  'Source',
  'Package names',
];

describe('tools-database fact sheet', () => {
  beforeEach(() => getLatestRelease.mockReset());

  it('renders a <dl> with the 12 facts', async () => {
    getLatestRelease.mockResolvedValue(parseRelease(fixture));
    const html = await render();
    expect(html).toContain('<dl');
    expect((html.match(/<dt/g) ?? []).length).toBe(12);
    expect((html.match(/<dd/g) ?? []).length).toBe(12);
    for (const term of TERMS) {
      expect(html).toContain(`>${term}</dt>`);
    }
    expect(html).toContain('alexvcasillas/tap/curl-runner');
    expect(html).toContain('MIT');
  });

  it('shows the live version, or falls back to a Downloads link', async () => {
    getLatestRelease.mockResolvedValue(parseRelease(fixture));
    expect(await render()).toContain('2.1.0');
    getLatestRelease.mockResolvedValue(null);
    const html = await render();
    expect(html).toContain('see <a');
    expect(html).toContain('>Downloads</a></dd>');
    expect(html).toContain('href="/downloads"');
    expect(html).not.toContain('live from GitHub Releases');
  });

  it('has no hidden content and no unverifiable claims', async () => {
    getLatestRelease.mockResolvedValue(null);
    const html = await render();
    expect(html).not.toMatch(
      /<[^>]*\shidden(=|\s|>)|display:\s*none|class="[^"]*(?:^|\s)hidden(?:\s|")/i,
    );
    expect(html).not.toMatch(/blazing|load testing|documentation generation/i);
  });
});
