import { prerenderToNodeStream } from 'react-dom/static.node';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import fixture from '../fixtures/release-2.1.0.json';
import fixture230 from '../fixtures/release-2.3.0.json';

const getLatestRelease = vi.fn();
vi.mock('@/lib/github-release', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/lib/github-release')>()),
  getLatestRelease: () => getLatestRelease(),
}));

import DownloadsPage from '@/app/(marketing)/downloads/page';
import { parseRelease } from '@/lib/github-release';

async function render(): Promise<string> {
  const { prelude } = await prerenderToNodeStream(await DownloadsPage());
  const chunks: Buffer[] = [];
  for await (const chunk of prelude) {
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks).toString('utf8');
}

describe('downloads page', () => {
  beforeEach(() => getLatestRelease.mockReset());

  it('renders version, date, binaries and the checksum line', async () => {
    getLatestRelease.mockResolvedValue(parseRelease(fixture));
    const html = await render();
    expect(html).toContain('DOWNLOAD · v2.1.0 · OCT 28, 2026');
    expect(html).toContain('macOS Apple Silicon');
    expect(html).toContain('Windows x64');
    expect(html).toContain('curl-runner-cli-2.1.0-linux-x64.tar.gz');
    expect(html).toContain('shasum -a 256 -c SHA256SUMS.txt');
    expect(html).toContain('SHA256SUMS.txt</a>');
  });

  it('shows the studio dmg link, the cask command and macOS 12+', async () => {
    getLatestRelease.mockResolvedValue(parseRelease(fixture230));
    const html = await render();
    expect(html).toContain('curl-runner-studio-2.3.0.dmg');
    expect(html).toMatch(/href="[^"]*curl-runner-studio-2\.3\.0\.dmg"/);
    expect(html).toContain('brew install --cask alexvcasillas/tap/curl-runner-studio');
    expect(html).toContain('macOS 12');
    expect(html).toContain('31.7 MB');
  });

  it('falls back to the releases link when there is no dmg', async () => {
    getLatestRelease.mockResolvedValue(parseRelease(fixture));
    const html = await render();
    expect(html).not.toMatch(/\.dmg/);
    expect(html).toContain('brew install --cask alexvcasillas/tap/curl-runner-studio');
    expect(html).toContain('Get the studio from GitHub Releases');
  });

  it('falls back to a GitHub Releases link when the release is unavailable', async () => {
    getLatestRelease.mockResolvedValue(null);
    const html = await render();
    expect(html).toContain('Download from GitHub Releases');
    expect(html).not.toMatch(/v\d+\.\d+\.\d+/);
    expect(html).not.toContain('macOS Apple Silicon');
    expect(html).toContain('One binary. Every platform.');
  });
});
