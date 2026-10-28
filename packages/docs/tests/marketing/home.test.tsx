import { prerenderToNodeStream } from 'react-dom/static.node';
import { describe, expect, it } from 'vitest';
import Home, { metadata } from '@/app/(marketing)/page';
import { HOME_FEATURES } from '@/lib/home-features';

async function renderHome(): Promise<string> {
  const { prelude } = await prerenderToNodeStream(<Home />);
  const chunks: Buffer[] = [];
  for await (const chunk of prelude) {
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks).toString('utf8');
}

describe('homepage', () => {
  it('has the hero headline', async () => {
    const html = await renderHome();
    expect(html).toContain('API tests as YAML.');
    expect(html).toContain('Run them anywhere.');
  });

  it('renders all nine feature titles', async () => {
    expect(HOME_FEATURES).toHaveLength(9);
    const html = await renderHome();
    for (const f of HOME_FEATURES) {
      expect(html).toContain(f.title);
    }
  });

  it('shows the benchmark tiles with the reproduction footnote', async () => {
    const html = await renderHome();
    expect(html).toContain('&lt; 7 ms');
    expect(html).toContain('13.5 MB');
    expect(html).toContain('scripts/bench.sh');
  });

  it('links the badge to the 2.0.0 changelog and captions the terminal', async () => {
    const html = await renderHome();
    expect(html).toContain('href="/changelog#2.0.0"');
    expect(html).toContain('Example request timings are illustrative.');
  });

  it('has no placeholder text', async () => {
    const html = await renderHome();
    expect(html).not.toMatch(/\[\d+\]/);
    expect(html).not.toContain('[VERIFY]');
  });

  it('exports metadata', () => {
    expect(metadata.title).toBe('curl-runner — API tests as YAML, run anywhere');
    expect(metadata.description).toContain('plain YAML');
  });
});
