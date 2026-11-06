import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };
const incoming = { current: new Headers({ 'x-pathname': '/t/acme/tokens' }) };

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
}));
const revalidatePath = vi.fn();
vi.mock('next/cache', () => ({ revalidatePath: (...a: unknown[]) => revalidatePath(...a) }));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { TokensView } = await import('@/components/TokensView');
const actions = await import('@/app/(app)/t/[team]/tokens/actions');
const { resetEnv } = await import('@/lib/env');

const VALUE = 'crt_Hx3nQ8vLp2YtR6wKc9ZbM1aJf5sD7uEg4oNiT0qBVyW';
const ID = '3f2b8a52-8f2d-4a3e-9f53-0e2d2f5a7c11';

const rows = [
  {
    id: ID,
    name: 'github-actions',
    by: 'ada@acme.example',
    created: 'Oct 2',
    expires: 'Never',
    expiresSoon: false,
    lastUsed: '2 hours ago',
  },
  {
    id: '9a1b8a52-8f2d-4a3e-9f53-0e2d2f5a7c22',
    name: 'staging-ci',
    by: 'ada@acme.example',
    created: 'Aug 1',
    expires: 'In 9 days',
    expiresSoon: true,
    lastUsed: null,
  },
];

function setup(over: Partial<React.ComponentProps<typeof TokensView>> = {}) {
  const onCreate = vi.fn(async (name: string, days: number | null) => ({
    ok: true as const,
    token: { id: ID, name, value: VALUE, expiresAt: days === null ? null : '2027-02-03T00:00:00Z' },
  }));
  const onRevoke = vi.fn(async (_id: string) => ({ ok: true as const }));
  const user = userEvent.setup();
  const utils = render(
    <TokensView
      team="acme"
      teamName="Acme"
      tokens={rows}
      onCreate={onCreate}
      onRevoke={onRevoke}
      {...over}
    />,
  );
  return { user, onCreate, onRevoke, ...utils };
}

describe('TokensView', () => {
  it('lists tokens with who made them, when they expire and when they were used', () => {
    setup();
    expect(screen.getByText('github-actions')).toBeInTheDocument();
    expect(screen.getAllByText('ada@acme.example')).toHaveLength(2);
    expect(screen.getByText('In 9 days')).toBeInTheDocument();
    expect(screen.getByText('2 hours ago')).toBeInTheDocument();
    expect(screen.getByText('Never used')).toBeInTheDocument();
    expect(screen.getAllByText('crt_••••')).toHaveLength(2);
  });

  it('shows the empty state with a Create token button', async () => {
    const { user } = setup({ tokens: [] });
    expect(screen.getByRole('heading', { name: 'No publish tokens yet' })).toBeInTheDocument();
    const buttons = screen.getAllByRole('button', { name: 'Create token' });
    await user.click(buttons[buttons.length - 1]);
    expect(
      await screen.findByRole('dialog', { name: 'Create a publish token' }),
    ).toBeInTheDocument();
  });

  it('creates a token with a name and an expiry, then shows the value once', async () => {
    const { user, onCreate } = setup();
    await user.click(screen.getByRole('button', { name: 'Create token' }));
    const dialog = await screen.findByRole('dialog', { name: 'Create a publish token' });
    const name = within(dialog).getByLabelText('Name');
    await user.clear(name);
    await user.type(name, 'release-script');
    await user.click(within(dialog).getByRole('radio', { name: '30 days' }));
    await user.click(within(dialog).getByRole('button', { name: 'Create token' }));
    expect(onCreate).toHaveBeenCalledWith('release-script', 30);

    const done = await screen.findByRole('dialog', { name: 'Token created' });
    expect(within(done).getByText('Copy it now. You won’t see it again.')).toBeInTheDocument();
    expect(within(done).getByText(VALUE)).toBeInTheDocument();
    expect(within(done).getByRole('button', { name: 'Copy token' })).toBeInTheDocument();
  });

  it('defaults to 90 days and sends never as null', async () => {
    const { user, onCreate } = setup();
    await user.click(screen.getByRole('button', { name: 'Create token' }));
    const dialog = await screen.findByRole('dialog', { name: 'Create a publish token' });
    expect(within(dialog).getByRole('radio', { name: '90 days' })).toBeChecked();
    await user.click(within(dialog).getByRole('radio', { name: 'Never' }));
    expect(within(dialog).getByText('It works until someone revokes it.')).toBeInTheDocument();
    await user.click(within(dialog).getByRole('button', { name: 'Create token' }));
    expect(onCreate).toHaveBeenCalledWith('github-actions', null);
  });

  it('puts the team slug and the secret name in the Action snippet', async () => {
    const { user } = setup();
    await user.click(screen.getByRole('button', { name: 'Create token' }));
    await user.click(
      within(await screen.findByRole('dialog', { name: 'Create a publish token' })).getByRole(
        'button',
        { name: 'Create token' },
      ),
    );
    const done = await screen.findByRole('dialog', { name: 'Token created' });
    const snippet = done.querySelector('pre')?.textContent ?? '';
    expect(snippet).toContain('uses: alexvcasillas/curl-runner/actions/publish@main');
    // biome-ignore lint/suspicious/noTemplateCurlyInString: the Actions expression is literal text.
    expect(snippet).toContain('token: ${{ secrets.CURL_RUNNER_TOKEN }}');
    expect(snippet).toContain('team: acme');
    // The snippet names the secret. It never holds the value.
    expect(snippet).not.toContain('crt_');
  });

  it('drops the value when the dialog closes, and a fresh render never has it', async () => {
    const { user, container, unmount } = setup();
    await user.click(screen.getByRole('button', { name: 'Create token' }));
    await user.click(
      within(await screen.findByRole('dialog', { name: 'Create a publish token' })).getByRole(
        'button',
        { name: 'Create token' },
      ),
    );
    await screen.findByRole('dialog', { name: 'Token created' });
    expect(document.body.innerHTML).toContain(VALUE);
    await user.click(screen.getByRole('button', { name: 'Done' }));
    await waitFor(() => expect(document.body.innerHTML).not.toContain('crt_H'));
    unmount();
    const again = render(
      <TokensView
        team="acme"
        teamName="Acme"
        tokens={rows}
        onCreate={vi.fn()}
        onRevoke={vi.fn()}
      />,
    );
    expect(again.container.innerHTML).not.toMatch(/crt_[A-Za-z0-9]/);
    expect(container.innerHTML).not.toMatch(/crt_[A-Za-z0-9]/);
  });

  it('shows why a create failed and keeps the dialog open', async () => {
    const { user } = setup({
      onCreate: vi.fn(async () => ({ ok: false as const, message: 'Give the token a name.' })),
    });
    await user.click(screen.getByRole('button', { name: 'Create token' }));
    const dialog = await screen.findByRole('dialog', { name: 'Create a publish token' });
    await user.click(within(dialog).getByRole('button', { name: 'Create token' }));
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('Give the token a name.');
  });

  it('revokes behind a confirm that says when it was last used', async () => {
    const { user, onRevoke } = setup();
    await user.click(screen.getAllByRole('button', { name: /^Revoke/ })[0]);
    const dialog = await screen.findByRole('alertdialog', { name: 'Revoke github-actions?' });
    expect(dialog).toHaveTextContent(
      'The next publish that uses it fails with “Sign in or use a valid token.”',
    );
    expect(dialog).toHaveTextContent('Last used 2 hours ago.');
    expect(onRevoke).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole('button', { name: 'Revoke token' }));
    expect(onRevoke).toHaveBeenCalledWith(ID);
  });

  it('says a token that was never used breaks nothing', async () => {
    const { user } = setup();
    await user.click(screen.getAllByRole('button', { name: /^Revoke/ })[1]);
    const dialog = await screen.findByRole('alertdialog', { name: 'Revoke staging-ci?' });
    expect(dialog).toHaveTextContent('It hasn’t been used, so no publish depends on it.');
  });

  it('shows the API message when a revoke fails', async () => {
    const { user } = setup({
      onRevoke: vi.fn(async () => ({ ok: false as const, message: 'It doesn’t exist.' })),
    });
    await user.click(screen.getAllByRole('button', { name: /^Revoke/ })[0]);
    const dialog = await screen.findByRole('alertdialog');
    await user.click(within(dialog).getByRole('button', { name: 'Revoke token' }));
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('It doesn’t exist.');
  });
});

describe('token actions', () => {
  const fetchMock = vi.fn();
  beforeEach(() => {
    fetchMock.mockReset();
    revalidatePath.mockReset();
    vi.stubGlobal('fetch', fetchMock);
    vi.stubEnv('API_INTERNAL_URL', 'http://api.test');
    vi.stubEnv('PUBLIC_APP_URL', 'http://localhost:3000');
    resetEnv();
  });
  const json = (status: number, body: unknown) =>
    new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

  it('creates a token and returns the value to the caller only', async () => {
    fetchMock.mockResolvedValue(
      json(201, { id: ID, name: 'ci', token: VALUE, expires_at: '2027-02-03T00:00:00Z' }),
    );
    const result = await actions.createToken('acme', ' ci ', 90);
    expect(result).toEqual({
      ok: true,
      token: { id: ID, name: 'ci', value: VALUE, expiresAt: '2027-02-03T00:00:00Z' },
    });
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('http://api.test/v1/teams/acme/tokens');
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body)).toEqual({ name: 'ci', expires_in_days: 90 });
    // Nothing cached or revalidated carries the value.
    expect(JSON.stringify(revalidatePath.mock.calls)).not.toContain('crt_');
    expect(revalidatePath).toHaveBeenCalledWith('/t/acme/tokens');
  });

  it('leaves expires_in_days out for never', async () => {
    fetchMock.mockResolvedValue(json(201, { id: ID, name: 'ci', token: VALUE, expires_at: null }));
    await actions.createToken('acme', 'ci', null);
    expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toEqual({ name: 'ci' });
  });

  it('turns an API error into its message', async () => {
    fetchMock.mockResolvedValue(
      json(400, {
        error: 'invalid_request',
        message: 'Give the token a name of up to 100 characters.',
      }),
    );
    expect(await actions.createToken('acme', 'ci', 30)).toEqual({
      ok: false,
      message: 'Give the token a name of up to 100 characters.',
      code: 'invalid_request',
    });
  });

  it('revokes with DELETE', async () => {
    fetchMock.mockResolvedValue(new Response(null, { status: 204 }));
    expect(await actions.revokeToken('acme', ID)).toEqual({ ok: true });
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe(`http://api.test/v1/teams/acme/tokens/${ID}`);
    expect(init.method).toBe('DELETE');
  });

  it('rejects inputs the page never sends, without calling the API', async () => {
    const bad = [
      actions.createToken('acme', '   ', 30),
      actions.createToken('acme', 'x'.repeat(101), 30),
      actions.createToken('acme', 'ci', 7 as never),
      actions.createToken('acme', 'ci', '30' as never),
      actions.createToken('Bad Slug', 'ci', 30),
      actions.revokeToken('acme', '../x'),
      actions.revokeToken('Bad Slug', ID),
    ];
    for (const r of await Promise.all(bad)) {
      expect(r.ok).toBe(false);
    }
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
