import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };
const incoming = { current: new Headers({ 'x-pathname': '/t/acme/c/payments-api/sharing' }) };

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
vi.mock('next/cache', () => ({ revalidatePath: vi.fn() }));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { SharePanel } = await import('@/components/SharePanel');
const actions = await import('@/app/(app)/t/[team]/c/[slug]/share-actions');
const { resetEnv } = await import('@/lib/env');

type Share = import('@/lib/types').Share;
const LINK = 'https://cloud.example.com/s/crsl_abc';
const mk = (over: Partial<Share> = {}): Share => ({
  access: 'public',
  link: LINK,
  expires_at: null,
  entries: [],
  ...over,
});

function setup(
  share: Share,
  { plan = 'team', billing = true }: { plan?: 'free' | 'team'; billing?: boolean } = {},
) {
  const onPatch = vi.fn(async (body: object) => ({
    ok: true as const,
    share: {
      ...share,
      ...('access' in body ? { access: (body as { access: Share['access'] }).access } : {}),
      ...('listed' in body ? { listed: (body as { listed: boolean }).listed } : {}),
      ...('description' in body
        ? { description: (body as { description: string }).description }
        : {}),
    },
  }));
  const onAdd = vi.fn(async (entry: string, notify: boolean) => ({
    ok: true as const,
    share: {
      ...share,
      entries: [
        ...share.entries,
        {
          entry,
          kind: entry.startsWith('*@') ? 'domain' : 'email',
          invited_at: '2026-11-05T10:00:00Z',
          opened: false,
        },
      ],
    },
    notify,
  }));
  const onRemove = vi.fn(async (entry: string) => ({
    ok: true as const,
    share: { ...share, entries: share.entries.filter((e) => e.entry !== entry) },
  }));
  const user = userEvent.setup();
  render(
    <SharePanel
      team="acme"
      slug="payments-api"
      teamName="Acme"
      name="Payments API"
      plan={plan}
      billing={billing}
      share={share}
      now="2026-11-05T10:00:00Z"
      onPatch={onPatch}
      onAdd={onAdd}
      onRemove={onRemove}
    />,
  );
  return { user, onPatch, onAdd, onRemove };
}

const SWITCH = 'List Payments API in the gallery';

const entry = (e: string, opened = false) => ({
  entry: e,
  kind: e.startsWith('*@') ? 'domain' : 'email',
  invited_at: '2026-11-03T10:00:00Z',
  opened,
});

describe('SharePanel gallery', () => {
  it('is hidden unless access is Public', () => {
    setup(mk({ access: 'off', description: 'x' }));
    expect(screen.queryByRole('switch')).toBeNull();
    expect(screen.queryByRole('heading', { name: 'Public gallery' })).toBeNull();
  });

  it('is hidden when restricted', () => {
    setup(mk({ access: 'restricted', description: 'x' }));
    expect(screen.queryByRole('switch')).toBeNull();
  });

  it('is disabled with the hint when there is no description', () => {
    setup(mk());
    expect(screen.getByRole('heading', { name: 'Public gallery' })).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: SWITCH })).toBeDisabled();
    expect(screen.getByRole('switch', { name: SWITCH })).toHaveAttribute('aria-checked', 'false');
    const card = screen.getByRole('region', { name: 'Public gallery' });
    expect(card).toHaveTextContent(
      'Add a description first. It’s the summary people read on curl-runner.com/collections.',
    );
    expect(within(card).queryByRole('link')).toBeNull();
    expect(within(card).getByRole('button', { name: 'Save description' })).toBeDisabled();
  });

  it('saves a description without listing it, then lists with the switch', async () => {
    const { user, onPatch } = setup(mk());
    await user.type(screen.getByLabelText('Description'), '  Charges and refunds ');
    expect(screen.getByText('19 / 500')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Save description' }));
    expect(onPatch).toHaveBeenCalledWith({ description: 'Charges and refunds' });
    expect(await screen.findByText('Charges and refunds')).toBeInTheDocument();
    expect(screen.queryByLabelText('Description')).toBeNull();
    expect(screen.getByRole('switch', { name: SWITCH })).toBeEnabled();
    // Edit brings the field back with the text.
    await user.click(screen.getByRole('button', { name: 'Edit' }));
    expect(screen.getByLabelText('Description')).toHaveValue('Charges and refunds');
    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByLabelText('Description')).toBeNull();
  });

  it('treats a blank description as none', () => {
    setup(mk({ description: '   ', listed: false }));
    expect(screen.getByRole('switch', { name: SWITCH })).toBeDisabled();
  });

  it('is off with the sentence and no link when there is a description', () => {
    setup(mk({ description: 'Pay stuff', listed: false }));
    const sw = screen.getByRole('switch', { name: SWITCH });
    expect(sw).toBeEnabled();
    expect(sw).toHaveAttribute('aria-checked', 'false');
    const card = screen.getByRole('region', { name: 'Public gallery' });
    expect(card).toHaveTextContent(
      'Anyone can find it on curl-runner.com: the name, description, Acme, the requests and their YAML. Variable values never are.',
    );
    expect(within(card).queryByRole('link')).toBeNull();
  });

  it('sends listed: true, then renders the listed state with a link', async () => {
    const { user, onPatch } = setup(mk({ description: 'Pay stuff', listed: false }));
    await user.click(screen.getByRole('switch', { name: SWITCH }));
    expect(onPatch).toHaveBeenCalledWith({ listed: true });
    expect(await screen.findByRole('switch', { name: SWITCH, checked: true })).toBeEnabled();
    expect(screen.getByRole('link', { name: 'View it on curl-runner.com' })).toHaveAttribute(
      'href',
      'https://www.curl-runner.com/collections/acme/payments-api',
    );
  });

  it('sends listed: false from the listed state', async () => {
    const { user, onPatch } = setup(mk({ description: 'Pay stuff', listed: true }));
    expect(screen.getByRole('link', { name: 'View it on curl-runner.com' })).toBeInTheDocument();
    await user.click(screen.getByRole('switch', { name: SWITCH }));
    expect(onPatch).toHaveBeenCalledWith({ listed: false });
    expect(await screen.findByRole('switch', { name: SWITCH, checked: false })).toBeInTheDocument();
    expect(screen.queryByRole('link', { name: 'View it on curl-runner.com' })).toBeNull();
  });

  it('still lets a listed collection without a description be unlisted', () => {
    setup(mk({ listed: true, description: null }));
    expect(screen.getByRole('switch', { name: SWITCH })).toBeEnabled();
  });

  it('shows the server error and leaves the switch where it was', async () => {
    const { user, onPatch } = setup(mk({ description: 'Pay stuff', listed: false }));
    onPatch.mockResolvedValueOnce({ ok: false, message: 'Add a description first.' } as never);
    await user.click(screen.getByRole('switch', { name: SWITCH }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Add a description first.');
    expect(screen.getByRole('switch', { name: SWITCH })).toHaveAttribute('aria-checked', 'false');
  });

  it('follows the share the server sends back after access changes', async () => {
    const { user, onPatch } = setup(mk({ description: 'Pay stuff', listed: true }));
    onPatch.mockResolvedValueOnce({
      ok: true,
      share: mk({ access: 'off', listed: false, description: 'Pay stuff' }),
    } as never);
    await user.click(screen.getByRole('radio', { name: /Off/ }));
    await waitFor(() => expect(screen.queryByRole('switch')).toBeNull());
  });
});

describe('SharePanel', () => {
  it('shows the link and disables Copy and Reset when sharing is off', () => {
    setup(mk({ access: 'off' }));
    expect(screen.getByRole('radio', { name: /Off/ })).toBeChecked();
    expect(screen.getByRole('button', { name: 'Copy link' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Reset link' })).toBeDisabled();
    expect(screen.queryByRole('heading', { name: 'People with access' })).not.toBeInTheDocument();
  });

  it('switches Off to Public straight away', async () => {
    const { user, onPatch } = setup(mk({ access: 'off' }));
    await user.click(screen.getByRole('radio', { name: /Public/ }));
    expect(onPatch).toHaveBeenCalledWith({ access: 'public' });
  });

  it('disables Restricted on a Free team, labels it and links to Billing', () => {
    setup(mk(), { plan: 'free' });
    const restricted = screen.getByRole('radio', { name: /Restricted/ });
    expect(restricted).toBeDisabled();
    expect(screen.getAllByText('Team plan').length).toBeGreaterThan(0);
    expect(screen.getByRole('link', { name: /Upgrade Acme/ })).toHaveAttribute(
      'href',
      '/t/acme/billing',
    );
  });

  it('has no Billing link when billing is off', () => {
    setup(mk(), { plan: 'free', billing: false });
    expect(screen.getByRole('radio', { name: /Restricted/ })).toBeDisabled();
    expect(screen.queryByRole('link', { name: /Upgrade/ })).not.toBeInTheDocument();
  });

  it('warns before Public becomes Restricted, and Keep public cancels', async () => {
    const { user, onPatch } = setup(mk());
    await user.click(screen.getByRole('radio', { name: /Restricted/ }));
    expect(onPatch).not.toHaveBeenCalled();
    const alert = screen.getByRole('alert');
    expect(alert).toHaveTextContent(
      'The link will need sign-in. Anyone already using it who isn’t on the list stops getting updates.',
    );
    await user.click(within(alert).getByRole('button', { name: 'Keep public' }));
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(onPatch).not.toHaveBeenCalled();

    await user.click(screen.getByRole('radio', { name: /Restricted/ }));
    await user.click(screen.getByRole('button', { name: 'Make restricted' }));
    expect(onPatch).toHaveBeenCalledWith({ access: 'restricted' });
    expect(await screen.findByRole('heading', { name: 'People with access' })).toBeInTheDocument();
  });

  it('Off to Restricted needs no warning', async () => {
    const { user, onPatch } = setup(mk({ access: 'off' }));
    await user.click(screen.getByRole('radio', { name: /Restricted/ }));
    expect(onPatch).toHaveBeenCalledWith({ access: 'restricted' });
  });

  it('shows the API message when the server refuses with 402', async () => {
    const { user, onPatch } = setup(mk({ access: 'off' }));
    onPatch.mockResolvedValueOnce({
      ok: false,
      message: 'Restricted sharing needs the Team plan.',
    } as never);
    await user.click(screen.getByRole('radio', { name: /Restricted/ }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Restricted sharing needs the Team plan.',
    );
    expect(screen.getByRole('radio', { name: /Off/ })).toBeChecked();
  });

  it('copies the link', async () => {
    const { user } = setup(mk());
    const writeText = vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue();
    await user.click(screen.getByRole('button', { name: 'Copy link' }));
    expect(writeText).toHaveBeenCalledWith(LINK);
    expect(screen.getByRole('button', { name: 'Copied' })).toBeInTheDocument();
  });

  it('confirms before resetting, then sends reset_link and shows the new link', async () => {
    const { user, onPatch } = setup(mk());
    onPatch.mockResolvedValueOnce({
      ok: true,
      share: mk({ link: 'https://cloud.example.com/s/crsl_new' }),
    } as never);
    await user.click(screen.getByRole('button', { name: 'Reset link' }));
    expect(onPatch).not.toHaveBeenCalled();
    const alert = screen.getByRole('alert');
    expect(alert).toHaveTextContent('The old link stops working right away');
    await user.click(within(alert).getByRole('button', { name: 'Cancel' }));
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Reset link' }));
    await user.click(within(screen.getByRole('alert')).getByRole('button', { name: 'Reset link' }));
    expect(onPatch).toHaveBeenCalledWith({ reset_link: true });
    await waitFor(() =>
      expect(screen.getByRole('textbox', { name: 'Share link' })).toHaveValue(
        'https://cloud.example.com/s/crsl_new',
      ),
    );
  });

  it('maps expiry to expires_in_days: 7, 30 or null', async () => {
    const { user, onPatch } = setup(mk());
    const select = screen.getByLabelText('Expires');
    await user.selectOptions(select, 'In 7 days');
    expect(onPatch).toHaveBeenLastCalledWith({ expires_in_days: 7 });
    await user.selectOptions(select, 'In 30 days');
    expect(onPatch).toHaveBeenLastCalledWith({ expires_in_days: 30 });
    await user.selectOptions(select, 'Never');
    expect(onPatch).toHaveBeenLastCalledWith({ expires_in_days: null });
  });

  it('shows an existing expiry as its nearest choice', () => {
    setup(mk({ expires_at: '2026-11-12T10:00:00Z' }));
    expect(screen.getByLabelText('Expires')).toHaveValue('7');
  });

  describe('access list', () => {
    const restricted = () =>
      mk({
        access: 'restricted',
        entries: [
          entry('jane@globex.example', true),
          entry('*@initech.example', true),
          entry('sam@umbrella.example'),
        ],
      });

    it('lists entries with kind and status', () => {
      setup(restricted());
      const rows = screen.getAllByRole('listitem');
      expect(rows).toHaveLength(3);
      expect(within(rows[0]).getByText('Email')).toBeInTheDocument();
      expect(within(rows[0]).getByText('Opened')).toBeInTheDocument();
      expect(within(rows[1]).getByText('Domain · anyone at initech.example')).toBeInTheDocument();
      expect(within(rows[2]).getByText('Invited · not opened')).toBeInTheDocument();
    });

    it('says so when the list is empty', () => {
      setup(mk({ access: 'restricted' }));
      expect(screen.getByText('Nobody on the list yet.')).toBeInTheDocument();
    });

    it('adds an email with notify on by default', async () => {
      const { user, onAdd } = setup(mk({ access: 'restricted' }));
      expect(screen.getByRole('checkbox', { name: /Email them the link/ })).toBeChecked();
      await user.type(screen.getByLabelText('Email or domain'), 'ada+cust@example.com');
      await user.click(screen.getByRole('button', { name: 'Add' }));
      expect(onAdd).toHaveBeenCalledWith('ada+cust@example.com', true);
      expect(await screen.findByText('ada+cust@example.com')).toBeInTheDocument();
      expect(screen.getByLabelText('Email or domain')).toHaveValue('');
    });

    it('adds a domain entry and sends notify off when unchecked', async () => {
      const { user, onAdd } = setup(mk({ access: 'restricted' }));
      await user.click(screen.getByRole('checkbox', { name: /Email them the link/ }));
      await user.type(screen.getByLabelText('Email or domain'), '*@customer.com');
      await user.click(screen.getByRole('button', { name: 'Add' }));
      expect(onAdd).toHaveBeenCalledWith('*@customer.com', false);
      expect(await screen.findByText('Domain · anyone at customer.com')).toBeInTheDocument();
    });

    it('rejects an invalid entry inline without calling the API', async () => {
      const { user, onAdd } = setup(mk({ access: 'restricted' }));
      await user.type(screen.getByLabelText('Email or domain'), 'foo');
      await user.click(screen.getByRole('button', { name: 'Add' }));
      expect(
        screen.getByText('Enter an email like ada@customer.com, or a domain like *@customer.com.'),
      ).toBeInTheDocument();
      expect(onAdd).not.toHaveBeenCalled();
    });

    it('trims and lower-cases before sending', async () => {
      const { user, onAdd } = setup(mk({ access: 'restricted' }));
      await user.type(screen.getByLabelText('Email or domain'), '  *@Customer.com ');
      await user.click(screen.getByRole('button', { name: 'Add' }));
      expect(onAdd).toHaveBeenCalledWith('*@customer.com', true);
    });

    it('shows the error from a refused add and keeps the text', async () => {
      const { user, onAdd } = setup(mk({ access: 'restricted' }));
      onAdd.mockResolvedValueOnce({ ok: false, message: 'An entry is an email.' } as never);
      await user.type(screen.getByLabelText('Email or domain'), 'nope@example.com');
      await user.click(screen.getByRole('button', { name: 'Add' }));
      expect(await screen.findByText('An entry is an email.')).toBeInTheDocument();
      expect(screen.getByLabelText('Email or domain')).toHaveValue('nope@example.com');
    });

    it('confirms before removing', async () => {
      const { user, onRemove } = setup(restricted());
      await user.click(screen.getByRole('button', { name: 'Remove sam@umbrella.example' }));
      const dialog = await screen.findByRole('alertdialog');
      expect(dialog).toHaveTextContent('Remove sam@umbrella.example?');
      expect(dialog).toHaveTextContent('Their local copy stays on their machine.');
      expect(onRemove).not.toHaveBeenCalled();
      await user.click(within(dialog).getByRole('button', { name: 'Remove access' }));
      expect(onRemove).toHaveBeenCalledWith('sam@umbrella.example');
      await waitFor(() =>
        expect(screen.queryByText('sam@umbrella.example')).not.toBeInTheDocument(),
      );
    });
  });
});

describe('share actions', () => {
  const fetchMock = vi.fn();
  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
    vi.stubEnv('API_INTERNAL_URL', 'http://api.test');
    vi.stubEnv('PUBLIC_APP_URL', 'http://localhost:3000');
    resetEnv();
  });
  const json = (status: number, body: unknown) =>
    new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

  it('patches the share and returns it', async () => {
    fetchMock.mockResolvedValue(json(200, mk()));
    const result = await actions.patchShare('acme', 'payments-api', { expires_in_days: null });
    expect(result).toEqual({ ok: true, share: mk() });
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('http://api.test/v1/teams/acme/collections/payments-api/share');
    expect(init.method).toBe('PATCH');
    expect(JSON.parse(init.body)).toEqual({ expires_in_days: null });
  });

  it('turns a 402 into its message', async () => {
    fetchMock.mockResolvedValue(
      json(402, { error: 'plan_limit', message: 'Restricted sharing needs the Team plan.' }),
    );
    expect(await actions.patchShare('acme', 'payments-api', { access: 'restricted' })).toEqual({
      ok: false,
      message: 'Restricted sharing needs the Team plan.',
    });
  });

  it('adds an entry then reads the list back', async () => {
    fetchMock
      .mockResolvedValueOnce(json(201, entry('ada+cust@example.com')))
      .mockResolvedValueOnce(json(200, mk({ entries: [entry('ada+cust@example.com')] })));
    const result = await actions.addEntry('acme', 'payments-api', 'ada+cust@example.com', true);
    expect(result.ok).toBe(true);
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('http://api.test/v1/teams/acme/collections/payments-api/share/entries');
    expect(JSON.parse(init.body)).toEqual({ entry: 'ada+cust@example.com', notify: true });
  });

  it('url-encodes the entry on remove', async () => {
    fetchMock
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
      .mockResolvedValueOnce(json(200, mk()));
    await actions.removeEntry('acme', 'payments-api', '*@customer.com');
    expect(fetchMock.mock.calls[0][0]).toBe(
      'http://api.test/v1/teams/acme/collections/payments-api/share/entries/*%40customer.com',
    );
    expect(fetchMock.mock.calls[0][1].method).toBe('DELETE');
  });

  it('rejects inputs the page never sends, without calling the API', async () => {
    const bad = [
      actions.patchShare('acme', 'payments-api', { access: 'admin' } as never),
      actions.patchShare('acme', 'payments-api', { expires_in_days: 5 } as never),
      actions.patchShare('acme', 'payments-api', { description: 'x'.repeat(501) }),
      actions.patchShare('acme', 'payments-api', { expires_in_days: '7' } as never),
      actions.patchShare('acme', 'payments-api', { reset_link: 'yes' } as never),
      actions.patchShare('acme', 'payments-api', { extra: 1 } as never),
      actions.addEntry('acme', 'payments-api', 'foo', true),
      actions.addEntry('acme', 'payments-api', 'a@b.com', 'yes' as never),
      actions.removeEntry('acme', 'payments-api', '..'),
      actions.removeEntry('acme', 'payments-api', '../x'),
    ];
    for (const r of await Promise.all(bad)) {
      expect(r.ok).toBe(false);
    }
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('accepts listed and still rejects unknown keys and non-booleans', async () => {
    fetchMock.mockResolvedValue(json(200, mk({ listed: true })));
    expect(await actions.patchShare('acme', 'payments-api', { listed: true })).toMatchObject({
      ok: true,
    });
    expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toEqual({ listed: true });
    fetchMock.mockClear();
    for (const bad of [{ listed: 'yes' }, { listed: 1 }, { listed: true, extra: 1 }]) {
      expect((await actions.patchShare('acme', 'payments-api', bad as never)).ok).toBe(false);
    }
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('sends a lower-cased entry', async () => {
    fetchMock
      .mockResolvedValueOnce(json(201, entry('a@b.com')))
      .mockResolvedValueOnce(json(200, mk()));
    await actions.addEntry('acme', 'payments-api', ' A@B.com ', false);
    expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toEqual({
      entry: 'a@b.com',
      notify: false,
    });
  });

  it('rejects an invalid slug without calling the API', async () => {
    expect((await actions.patchShare('acme', 'Bad Slug', {})).ok).toBe(false);
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
