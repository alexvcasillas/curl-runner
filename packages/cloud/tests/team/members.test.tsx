import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };
const incoming = { current: new Headers({ 'x-pathname': '/t/acme/members' }) };

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

const { MembersView } = await import('@/components/MembersView');
const actions = await import('@/app/(app)/t/[team]/members/actions');
const { resetEnv } = await import('@/lib/env');

const ADA = { user_id: 'u1', email: 'ada@acme.com', name: 'Ada Lovelace', role: 'owner' as const };
const GRACE = {
  user_id: 'u2',
  email: 'grace@acme.com',
  name: 'Grace Hopper',
  role: 'owner' as const,
};
const LINUS = {
  user_id: 'u3',
  email: 'linus@acme.com',
  name: 'Linus Torvalds',
  role: 'member' as const,
};
const INVITE = {
  id: '3f2b8a52-8f2d-4a3e-9f53-0e2d2f5a7c11',
  email: 'dana@acme.com',
  role: 'member' as const,
  expires: 'Expires in 5 days',
};

type Props = React.ComponentProps<typeof MembersView>;

function setup(over: Partial<Props> = {}) {
  const fns = {
    onInvite: vi.fn(async (email: string, _role: string) => ({ ok: true as const, email })),
    onRole: vi.fn(async (_id: string, _role: string) => ({ ok: true as const })),
    onRemove: vi.fn(async (_id: string) => ({ ok: true as const })),
    onCancel: vi.fn(async (_id: string) => ({ ok: true as const })),
    onLeave: vi.fn(async () => ({ ok: true as const })),
  };
  const user = userEvent.setup();
  render(
    <MembersView
      team="acme"
      teamName="Acme"
      me={{ id: 'u1', email: 'ada@acme.com' }}
      owner
      plan="team"
      billing
      members={[ADA, GRACE, LINUS]}
      invites={[INVITE]}
      costNote={null}
      {...fns}
      {...over}
    />,
  );
  return { user, ...fns };
}

describe('MembersView, owner', () => {
  it('lists members with roles and the seat note', () => {
    setup();
    expect(screen.getByText('Linus Torvalds')).toBeInTheDocument();
    expect(screen.getByText('linus@acme.com')).toBeInTheDocument();
    expect(screen.getByText('(you)')).toBeInTheDocument();
    expect(
      screen.getByText('3 members on the Team plan, billed per member. Customers are never seats.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('combobox', { name: 'Role for Linus Torvalds' })).toHaveValue('member');
  });

  it('shows Remove for others and Leave team for me', () => {
    setup();
    expect(screen.getAllByRole('button', { name: /^Remove/ })).toHaveLength(2);
    expect(screen.getAllByRole('button', { name: 'Leave team' })).toHaveLength(1);
  });

  it('invites with an email and a role', async () => {
    const { user, onInvite } = setup();
    await user.type(screen.getByLabelText('Email address'), ' Linus2@Acme.com ');
    await user.selectOptions(screen.getByLabelText('Role'), 'owner');
    await user.click(screen.getByRole('button', { name: 'Send invite' }));
    expect(onInvite).toHaveBeenCalledWith('linus2@acme.com', 'owner');
    expect(await screen.findByRole('status')).toHaveTextContent(
      'Invite sent to linus2@acme.com. The link works for 7 days.',
    );
    expect(screen.getByLabelText('Email address')).toHaveValue('');
  });

  it('says so without calling the API when the email is already a member', async () => {
    const { user, onInvite } = setup();
    await user.type(screen.getByLabelText('Email address'), 'grace@acme.com');
    await user.click(screen.getByRole('button', { name: 'Send invite' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'grace@acme.com is already a member of Acme.',
    );
    expect(onInvite).not.toHaveBeenCalled();
  });

  it('checks the address before sending', async () => {
    const { user, onInvite } = setup();
    await user.type(screen.getByLabelText('Email address'), 'not-an-email');
    await user.click(screen.getByRole('button', { name: 'Send invite' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Enter a valid email address.');
    expect(onInvite).not.toHaveBeenCalled();
  });

  it('shows the 402 copy with a Billing link when billing is on', async () => {
    const { user } = setup({
      onInvite: vi.fn(async () => ({
        ok: false as const,
        message: 'The Free plan allows 3 members. Upgrade to add more.',
        code: 'plan_limit' as const,
      })),
    });
    await user.type(screen.getByLabelText('Email address'), 'new@acme.com');
    await user.click(screen.getByRole('button', { name: 'Send invite' }));
    const note = await screen.findByRole('status');
    expect(note).toHaveTextContent(
      'The Free plan allows 3 members, counting pending invites. Upgrade to add more, or cancel an invite to free a seat.',
    );
    expect(within(note).getByRole('link', { name: 'Upgrade to Team' })).toHaveAttribute(
      'href',
      '/t/acme/billing',
    );
  });

  it('has no Billing link on the 402 copy when billing is off', async () => {
    const { user } = setup({
      billing: false,
      onInvite: vi.fn(async () => ({
        ok: false as const,
        message: 'x',
        code: 'plan_limit' as const,
      })),
    });
    await user.type(screen.getByLabelText('Email address'), 'new@acme.com');
    await user.click(screen.getByRole('button', { name: 'Send invite' }));
    const note = await screen.findByRole('status');
    expect(note).toHaveTextContent('The Free plan allows 3 members, counting pending invites.');
    expect(within(note).queryByRole('link')).not.toBeInTheDocument();
  });

  it('turns the form off when a Free team already has 3 seats', () => {
    setup({ plan: 'free', members: [ADA, GRACE], invites: [INVITE] });
    expect(screen.getByLabelText('Email address')).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Send invite' })).toBeDisabled();
    expect(screen.getByRole('status')).toHaveTextContent('The Free plan allows 3 members');
    expect(screen.getByRole('link', { name: 'Upgrade to Team' })).toBeInTheDocument();
    expect(
      screen.getByText('2 members and 1 pending invite: 3 of 3 on the Free plan.'),
    ).toBeInTheDocument();
  });

  it('shows the seat cost note when the page has one', () => {
    setup({ costNote: 'On the Team plan, each member who accepts adds $8 a month.' });
    expect(
      screen.getByText('On the Team plan, each member who accepts adds $8 a month.'),
    ).toBeInTheDocument();
  });

  it('lists pending invites and cancels one behind a confirm', async () => {
    const { user, onCancel } = setup();
    expect(screen.getByText('Expires in 5 days')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Cancel invite to dana@acme.com' }));
    const dialog = await screen.findByRole('alertdialog', {
      name: 'Cancel the invite to dana@acme.com?',
    });
    expect(dialog).toHaveTextContent('The link in their email stops working.');
    await user.click(within(dialog).getByRole('button', { name: 'Cancel invite' }));
    expect(onCancel).toHaveBeenCalledWith(INVITE.id);
  });

  it('keeps an invite on Keep invite', async () => {
    const { user, onCancel } = setup();
    await user.click(screen.getByRole('button', { name: 'Cancel invite to dana@acme.com' }));
    const dialog = await screen.findByRole('alertdialog');
    await user.click(within(dialog).getByRole('button', { name: 'Keep invite' }));
    expect(onCancel).not.toHaveBeenCalled();
  });

  it('changes a role', async () => {
    const { user, onRole } = setup();
    await user.selectOptions(
      screen.getByRole('combobox', { name: 'Role for Linus Torvalds' }),
      'owner',
    );
    expect(onRole).toHaveBeenCalledWith('u3', 'owner');
  });

  it('shows the API message when a role change is refused', async () => {
    const { user } = setup({
      onRole: vi.fn(async () => ({
        ok: false as const,
        message: 'A team needs at least one owner. Make someone else an owner first.',
      })),
    });
    await user.selectOptions(
      screen.getByRole('combobox', { name: 'Role for Grace Hopper' }),
      'member',
    );
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'A team needs at least one owner. Make someone else an owner first.',
    );
  });

  it('removes a member behind a confirm, and names the seat only on Team with billing', async () => {
    const { user, onRemove } = setup();
    await user.click(screen.getByRole('button', { name: 'Remove Linus Torvalds' }));
    const dialog = await screen.findByRole('alertdialog', {
      name: 'Remove Linus Torvalds from Acme?',
    });
    expect(dialog).toHaveTextContent(
      'They lose access to Acme’s collections and tokens straight away.',
    );
    expect(dialog).toHaveTextContent('The next invoice drops one seat');
    await user.click(within(dialog).getByRole('button', { name: 'Remove member' }));
    expect(onRemove).toHaveBeenCalledWith('u3');
  });

  it('leaves out the invoice sentence without billing', async () => {
    const { user } = setup({ billing: false });
    await user.click(screen.getByRole('button', { name: 'Remove Linus Torvalds' }));
    const dialog = await screen.findByRole('alertdialog');
    expect(dialog).not.toHaveTextContent('invoice');
  });

  it('leaves the team behind a confirm', async () => {
    const { user, onLeave } = setup();
    await user.click(screen.getByRole('button', { name: 'Leave team' }));
    const dialog = await screen.findByRole('alertdialog', { name: 'Leave Acme?' });
    expect(dialog).toHaveTextContent('Collections shared with ada@acme.com still open for you');
    await user.click(within(dialog).getByRole('button', { name: 'Leave team' }));
    expect(onLeave).toHaveBeenCalled();
  });

  it('tells the only owner to make someone else an owner first', async () => {
    const { user, onLeave } = setup({ members: [ADA, LINUS] });
    await user.click(screen.getByRole('button', { name: 'Leave team' }));
    const dialog = await screen.findByRole('alertdialog', {
      name: 'Make someone else an owner first',
    });
    expect(dialog).toHaveTextContent(
      'A team needs at least one owner, and you’re the only one in Acme.',
    );
    expect(onLeave).not.toHaveBeenCalled();
  });

  it('shows the API message when leaving is refused', async () => {
    const { user } = setup({
      onLeave: vi.fn(async () => ({
        ok: false as const,
        message: 'A team needs at least one owner. Make someone else an owner first.',
      })),
    });
    await user.click(screen.getByRole('button', { name: 'Leave team' }));
    const dialog = await screen.findByRole('alertdialog', { name: 'Leave Acme?' });
    await user.click(within(dialog).getByRole('button', { name: 'Leave team' }));
    expect(await within(dialog).findByRole('alert')).toHaveTextContent(
      'A team needs at least one owner.',
    );
  });
});

describe('MembersView, member', () => {
  const asLinus = {
    me: { id: 'u3', email: 'linus@acme.com' },
    owner: false,
  };

  it('has no invite form, no role controls, no Remove and no Cancel invite', () => {
    setup(asLinus);
    expect(screen.queryByLabelText('Email address')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Send invite' })).not.toBeInTheDocument();
    expect(screen.queryByRole('combobox')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /^Remove/ })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /^Cancel invite/ })).not.toBeInTheDocument();
    expect(screen.getAllByText('Owner').length).toBeGreaterThan(0);
  });

  it('can still leave', async () => {
    const { user, onLeave } = setup(asLinus);
    await user.click(screen.getByRole('button', { name: 'Leave team' }));
    const dialog = await screen.findByRole('alertdialog', { name: 'Leave Acme?' });
    await user.click(within(dialog).getByRole('button', { name: 'Leave team' }));
    await waitFor(() => expect(onLeave).toHaveBeenCalled());
  });
});

describe('member actions', () => {
  const fetchMock = vi.fn();
  const U = '9a1b8a52-8f2d-4a3e-9f53-0e2d2f5a7c22';
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

  it('invites with the normalized email and role', async () => {
    fetchMock.mockResolvedValue(
      json(201, {
        id: INVITE.id,
        email: 'a@b.com',
        role: 'member',
        expires_at: '2026-11-12T00:00:00Z',
      }),
    );
    expect(await actions.inviteMember('acme', ' A@B.com ', 'member')).toEqual({
      ok: true,
      email: 'a@b.com',
    });
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('http://api.test/v1/teams/acme/invites');
    expect(JSON.parse(init.body)).toEqual({ email: 'a@b.com', role: 'member' });
    expect(revalidatePath).toHaveBeenCalledWith('/t/acme/members');
  });

  it('keeps the plan_limit code so the page can show its own copy', async () => {
    fetchMock.mockResolvedValue(
      json(402, {
        error: 'plan_limit',
        message: 'The Free plan allows 3 members. Upgrade to add more.',
      }),
    );
    expect(await actions.inviteMember('acme', 'a@b.com', 'member')).toEqual({
      ok: false,
      message: 'The Free plan allows 3 members. Upgrade to add more.',
      code: 'plan_limit',
    });
  });

  it('revokes an invite with DELETE …/invites/{id}', async () => {
    fetchMock.mockResolvedValue(new Response(null, { status: 204 }));
    expect(await actions.cancelInvite('acme', INVITE.id)).toEqual({ ok: true });
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe(`http://api.test/v1/teams/acme/invites/${INVITE.id}`);
    expect(init.method).toBe('DELETE');
  });

  it('changes a role with PATCH and removes with DELETE', async () => {
    fetchMock.mockResolvedValue(json(200, { user_id: U, role: 'owner' }));
    expect(await actions.changeRole('acme', U, 'owner')).toEqual({ ok: true });
    expect(fetchMock.mock.calls[0][0]).toBe(`http://api.test/v1/teams/acme/members/${U}`);
    expect(fetchMock.mock.calls[0][1].method).toBe('PATCH');
    expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toEqual({ role: 'owner' });
    fetchMock.mockResolvedValue(new Response(null, { status: 204 }));
    expect(await actions.removeMember('acme', U)).toEqual({ ok: true });
    expect(fetchMock.mock.calls[1][1].method).toBe('DELETE');
  });

  it('leaves as the signed-in user, then goes home', async () => {
    fetchMock
      .mockResolvedValueOnce(
        json(200, {
          user: { id: 'u1', email: 'ada@acme.com', name: null },
          teams: [{ slug: 'acme', name: 'Acme', role: 'member', plan: 'free' }],
          admin: false,
        }),
      )
      .mockResolvedValueOnce(new Response(null, { status: 204 }));
    await expect(actions.leaveTeam('acme')).rejects.toThrow('NEXT_REDIRECT /');
    expect(fetchMock.mock.calls[1][0]).toBe('http://api.test/v1/teams/acme/members/u1');
    expect(fetchMock.mock.calls[1][1].method).toBe('DELETE');
  });

  it('shows the last-owner message and stays put', async () => {
    fetchMock
      .mockResolvedValueOnce(
        json(200, {
          user: { id: 'u1', email: 'ada@acme.com', name: null },
          teams: [{ slug: 'acme', name: 'Acme', role: 'owner', plan: 'free' }],
          admin: false,
        }),
      )
      .mockResolvedValueOnce(
        json(409, {
          error: 'conflict',
          message: 'A team needs at least one owner. Make someone else an owner first.',
        }),
      );
    expect(await actions.leaveTeam('acme')).toEqual({
      ok: false,
      message: 'A team needs at least one owner. Make someone else an owner first.',
      code: 'conflict',
    });
  });

  it('rejects inputs the page never sends, without calling the API', async () => {
    const bad = [
      actions.inviteMember('acme', 'nope', 'member'),
      actions.inviteMember('acme', '*@acme.com', 'member'),
      actions.inviteMember('acme', 'a@b.com', 'admin' as never),
      actions.inviteMember('Bad Slug', 'a@b.com', 'member'),
      actions.changeRole('acme', '../x', 'owner'),
      actions.changeRole('acme', U, 'root' as never),
      actions.removeMember('acme', '..'),
      actions.cancelInvite('acme', 'x'),
    ];
    for (const r of await Promise.all(bad)) {
      expect(r.ok).toBe(false);
    }
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
