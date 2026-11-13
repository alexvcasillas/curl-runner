import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';
import { info, me } from '../fixtures';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };
const incoming = { current: new Headers({ 'x-pathname': '/t/acme/c/payments-api' }) };

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
  usePathname: () => '/t/acme/c/payments-api',
  useRouter: () => ({ push: vi.fn() }),
}));
vi.mock('next/cache', () => ({ revalidatePath: vi.fn() }));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { default: CollectionPage } = await import('@/app/(app)/t/[team]/c/[slug]/page');
const { SourceSwitch } = await import('@/components/SourceSwitch');
const { DeleteCollection } = await import('@/components/DeleteCollection');
const { InputsTable } = await import('@/components/InputsTable');
const { switchSource, deleteCollection } = await import('@/app/(app)/t/[team]/c/[slug]/actions');
const { generateMetadata } = await import('@/app/(app)/t/[team]/c/[slug]/page');
const { resetEnv } = await import('@/lib/env');

function json(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}
const fetchMock = vi.fn();
const byPath = (routes: Record<string, () => Response>) =>
  fetchMock.mockImplementation(async (url: string, init?: RequestInit) => {
    const path = new URL(url).pathname;
    const make = routes[`${init?.method ?? 'GET'} ${path}`] ?? routes[path];
    if (!make) {
      throw new Error(`unexpected ${init?.method ?? 'GET'} ${path}`);
    }
    return make();
  });

const base = '/v1/teams/acme/collections';
const sha = '3f9a2c1d8e7b6a5f4e3d2c1b0a9f8e7d6c5b4a39';
const versions = [
  {
    number: 14,
    hash: 'h14',
    published_at: '2026-11-05T13:02:00Z',
    publisher: 'token github-actions',
    git: { commit: sha, ref: 'main', repo_url: 'https://github.com/acme/payments' },
    file_count: 12,
  },
  {
    number: 13,
    hash: 'h13',
    published_at: '2026-11-04T17:40:00Z',
    publisher: 'grace@acme.example',
    git: {
      commit: 'a81d07e5c2',
      ref: 'release/2.3',
      repo_url: 'https://git.example.com/acme/payments',
    },
    file_count: 12,
  },
  {
    number: 12,
    hash: 'h12',
    published_at: '2026-11-02T09:15:00Z',
    publisher: 'ada@acme.example',
    git: null,
    file_count: 11,
  },
];
const inputs = [
  {
    name: 'API_TOKEN',
    description: 'A token for the staging API.',
    secret: true,
    required: true,
    locked: false,
    default: 'hunter2-super-secret',
  },
  {
    name: 'BASE_URL',
    description: null,
    secret: false,
    required: false,
    locked: true,
    default: 'https://api.example.com',
  },
  {
    name: 'ACCOUNT_ID',
    description: 'Your account ID.',
    secret: false,
    required: true,
    locked: false,
    default: null,
  },
];
const collection = (source: 'git' | 'cloud' = 'git', access = 'public') => ({
  slug: 'payments-api',
  name: 'Payments API',
  source,
  access,
  current_version: 14,
  updated_at: versions[0].published_at,
  publisher: versions[0].publisher,
});
const share =
  (entries: unknown[] = []) =>
  () =>
    json(200, {
      access: 'public',
      link: 'https://cloud.example.com/s/abc123',
      expires_at: null,
      entries,
    });

function routes(over: Record<string, () => Response> = {}, source: 'git' | 'cloud' = 'git') {
  byPath({
    '/v1/me': () => json(200, me),
    '/v1/info': () => json(200, info),
    [base]: () => json(200, [collection(source)]),
    [`${base}/payments-api/versions`]: () => json(200, versions),
    [`${base}/payments-api/versions/14`]: () => json(200, { ...versions[0], inputs }),
    [`${base}/payments-api/share`]: share(),
    ...over,
  });
}

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

const page = (tab?: string, team = 'acme', slug = 'payments-api') => ({
  params: Promise.resolve({ team, slug }),
  searchParams: Promise.resolve(tab ? { tab } : {}),
});

describe('collection header', () => {
  it('shows the name, source, access, the latest publish and the link', async () => {
    routes();
    render(await CollectionPage(page()));
    expect(screen.getByRole('heading', { level: 1, name: 'Payments API' })).toBeInTheDocument();
    const head = screen.getByRole('heading', { level: 1 }).parentElement as HTMLElement;
    expect(within(head).getByText('Git')).toBeInTheDocument();
    expect(within(head).getByText('Public')).toBeInTheDocument();
    expect(screen.getByRole('navigation', { name: 'Breadcrumb' })).toHaveTextContent(
      'Collections/Payments API',
    );
    expect(screen.getByRole('button', { name: 'Copy link' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Share' })).toHaveAttribute(
      'href',
      '/t/acme/c/payments-api/sharing',
    );
  });

  it('is not found for an unknown collection or team', async () => {
    routes();
    await expect(CollectionPage(page(undefined, 'acme', 'nope'))).rejects.toThrow('NEXT_NOT_FOUND');
    await expect(CollectionPage(page(undefined, 'other'))).rejects.toThrow('NEXT_NOT_FOUND');
  });

  it('has the tabs, with Sharing on its own page', async () => {
    routes();
    render(await CollectionPage(page()));
    const nav = screen.getByRole('navigation', { name: 'Collection' });
    expect(within(nav).getByRole('link', { name: 'Versions' })).toHaveAttribute(
      'aria-current',
      'page',
    );
    expect(within(nav).getByRole('link', { name: 'Inputs' })).toHaveAttribute(
      'href',
      '/t/acme/c/payments-api?tab=inputs',
    );
    expect(within(nav).getByRole('link', { name: 'Settings' })).toHaveAttribute(
      'href',
      '/t/acme/c/payments-api?tab=settings',
    );
    expect(within(nav).getByRole('link', { name: 'Sharing' })).toHaveAttribute(
      'href',
      '/t/acme/c/payments-api/sharing',
    );
  });
});

describe('route params', () => {
  it('is not found for a slug the server would never issue, without calling the API', async () => {
    routes();
    fetchMock.mockClear();
    await expect(CollectionPage(page(undefined, 'acme', 'Bad_Slug'))).rejects.toThrow(
      'NEXT_NOT_FOUND',
    );
    await expect(CollectionPage(page(undefined, '../x', 'payments-api'))).rejects.toThrow(
      'NEXT_NOT_FOUND',
    );
    expect(fetchMock.mock.calls.map((c) => new URL(c[0]).pathname)).not.toContain('/v1/teams/../x');
  });
});

describe('metadata', () => {
  const props = { params: Promise.resolve({ team: 'acme', slug: 'payments-api' }) };

  it('titles the page with the collection name', async () => {
    routes();
    expect(await generateMetadata({ ...props, searchParams: Promise.resolve({}) })).toEqual({
      title: 'Payments API',
    });
  });

  it('falls back to a plain title on an API error only', async () => {
    routes({ [base]: () => json(503, { error: 'unavailable', message: '' }) });
    expect(await generateMetadata({ ...props, searchParams: Promise.resolve({}) })).toEqual({
      title: 'Collection',
    });
    fetchMock.mockRejectedValue(new TypeError('boom'));
    // A network failure is an ApiError too (503), so still a title. Anything else would throw.
    expect((await generateMetadata({ ...props, searchParams: Promise.resolve({}) })).title).toBe(
      'Collection',
    );
  });
});

describe('versions', () => {
  it('links the commit only for github.com and gitlab.com', async () => {
    routes();
    render(await CollectionPage(page()));
    const rows = screen.getAllByRole('row').slice(1);
    expect(rows).toHaveLength(3);
    const link = within(rows[0]).getByRole('link', { name: '3f9a2c1' });
    expect(link).toHaveAttribute('href', `https://github.com/acme/payments/commit/${sha}`);
    expect(within(rows[0]).getByText('Current')).toBeInTheDocument();
    expect(within(rows[0]).getByText('main')).toBeInTheDocument();
    expect(within(rows[0]).getByText('token github-actions')).toBeInTheDocument();
    expect(within(rows[0]).getByText('12')).toBeInTheDocument();

    // A host we don't know stays plain text.
    expect(within(rows[1]).getByText('a81d07e')).toBeInTheDocument();
    expect(within(rows[1]).queryByRole('link', { name: 'a81d07e' })).not.toBeInTheDocument();
    // No git info: a dash.
    expect(within(rows[2]).queryByRole('link', { name: /^[0-9a-f]{7}$/ })).not.toBeInTheDocument();
  });

  it('links each version to its changes and its zip', async () => {
    routes();
    render(await CollectionPage(page()));
    const rows = screen.getAllByRole('row').slice(1);
    expect(within(rows[0]).getByRole('link', { name: 'Changes' })).toHaveAttribute(
      'href',
      '/t/acme/c/payments-api/versions/14',
    );
    expect(within(rows[1]).getByRole('link', { name: 'Download v13 as a zip' })).toHaveAttribute(
      'href',
      '/t/acme/c/payments-api/versions/13/download',
    );
  });

  it('shows the empty state before the first version', async () => {
    routes({
      [base]: () => json(200, [{ ...collection(), current_version: null, updated_at: null }]),
      [`${base}/payments-api/versions`]: () => json(200, []),
    });
    render(await CollectionPage(page()));
    expect(screen.getByRole('heading', { name: 'No versions yet' })).toBeInTheDocument();
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
  });

  it('shows the newest 10 and reveals older ones on request', async () => {
    const many = Array.from({ length: 14 }, (_, i) => ({
      ...versions[2],
      number: 14 - i,
      hash: `h${i}`,
    }));
    routes({ [`${base}/payments-api/versions`]: () => json(200, many) });
    const user = userEvent.setup();
    render(await CollectionPage(page()));
    expect(screen.getAllByRole('row').slice(1)).toHaveLength(10);
    expect(screen.getByText(/14 versions\. Cloud keeps every one\./)).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Show older' }));
    expect(screen.getAllByRole('row').slice(1)).toHaveLength(14);
    expect(screen.queryByRole('button', { name: 'Show older' })).not.toBeInTheDocument();
  });
});

describe('inputs', () => {
  it('lists name, description, required, secret, locked and default', async () => {
    routes();
    render(await CollectionPage(page('inputs')));
    const rows = screen.getAllByRole('row').slice(1);
    expect(rows).toHaveLength(3);
    expect(within(rows[1]).getByText('BASE_URL')).toBeInTheDocument();
    expect(within(rows[1]).getByText('https://api.example.com')).toBeInTheDocument();
    expect(within(rows[2]).getByText('None')).toBeInTheDocument();
    expect(within(rows[0]).getByText('A token for the staging API.')).toBeInTheDocument();
  });

  it('never shows the default of a secret', async () => {
    routes();
    const { container } = render(await CollectionPage(page('inputs')));
    expect(container).not.toHaveTextContent('hunter2-super-secret');
    expect(screen.getByText('Kept on each customer’s machine')).toBeInTheDocument();
  });

  it('hides a secret default in the table itself', () => {
    const { container } = render(<InputsTable name="Payments API" inputs={inputs} />);
    expect(container.innerHTML).not.toContain('hunter2');
  });

  it('says so when a version declares none', async () => {
    routes({
      [`${base}/payments-api/versions/14`]: () => json(200, { ...versions[0], inputs: [] }),
    });
    render(await CollectionPage(page('inputs')));
    expect(screen.getByRole('heading', { name: 'No inputs' })).toBeInTheDocument();
  });
});

describe('settings', () => {
  it('shows the source, the download and, to an owner, delete', async () => {
    routes();
    render(await CollectionPage(page('settings')));
    expect(screen.getByRole('radiogroup', { name: 'Source' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Download v14' })).toHaveAttribute(
      'href',
      '/t/acme/c/payments-api/versions/14/download',
    );
    expect(screen.getByRole('button', { name: 'Delete Payments API' })).toBeInTheDocument();
  });

  it('keeps delete from a member', async () => {
    routes({
      '/v1/me': () =>
        json(200, { ...me, teams: [{ slug: 'acme', name: 'Acme', role: 'member', plan: 'team' }] }),
    });
    render(await CollectionPage(page('settings')));
    expect(screen.queryByRole('button', { name: /^Delete/ })).not.toBeInTheDocument();
    expect(screen.getByText('Only team owners can delete a collection.')).toBeInTheDocument();
  });
});

describe('SourceSwitch', () => {
  const ok = { ok: true } as const;

  function setup(
    source: 'git' | 'cloud',
    onSwitch = vi.fn().mockResolvedValue(ok),
    version: number | null = 14,
    owner = true,
  ) {
    render(
      <SourceSwitch
        name="Payments API"
        source={source}
        version={version}
        downloadHref="/t/acme/c/payments-api/versions/14/download"
        owner={owner}
        onSwitch={onSwitch}
      />,
    );
    return onSwitch;
  }

  it('warns before switching git to Cloud, and switches on confirm', async () => {
    const user = userEvent.setup();
    const onSwitch = setup('git');
    expect(screen.getByRole('radio', { name: 'Files' })).toBeChecked();
    expect(screen.getByText(/The studio can’t sync edits to it/)).toBeInTheDocument();
    await user.click(screen.getByRole('radio', { name: 'Cloud' }));
    const warning = screen.getByRole('alert');
    expect(warning).toHaveTextContent('CI publishes to Payments API will be refused');
    expect(warning).toHaveTextContent('synced from the studio');
    expect(onSwitch).not.toHaveBeenCalled();
    // Nothing changed yet.
    expect(screen.getByRole('radio', { name: 'Files' })).toBeChecked();
    await user.click(screen.getByRole('button', { name: 'Switch to Cloud' }));
    expect(onSwitch).toHaveBeenCalledWith('cloud');
  });

  it('warns that studio edits get overwritten going Cloud to git, and offers the zip', async () => {
    const user = userEvent.setup();
    const onSwitch = setup('cloud');
    await user.click(screen.getByRole('radio', { name: 'Files' }));
    const warning = screen.getByRole('alert');
    expect(warning).toHaveTextContent('v14 stays current until the next CI publish replaces it');
    expect(warning).toHaveTextContent('Studio edits that aren’t in git yet will be overwritten');
    expect(within(warning).getByRole('link', { name: 'Download v14 (.zip)' })).toHaveAttribute(
      'href',
      '/t/acme/c/payments-api/versions/14/download',
    );
    await user.click(screen.getByRole('button', { name: 'Switch to Files' }));
    expect(onSwitch).toHaveBeenCalledWith('git');
  });

  it('shows members the source as text, without the switch', () => {
    setup('cloud', vi.fn(), 14, false);
    expect(screen.queryByRole('radio')).not.toBeInTheDocument();
    expect(screen.getByText('Source: Cloud')).toBeInTheDocument();
    expect(screen.getByText('Only team owners can change this.')).toBeInTheDocument();
  });

  it('cancels without calling the API', async () => {
    const user = userEvent.setup();
    const onSwitch = setup('git');
    await user.click(screen.getByRole('radio', { name: 'Cloud' }));
    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(onSwitch).not.toHaveBeenCalled();
  });

  it('shows the API message when the switch fails', async () => {
    const user = userEvent.setup();
    setup(
      'git',
      vi.fn().mockResolvedValue({ ok: false, message: 'Cloud isn’t answering right now.' }),
    );
    await user.click(screen.getByRole('radio', { name: 'Cloud' }));
    await user.click(screen.getByRole('button', { name: 'Switch to Cloud' }));
    expect(await screen.findByText('Cloud isn’t answering right now.')).toBeInTheDocument();
    // The choice is still open, so they can retry.
    expect(screen.getByRole('button', { name: 'Switch to Cloud' })).toBeEnabled();
  });

  it('has no download to offer before the first version', async () => {
    const user = userEvent.setup();
    setup('cloud', undefined, null);
    await user.click(screen.getByRole('radio', { name: 'Files' }));
    expect(screen.queryByRole('link', { name: /Download/ })).not.toBeInTheDocument();
  });
});

describe('DeleteCollection', () => {
  const render_ = (onDelete = vi.fn().mockResolvedValue({ ok: true }), people = 4) => {
    render(
      <DeleteCollection
        name="Payments API"
        slug="payments-api"
        versions={14}
        people={people}
        source="git"
        onDelete={onDelete}
      />,
    );
    return onDelete;
  };

  it('needs the slug typed first', async () => {
    const user = userEvent.setup();
    const onDelete = render_();
    await user.click(screen.getByRole('button', { name: 'Delete Payments API' }));
    const dialog = screen.getByRole('alertdialog');
    expect(dialog).toHaveTextContent(
      'This deletes all 14 versions, the share link and the access list.',
    );
    expect(dialog).toHaveTextContent('4 people on the access list stop getting updates.');
    const confirm = within(dialog).getByRole('button', { name: 'Delete collection' });
    expect(confirm).toBeDisabled();
    await user.type(within(dialog).getByLabelText(/Type/), 'payments');
    expect(confirm).toBeDisabled();
    await user.type(within(dialog).getByLabelText(/Type/), '-api');
    expect(confirm).toBeEnabled();
    await user.click(confirm);
    expect(onDelete).toHaveBeenCalledTimes(1);
  });

  it('shows the API message when delete fails', async () => {
    const user = userEvent.setup();
    render_(vi.fn().mockResolvedValue({ ok: false, message: 'Only team owners can do this.' }));
    await user.click(screen.getByRole('button', { name: 'Delete Payments API' }));
    const dialog = screen.getByRole('alertdialog');
    await user.type(within(dialog).getByLabelText(/Type/), 'payments-api');
    await user.click(within(dialog).getByRole('button', { name: 'Delete collection' }));
    expect(await within(dialog).findByText('Only team owners can do this.')).toBeInTheDocument();
  });
});

describe('actions', () => {
  it('switches the source with PATCH', async () => {
    byPath({ [`PATCH ${base}/payments-api`]: () => json(200, collection('cloud')) });
    expect(await switchSource('acme', 'payments-api', 'cloud')).toEqual({ ok: true });
    const [, init] = fetchMock.mock.calls[0];
    expect(init.method).toBe('PATCH');
    expect(JSON.parse(init.body)).toEqual({ source: 'cloud' });
  });

  it('turns an API error into its message', async () => {
    byPath({
      [`PATCH ${base}/payments-api`]: () =>
        json(404, { error: 'not_found', message: 'No such collection.' }),
    });
    expect(await switchSource('acme', 'payments-api', 'git')).toEqual({
      ok: false,
      message: 'No such collection.',
      code: 'not_found',
    });
  });

  it('refuses a source other than git or cloud', async () => {
    byPath({});
    // @ts-expect-error not a source
    expect((await switchSource('acme', 'payments-api', 'ftp')).ok).toBe(false);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('deletes with DELETE and goes back to the list', async () => {
    byPath({ [`DELETE ${base}/payments-api`]: () => new Response(null, { status: 204 }) });
    await expect(deleteCollection('acme', 'payments-api')).rejects.toThrow('NEXT_REDIRECT /t/acme');
    expect(fetchMock.mock.calls[0][1].method).toBe('DELETE');
  });

  it('says why a delete failed', async () => {
    byPath({
      [`DELETE ${base}/payments-api`]: () =>
        json(403, { error: 'forbidden', message: 'Only owners.' }),
    });
    expect(await deleteCollection('acme', 'payments-api')).toEqual({
      ok: false,
      message: 'Only owners.',
      code: 'forbidden',
    });
  });
});
