import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };
const incoming = { current: new Headers({ 'x-pathname': '/t/acme/settings' }) };

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

const { TeamSettings } = await import('@/components/TeamSettings');
const actions = await import('@/app/(app)/t/[team]/settings/actions');
const { resetEnv } = await import('@/lib/env');

type Props = React.ComponentProps<typeof TeamSettings>;

function setup(over: Partial<Props> = {}) {
  const fns = {
    onRename: vi.fn(async (_name: string) => ({ ok: true as const })),
    onDelete: vi.fn(async () => ({ ok: true as const })),
    onLeave: vi.fn(async () => ({ ok: true as const })),
  };
  const user = userEvent.setup();
  render(
    <TeamSettings
      team="acme"
      name="Acme"
      owner
      me={{ id: 'u1', email: 'ada@acme.example' }}
      soleOwner={false}
      slugHost="cloud.example.com"
      collections={4}
      tokens={3}
      subscription
      {...fns}
      {...over}
    />,
  );
  return { user, ...fns };
}

describe('TeamSettings, owner', () => {
  it('renames the team', async () => {
    const { user, onRename } = setup();
    const input = screen.getByLabelText('Team name');
    await user.clear(input);
    await user.type(input, '  Acme Labs ');
    await user.click(screen.getByRole('button', { name: 'Save' }));
    expect(onRename).toHaveBeenCalledWith('Acme Labs');
    expect(await screen.findByRole('status')).toHaveTextContent('Saved.');
  });

  it('keeps Save off until the name changes and refuses an empty one', async () => {
    const { user, onRename } = setup();
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
    await user.clear(screen.getByLabelText('Team name'));
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
    expect(onRename).not.toHaveBeenCalled();
  });

  it('shows the API message when a rename fails', async () => {
    const { user } = setup({
      onRename: vi.fn(async () => ({ ok: false as const, message: 'The team needs a name.' })),
    });
    await user.type(screen.getByLabelText('Team name'), 'x');
    await user.click(screen.getByRole('button', { name: 'Save' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('The team needs a name.');
  });

  it('shows the slug, which can’t change', () => {
    setup();
    expect(screen.getByText('cloud.example.com/t/')).toBeInTheDocument();
    expect(screen.getByText('acme', { selector: 'span span' })).toBeInTheDocument();
  });

  it('asks for the slug before it deletes', async () => {
    const { user, onDelete } = setup();
    await user.click(screen.getByRole('button', { name: 'Delete team' }));
    const dialog = await screen.findByRole('alertdialog', { name: 'Delete the Acme team' });
    expect(dialog).toHaveTextContent('This can’t be undone.');
    expect(dialog).toHaveTextContent('4 collections and 3 tokens stop working.');
    expect(dialog).toHaveTextContent('The subscription is cancelled now');
    const confirm = within(dialog).getByRole('button', { name: 'Delete team' });
    expect(confirm).toBeDisabled();
    await user.type(within(dialog).getByLabelText(/Type/), 'acm');
    expect(confirm).toBeDisabled();
    await user.type(within(dialog).getByLabelText(/Type/), 'e');
    expect(confirm).toBeEnabled();
    await user.click(confirm);
    expect(onDelete).toHaveBeenCalled();
  });

  it('leaves the subscription sentence out when nothing is billed', async () => {
    const { user } = setup({ subscription: false });
    await user.click(screen.getByRole('button', { name: 'Delete team' }));
    expect(await screen.findByRole('alertdialog')).not.toHaveTextContent('subscription');
  });

  it('shows the API message when the delete fails, such as Stripe being unreachable', async () => {
    const { user } = setup({
      onDelete: vi.fn(async () => ({
        ok: false as const,
        message: 'Billing is not reachable right now. Try again in a minute.',
      })),
    });
    await user.click(screen.getByRole('button', { name: 'Delete team' }));
    const dialog = await screen.findByRole('alertdialog');
    await user.type(within(dialog).getByLabelText(/Type/), 'acme');
    await user.click(within(dialog).getByRole('button', { name: 'Delete team' }));
    expect(await within(dialog).findByRole('alert')).toHaveTextContent(
      'Billing is not reachable right now. Try again in a minute.',
    );
  });
});

describe('TeamSettings, leave', () => {
  it('leaves behind a confirm', async () => {
    const { user, onLeave } = setup();
    await user.click(screen.getByRole('button', { name: 'Leave team' }));
    const dialog = await screen.findByRole('alertdialog', { name: 'Leave Acme?' });
    await user.click(within(dialog).getByRole('button', { name: 'Leave team' }));
    expect(onLeave).toHaveBeenCalled();
  });

  it('sends the only owner to Members instead of offering Leave team', () => {
    setup({ soleOwner: true });
    expect(screen.queryByRole('button', { name: 'Leave team' })).not.toBeInTheDocument();
    expect(
      screen.getByText('You’re the only owner. Make someone else an owner before you leave.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Go to members' })).toHaveAttribute(
      'href',
      '/t/acme/members',
    );
  });
});

describe('TeamSettings, member', () => {
  it('can’t rename or delete, and can still leave', () => {
    setup({ owner: false });
    expect(screen.getByLabelText('Team name')).toBeDisabled();
    expect(screen.queryByRole('button', { name: 'Save' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Delete team' })).not.toBeInTheDocument();
    expect(screen.getByText('Ask an owner to rename the team.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Leave team' })).toBeInTheDocument();
  });
});

describe('settings actions', () => {
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

  it('renames with PATCH', async () => {
    fetchMock.mockResolvedValue(
      json(200, { slug: 'acme', name: 'Acme Labs', plan: 'free', role: 'owner' }),
    );
    expect(await actions.renameTeam('acme', ' Acme Labs ')).toEqual({ ok: true });
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('http://api.test/v1/teams/acme');
    expect(init.method).toBe('PATCH');
    expect(JSON.parse(init.body)).toEqual({ name: 'Acme Labs' });
    expect(revalidatePath).toHaveBeenCalledWith('/', 'layout');
  });

  it('deletes with DELETE, then goes home', async () => {
    fetchMock.mockResolvedValue(new Response(null, { status: 204 }));
    await expect(actions.deleteTeam('acme')).rejects.toThrow('NEXT_REDIRECT /');
    expect(fetchMock.mock.calls[0][0]).toBe('http://api.test/v1/teams/acme');
    expect(fetchMock.mock.calls[0][1].method).toBe('DELETE');
  });

  it('keeps the team and shows the API message on a 503', async () => {
    fetchMock.mockResolvedValue(
      json(503, {
        error: 'unavailable',
        message: 'Billing is not reachable right now. Try again in a minute.',
      }),
    );
    expect(await actions.deleteTeam('acme')).toEqual({
      ok: false,
      message: 'Billing is not reachable right now. Try again in a minute.',
      code: 'unavailable',
    });
  });

  it('rejects inputs the page never sends, without calling the API', async () => {
    for (const r of await Promise.all([
      actions.renameTeam('acme', '   '),
      actions.renameTeam('acme', 'x'.repeat(101)),
      actions.renameTeam('Bad Slug', 'ok'),
      actions.renameTeam('acme', 5 as never),
    ])) {
      expect(r.ok).toBe(false);
    }
    await expect(actions.deleteTeam('Bad Slug')).resolves.toMatchObject({ ok: false });
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
