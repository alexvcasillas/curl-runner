import { render, screen, within } from '@testing-library/react';
import { unzipSync } from 'fflate';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';
import { info, me } from '../fixtures';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };
const incoming = { current: new Headers({ 'x-pathname': '/t/acme/c/payments-api/versions/2' }) };

vi.mock('next/headers', () => ({
  cookies: async () => jar.current.store,
  headers: async () => incoming.current,
}));
vi.mock('next/navigation', () => ({
  redirect: (url: string) => {
    throw new Error(`NEXT_REDIRECT ${url}`);
  },
  notFound: () => {
    throw new Error('NEXT_NOT_FOUND');
  },
  usePathname: () => '/t/acme/c/payments-api/versions/2',
  useRouter: () => ({ push: vi.fn() }),
}));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { default: DiffPage } = await import('@/app/(app)/t/[team]/c/[slug]/versions/[n]/page');
const { GET: download } = await import('@/app/(app)/t/[team]/c/[slug]/versions/[n]/download/route');
const { resetEnv } = await import('@/lib/env');

function json(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}
const fetchMock = vi.fn();
const byPath = (routes: Record<string, () => Response>) =>
  fetchMock.mockImplementation(async (url: string) => {
    const path = new URL(url).pathname;
    const make = routes[path];
    if (!make) {
      throw new Error(`unexpected ${path}`);
    }
    return make();
  });

const base = '/v1/teams/acme/collections/payments-api';
const summary = (number: number, files = 2) => ({
  number,
  hash: `h${number}`,
  published_at: '2026-11-05T13:02:00Z',
  publisher: 'token github-actions',
  git: { commit: '3f9a2c1d8e7b', ref: 'main', repo_url: 'https://github.com/acme/payments' },
  file_count: files,
});
const v1Files = [
  { path: 'curl-runner.yaml', text: 'cloud:\n  collection: payments-api\n' },
  { path: 'requests/create.yaml', text: 'request:\n  method: POST\n  timeout: 5000\n' },
];
const v2Files = [
  { path: 'curl-runner.yaml', text: 'cloud:\n  collection: payments-api\n' },
  { path: 'requests/create.yaml', text: 'request:\n  method: POST\n  timeout: 10000\n' },
  { path: 'requests/refund.yaml', text: 'request:\n  method: POST\n' },
];

const routes = (over: Record<string, () => Response> = {}) =>
  byPath({
    '/v1/me': () => json(200, me),
    '/v1/info': () => json(200, info),
    '/v1/teams/acme/collections': () =>
      json(200, [
        {
          slug: 'payments-api',
          name: 'Payments API',
          source: 'git',
          access: 'public',
          current_version: 2,
          updated_at: '2026-11-05T13:02:00Z',
        },
      ]),
    [`${base}/versions`]: () => json(200, [summary(2, 3), summary(1)]),
    [`${base}/versions/2`]: () => json(200, { ...summary(2, 3), inputs: [] }),
    [`${base}/versions/1`]: () => json(200, { ...summary(1), inputs: [] }),
    [`${base}/versions/2/files`]: () => json(200, v2Files),
    [`${base}/versions/1/files`]: () => json(200, v1Files),
    ...over,
  });

beforeEach(() => {
  resetEnv();
  vi.stubEnv('API_INTERNAL_URL', 'http://api.internal:8080');
  vi.stubGlobal('fetch', fetchMock);
  fetchMock.mockReset();
  jar.current = cookieJar({ cr_session: 'crs_ok' });
});
afterEach(() => {
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
  resetEnv();
});

const page = (n: string, search: Record<string, string> = {}) => ({
  params: Promise.resolve({ team: 'acme', slug: 'payments-api', n }),
  searchParams: Promise.resolve(search),
});

describe('version diff page', () => {
  it('shows what changed against the previous version', async () => {
    routes();
    render(await DiffPage(page('2')));
    expect(
      screen.getByRole('heading', { level: 1, name: 'What changed in v2' }),
    ).toBeInTheDocument();
    expect(screen.getByText(/2 files changed: 1 added, 1 changed\./)).toBeInTheDocument();
    expect(screen.getByRole('navigation', { name: 'Breadcrumb' })).toHaveTextContent(
      'Collections/Payments API/v2',
    );

    const files = within(screen.getByRole('navigation', { name: 'Changed files' }));
    expect(files.getAllByRole('link').map((a) => a.textContent)).toEqual([
      expect.stringContaining('requests/create.yaml'),
      expect.stringContaining('requests/refund.yaml'),
    ]);

    const changed = screen.getByRole('region', { name: 'requests/create.yaml' });
    expect(within(changed).getByText('Changed')).toBeInTheDocument();
    expect(within(changed).getByText('timeout: 5000')).toBeInTheDocument();
    expect(within(changed).getByText('timeout: 10000')).toBeInTheDocument();
    const added = screen.getByRole('region', { name: 'requests/refund.yaml' });
    expect(within(added).getByText('Added')).toBeInTheDocument();
    // The unchanged file is left out.
    expect(screen.queryByRole('region', { name: 'curl-runner.yaml' })).not.toBeInTheDocument();
  });

  it('shows every file as added for version 1', async () => {
    routes();
    render(await DiffPage(page('1')));
    expect(
      screen.getByRole('heading', { level: 1, name: 'What changed in v1' }),
    ).toBeInTheDocument();
    expect(screen.getByText(/2 files changed: 2 added\./)).toBeInTheDocument();
    expect(screen.getAllByText('Added')).toHaveLength(2);
    expect(screen.queryByRole('combobox')).not.toBeInTheDocument();
    expect(fetchMock.mock.calls.map((c) => new URL(c[0]).pathname)).not.toContain(
      `${base}/versions/0/files`,
    );
  });

  it('compares with an older version on request', async () => {
    routes({
      [`${base}/versions`]: () => json(200, [summary(3), summary(2), summary(1)]),
      [`${base}/versions/3`]: () => json(200, { ...summary(3), inputs: [] }),
      [`${base}/versions/3/files`]: () => json(200, v2Files),
    });
    render(await DiffPage(page('3', { compare: '1' })));
    expect(screen.getByText(/against v1/)).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'Compare with' })).toHaveValue('1');
    expect(screen.getAllByRole('option').map((o) => o.textContent)).toEqual(['v2', 'v1']);
  });

  it('says so when nothing changed', async () => {
    routes({ [`${base}/versions/2/files`]: () => json(200, v1Files) });
    render(await DiffPage(page('2')));
    expect(screen.getByRole('heading', { name: 'No file changes' })).toBeInTheDocument();
  });

  it('is not found for a version that does not exist', async () => {
    routes();
    await expect(DiffPage(page('9'))).rejects.toThrow('NEXT_NOT_FOUND');
    await expect(DiffPage(page('abc'))).rejects.toThrow('NEXT_NOT_FOUND');
    await expect(DiffPage(page('0'))).rejects.toThrow('NEXT_NOT_FOUND');
  });
});

describe('zip download', () => {
  const ctx = (n: string, team = 'acme', slug = 'payments-api') => ({
    params: Promise.resolve({ team, slug, n }),
  });

  it('returns a zip with every path', async () => {
    routes();
    const res = await download(new Request('http://localhost/x'), ctx('2'));
    expect(res.status).toBe(200);
    expect(res.headers.get('content-type')).toBe('application/zip');
    expect(res.headers.get('content-disposition')).toBe(
      'attachment; filename="payments-api-v2.zip"',
    );
    const entries = unzipSync(new Uint8Array(await res.arrayBuffer()));
    expect(Object.keys(entries).sort()).toEqual(v2Files.map((f) => f.path).sort());
    expect(new TextDecoder().decode(entries['requests/create.yaml'])).toBe(v2Files[1].text);
  });

  it('is 404 for a missing version, with no body from the API leaking', async () => {
    routes({
      [`${base}/versions/7/files`]: () =>
        json(404, { error: 'not_found', message: 'secret detail' }),
    });
    const res = await download(new Request('http://localhost/x'), ctx('7'));
    expect(res.status).toBe(404);
    expect(res.headers.get('content-type')).not.toBe('application/zip');
  });

  it('rejects a version that is not a number', async () => {
    routes();
    const res = await download(new Request('http://localhost/x'), ctx('2/../x'));
    expect(res.status).toBe(404);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('is 503 when the API is down', async () => {
    fetchMock.mockRejectedValue(new Error('down'));
    const res = await download(new Request('http://localhost/x'), ctx('2'));
    expect(res.status).toBe(503);
  });
});
