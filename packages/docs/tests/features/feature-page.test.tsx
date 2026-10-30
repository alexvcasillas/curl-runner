import { prerenderToNodeStream } from 'react-dom/static.node';
import { describe, expect, it } from 'vitest';
import FeaturePage, {
  generateMetadata,
  generateStaticParams,
} from '@/app/(marketing)/features/[slug]/page';
import { getAllFeatures } from '@/lib/features-data';

async function render(slug: string): Promise<string> {
  const { prelude } = await prerenderToNodeStream(
    <FeaturePage params={Promise.resolve({ slug })} />,
  );
  const chunks: Buffer[] = [];
  for await (const chunk of prelude) {
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks).toString('utf8');
}

describe('feature page', () => {
  it('prerenders every feature', () => {
    expect(generateStaticParams()).toEqual(getAllFeatures().map((f) => ({ slug: f.slug })));
  });

  it('renders the curl --retry section on the retry page', async () => {
    const html = await render('retry-mechanism');
    expect(html).toContain('From curl --retry');
    expect(html).toContain('href="https://curl.se/docs/manpage.html#--retry"');
    expect(html).toContain('retryableStatuses');
  });

  it('renders the retry mechanism page', async () => {
    const html = await render('retry-mechanism');
    expect(html).toContain('Flaky networks shouldn’t fail your suite.');
    expect(html).toContain('retry.yaml');
    expect(html).toContain('href="/docs/features/retry-mechanism"');
    expect(html).toContain('href="/downloads"');
    expect(html).toContain('Exponential backoff');
    expect(html).toContain('Good for');
    expect(html).toContain('Rate-limited APIs that answer 429');
  });

  it('lists every feature in the sidebar and marks the current one', async () => {
    const html = await render('retry-mechanism');
    for (const f of getAllFeatures()) {
      expect(html).toContain(`href="/features/${f.slug}"`);
    }
    expect(html).toMatch(
      /aria-current="page"[^>]*href="\/features\/retry-mechanism"|href="\/features\/retry-mechanism"[^>]*aria-current="page"/,
    );
  });

  it('links to the previous and next features', async () => {
    const html = await render('retry-mechanism');
    expect(html).toContain('Response Validation');
    expect(html).toMatch(/rel="prev"|Previous/);
    expect(html).toContain('Watch Mode');
  });

  it('omits the previous card on the first feature and the next card on the last', async () => {
    expect(await render('parallel-execution')).not.toContain('Previous');
    expect(await render('studio')).not.toContain('>Next<');
  });

  it('builds metadata from the feature', async () => {
    const meta = await generateMetadata({ params: Promise.resolve({ slug: 'retry-mechanism' }) });
    expect(meta.title).toEqual({
      absolute: 'Retry failed curl requests with backoff — curl-runner',
    });
    expect(meta.alternates?.canonical).toBe('https://www.curl-runner.com/features/retry-mechanism');
  });
});
