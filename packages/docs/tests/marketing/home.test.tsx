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
    expect(metadata.title).toEqual({ absolute: 'curl-runner — API tests as YAML, run anywhere' });
    expect(metadata.description).toContain('plain YAML');
    expect(metadata.alternates?.canonical).toBe('/');
    expect(metadata.openGraph).toMatchObject({ url: '/', type: 'website' });
    expect(metadata.twitter).toMatchObject({ card: 'summary_large_image' });
  });

  it('embeds SoftwareApplication JSON-LD with no invented claims', async () => {
    const html = await renderHome();
    const match = html.match(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/);
    expect(match).not.toBeNull();
    const data = JSON.parse(match?.[1] ?? '{}');
    expect(data['@type']).toBe('SoftwareApplication');
    expect(data.offers.price).toBe('0');
    expect(data.featureList.length).toBeGreaterThan(0);
    expect(JSON.stringify(data)).not.toMatch(/blazing|fastest/i);
  });
});
