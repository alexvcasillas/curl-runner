import { prerenderToNodeStream } from 'react-dom/static.node';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import CollectionNotFound from '@/app/(marketing)/collections/[team]/[slug]/not-found';
import CollectionPage, {
  generateMetadata as detailMeta,
} from '@/app/(marketing)/collections/[team]/[slug]/page';
import TeamPage from '@/app/(marketing)/collections/[team]/page';
import CollectionsPage from '@/app/(marketing)/collections/page';
import {
  folderOf,
  groupRequests,
  relativeTime,
  safeShareLink,
  shareToken,
  studioLink,
} from '@/lib/gallery';
import { DETAIL, json, LIST, PAYMENTS, USERS } from './fixtures';

async function render(element: React.ReactElement): Promise<string> {
  const { prelude } = await prerenderToNodeStream(element);
  const chunks: Buffer[] = [];
  for await (const chunk of prelude) {
    chunks.push(Buffer.from(chunk));
  }
  return Buffer.concat(chunks).toString('utf8').replaceAll('<!-- -->', '');
}

const fetchMock = vi.fn();

function called(): string[] {
  return fetchMock.mock.calls.map((c) => String(c[0]));
}

async function isNotFound(promise: Promise<unknown>): Promise<boolean> {
  try {
    await promise;
    return false;
  } catch (error) {
    return String((error as { digest?: string }).digest).includes('404');
  }
}

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal('fetch', fetchMock);
  vi.stubEnv('CURL_RUNNER_API_URL', 'https://api.test');
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
});

describe('collections list', () => {
  const page = (q?: string, cursor?: string) =>
    CollectionsPage({ searchParams: Promise.resolve({ q, cursor }) });

  it('renders cards, count line, show more and the list-your-own box', async () => {
    fetchMock.mockImplementation(async () => json(LIST));
    const html = await render(await page());
    expect(html).toContain('APIs, ready to run.');
    expect(html).toContain('48 collections from 21 teams');
    expect(html).toContain('Payments API');
    expect(html).toContain('18 requests · v14 · Updated 2 hours ago');
    expect(html).toContain('href="/collections/acme/payments-api"');
    expect(html).toContain('placeholder="Search by name, description or team"');
    expect(html).toContain('href="/collections?cursor=cur_2"');
    expect(html).toContain('List your own');
    expect(html).toContain('href="/docs/features/cloud"');
    expect(called()[0]).toBe('https://api.test/v1/public/collections');
    expect(fetchMock.mock.calls[0][1]).toMatchObject({ next: { revalidate: 300 } });
  });

  it('passes the search query as q and shows a plain count', async () => {
    fetchMock.mockImplementation(async () =>
      json({ ...LIST, collections: [PAYMENTS, USERS], total: 2 }),
    );
    const html = await render(await page('pay'));
    expect(called()[0]).toBe('https://api.test/v1/public/collections?q=pay');
    expect(html).toContain('2 collections<');
    expect(html).not.toContain('from 21 teams');
    expect(html).toContain('q=pay&amp;cursor=cur_2');
  });

  it('shows the no-match state', async () => {
    fetchMock.mockImplementation(async () =>
      json({ collections: [], next_cursor: null, total: 0, teams: 0 }),
    );
    const html = await render(await page('zzz'));
    expect(html).toContain('No collections match “zzz”');
    expect(html).toContain('Search looks at names, descriptions and teams.');
    expect(html).toContain('href="/collections"');
  });

  it('shows the empty state', async () => {
    fetchMock.mockImplementation(async () =>
      json({ collections: [], next_cursor: null, total: 0, teams: 0 }),
    );
    const html = await render(await page());
    expect(html).toContain('No collections yet');
    expect(html).toContain('Publish yours');
  });

  it.each([
    ['5xx', () => json({ error: 'boom' }, 503)],
    ['429', () => json({ error: 'rate_limited', message: 'slow down' }, 429)],
    ['network error', () => Promise.reject(new Error('offline'))],
    ['bad body', () => json({ nope: true })],
  ])('shows the calm error on %s without throwing', async (_name, respond) => {
    fetchMock.mockImplementation(respond);
    const html = await render(await page());
    expect(html).toContain('The gallery can’t be reached right now');
    expect(html).toContain('keep working in the studio and the CLI');
  });
});

describe('team page', () => {
  const page = (team: string) =>
    TeamPage({ params: Promise.resolve({ team }), searchParams: Promise.resolve({}) });

  it('lists the team collections without a team line', async () => {
    fetchMock.mockImplementation(async () =>
      json({ collections: [PAYMENTS, USERS], next_cursor: null, total: 2, teams: 1 }),
    );
    const html = await render(await page('acme'));
    expect(called()[0]).toBe('https://api.test/v1/public/collections?team=acme');
    expect(html).toContain('<h1');
    expect(html).toContain('2 public collections · curl-runner.com/collections/acme');
    expect(html).toContain('Users API');
    expect(html).not.toContain('Show more');
  });

  it('404s when the team has nothing listed', async () => {
    fetchMock.mockImplementation(async () =>
      json({ collections: [], next_cursor: null, total: 0, teams: 0 }),
    );
    expect(await isNotFound(page('acme'))).toBe(true);
  });

  it('404s for an invalid slug without calling the API', async () => {
    expect(await isNotFound(page('Bad_Slug'))).toBe(true);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('shows the calm error when the API is down', async () => {
    fetchMock.mockRejectedValue(new Error('offline'));
    expect(await render(await page('acme'))).toContain('can’t be reached');
  });
});

describe('collection detail', () => {
  const page = (team = 'acme', slug = 'payments-api', file?: string, req?: string) =>
    CollectionPage({
      params: Promise.resolve({ team, slug }),
      searchParams: Promise.resolve({ file, req }),
    });

  beforeEach(() => fetchMock.mockImplementation(async () => json(DETAIL)));

  it('renders header, meta, studio deep link, pull command and copy button', async () => {
    const html = await render(await page());
    expect(called()[0]).toBe('https://api.test/v1/public/collections/acme/payments-api');
    expect(html).toContain('<h1');
    expect(html).toContain('4 requests in 1 folder · v14 · Updated 2 hours ago');
    expect(html).toContain('Open in curl-runner studio');
    expect(html).toContain(
      'href="curl-runner://open/acme/payments-api?share=crsl_abc&amp;server=https%3A%2F%2Fapi.test"',
    );
    expect(html).toContain('href="/downloads"');
    expect(html).toContain('curl-runner pull https://cloud.curl-runner.com/s/crsl_abc');
    expect(html).toContain('aria-label="Copy the pull command"');
    expect(html).toContain('Nothing you enter is sent back to Acme.');
  });

  it('lists inputs with the secret note', async () => {
    const html = await render(await page());
    expect(html).toContain('BASE_URL');
    expect(html).toContain('Where the API lives');
    expect(html).toContain('API_KEY');
    expect(html.match(/Secret · stays on your machine/g)).toHaveLength(1);
  });

  it('groups requests by folder, root first, and skips the config file', async () => {
    const html = await render(await page());
    expect(html.indexOf('Health')).toBeLessThan(html.indexOf('>charges<'));
    expect(html).toContain('Create charge');
    expect(html).toContain('Refund');
    expect(html).toContain('Void');
    expect(html).not.toContain('href="/collections/acme/payments-api?file=curl-runner.yaml"');
    expect(html).toContain('color:#62C073');
  });

  it('shows the first request file by default and the selected one with ?file=', async () => {
    const def = await render(await page());
    expect(def).toContain('health.yaml');
    expect(def).toContain('Read only');
    expect(def).toContain('api.example.com/health');

    const picked = await render(await page('acme', 'payments-api', 'charges/create-charge.yaml'));
    expect(picked).toContain('charges/create-charge.yaml');
    expect(picked).toContain('api.example.com/charges');
  });

  it('shows the whole file for a request in a multi-request file', async () => {
    const html = await render(await page('acme', 'payments-api', 'charges/refunds.yaml', '1'));
    expect(html).toContain('charges/refunds.yaml');
    expect(html).toContain(
      'href="/collections/acme/payments-api?file=charges%2Frefunds.yaml&amp;req=1"',
    );
  });

  it('falls back to the first request for an unknown ?file=', async () => {
    expect(await render(await page('acme', 'payments-api', '../../etc/passwd'))).toContain(
      'health.yaml',
    );
  });

  it('404s when not listed', async () => {
    fetchMock.mockImplementation(async () => json({ error: 'not_found' }, 404));
    expect(await isNotFound(page())).toBe(true);
  });

  it.each([
    ['Acme', 'payments-api'],
    ['acme', '-bad'],
    ['acme', 'a'.repeat(64)],
    ['acme', '../x'],
  ])('404s for invalid slug %s/%s without calling the API', async (team, slug) => {
    expect(await isNotFound(page(team, slug))).toBe(true);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('shows the calm error when the API is down', async () => {
    fetchMock.mockImplementation(async () => json({}, 502));
    expect(await render(await page())).toContain('can’t be reached');
  });

  it('builds SEO metadata', async () => {
    const meta = await detailMeta({
      params: Promise.resolve({ team: 'acme', slug: 'payments-api' }),
      searchParams: Promise.resolve({}),
    });
    expect(meta.title).toEqual({ absolute: 'Payments API by Acme — curl-runner collections' });
    expect(meta.description).toBe(PAYMENTS.description);
    expect(meta.alternates?.canonical).toBe(
      'https://www.curl-runner.com/collections/acme/payments-api',
    );
    expect(meta.openGraph?.url).toBe('https://www.curl-runner.com/collections/acme/payments-api');
  });

  it('omits the studio and pull actions for an unsafe share link', async () => {
    fetchMock.mockImplementation(async () =>
      json({ ...DETAIL, link: 'https://x.test/s/a; rm -rf /' }),
    );
    const html = await render(await page());
    expect(html).not.toContain('Open in curl-runner studio');
    expect(html).not.toContain('curl-runner pull');
  });
});

describe('not-listed page', () => {
  it('says so and links back to the gallery', async () => {
    const html = await render(<CollectionNotFound />);
    expect(html).toContain('This collection isn’t listed');
    expect(html).toContain('If someone sent you its share link');
    expect(html).toContain('Browse collections');
    expect(html).toContain('href="/collections"');
  });
});

describe('helpers', () => {
  it('formats relative time', () => {
    const now = Date.parse('2026-11-13T12:00:00Z');
    expect(relativeTime('2026-11-13T11:59:50Z', now)).toBe('just now');
    expect(relativeTime('2026-11-13T11:00:00Z', now)).toBe('1 hour ago');
    expect(relativeTime('2026-11-10T12:00:00Z', now)).toBe('3 days ago');
    expect(relativeTime('2026-08-13T12:00:00Z', now)).toBe('3 months ago');
    expect(relativeTime('nope', now)).toBe('');
  });

  it('groups by top directory', () => {
    expect(folderOf('a/b/c.yaml')).toBe('a');
    expect(folderOf('c.yaml')).toBeNull();
    const groups = groupRequests(DETAIL.files);
    expect(groups.map((g) => g.folder)).toEqual([null, 'charges']);
    expect(groups[1].rows).toHaveLength(3);
  });

  it('builds the same deep link as the Cloud share page', () => {
    vi.stubEnv('CURL_RUNNER_API_URL', '');
    expect(studioLink('acme', 'pay', 'crsl_x')).toBe(
      'curl-runner://open/acme/pay?share=crsl_x&server=https%3A%2F%2Fapi.curl-runner.com',
    );
    expect(shareToken('https://cloud.curl-runner.com/s/crsl_x/')).toBe('crsl_x');
    expect(safeShareLink('https://cloud.curl-runner.com/s/crsl_x')).not.toBeNull();
    expect(safeShareLink('javascript:alert(1)')).toBeNull();
  });
});
