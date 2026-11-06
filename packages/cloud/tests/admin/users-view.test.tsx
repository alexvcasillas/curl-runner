import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Perm } from '@/lib/admin';
import { ADA, JANE, LINUS, NOW, page, rows } from './users-fixtures';

const push = vi.fn();
const replace = vi.fn();
vi.mock('next/navigation', () => ({ useRouter: () => ({ push, replace, refresh: vi.fn() }) }));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { UsersView } = await import('@/components/admin/UsersView');

const ALL: Perm[] = ['users', 'teams', 'billing', 'emails', 'settings', 'admins'];
const ok = { ok: true } as const;

function makeActions() {
  return {
    suspend: vi.fn(async () => ok),
    unsuspend: vi.fn(async () => ok),
    signOut: vi.fn(async () => ok),
    remove: vi.fn(async () => ok),
    email: vi.fn(async () => ok),
    bulk: vi.fn(async (ids: string[]) => ({ ok: true as const, done: ids.length, skipped: [] })),
  };
}

function show(
  over: Partial<React.ComponentProps<typeof UsersView>> = {},
  actions = makeActions(),
  permissions: Perm[] = ALL,
) {
  render(
    <UsersView
      page={page}
      q=""
      filter="all"
      trail={[]}
      pageSize={50}
      permissions={permissions}
      now={NOW}
      actions={actions}
      {...over}
    />,
  );
  return actions;
}

beforeEach(() => {
  push.mockReset();
  replace.mockReset();
});

describe('users list', () => {
  it('renders a row per user with the board columns', () => {
    show();
    const table = screen.getByRole('table', { name: 'Users' });
    const ada = within(table).getByRole('row', { name: /Ada Lovelace/ });
    expect(ada).toHaveTextContent('ada@acme.example');
    expect(ada).toHaveTextContent('Email · GitHub');
    expect(ada).toHaveTextContent('Acme (owner), Globex Partners');
    expect(ada).toHaveTextContent('6 minutes ago');
    expect(ada).toHaveTextContent('Active');
    const jane = within(table).getByRole('row', { name: /jane@globex.example/ });
    expect(jane).toHaveTextContent('None');
    expect(jane).toHaveTextContent('Suspended');
    expect(jane).toHaveTextContent('Never');
    expect(within(ada).getByRole('link', { name: /Ada Lovelace/ })).toHaveAttribute(
      'href',
      `/admin/users/${ADA}`,
    );
  });

  it('shows the position and total', () => {
    show();
    expect(screen.getByText('Showing 1–3 of 1,248')).toBeInTheDocument();
  });

  it('pages with the API cursor and a trail back', () => {
    show();
    expect(screen.getByRole('link', { name: 'Next' })).toHaveAttribute(
      'href',
      '/admin/users?cursor=CURSOR_2&trail=-',
    );
    expect(screen.getByRole('button', { name: 'Previous' })).toBeDisabled();
  });

  it('goes back one page through the trail', () => {
    show({ cursor: 'CURSOR_3', trail: ['-', 'CURSOR_2'], page: { ...page, next_cursor: null } });
    expect(screen.getByRole('link', { name: 'Previous' })).toHaveAttribute(
      'href',
      '/admin/users?cursor=CURSOR_2&trail=-',
    );
    expect(screen.getByText('Showing 101–103 of 1,248')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Next' })).toBeDisabled();
  });

  it('filters by chip and keeps the search', async () => {
    show({ q: 'ada' });
    const group = screen.getByRole('radiogroup', { name: 'Filter users' });
    expect(within(group).getAllByRole('radio')).toHaveLength(5);
    expect(within(group).getByRole('radio', { name: 'All' })).toBeChecked();
    await userEvent.click(within(group).getByRole('radio', { name: 'Suspended' }));
    expect(push).toHaveBeenCalledWith('/admin/users?q=ada&filter=suspended');
  });

  it('searches on submit', async () => {
    show();
    await userEvent.type(screen.getByRole('searchbox', { name: 'Search users' }), 'linus{Enter}');
    expect(push).toHaveBeenCalledWith('/admin/users?q=linus');
  });

  it('links Export CSV to the route with the current filters', () => {
    show({ q: 'ada', filter: 'members' });
    expect(screen.getByRole('link', { name: 'Export CSV' })).toHaveAttribute(
      'href',
      '/admin/users/export?q=ada&filter=members',
    );
  });

  it('says so when nothing matches', () => {
    show({ page: { items: [], next_cursor: null, total: 0 }, q: 'zzz' });
    expect(screen.getByText('No users match')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Clear search' })).toHaveAttribute(
      'href',
      '/admin/users',
    );
  });
});

describe('selection and the bulk bar', () => {
  it('shows the bar after a selection and clears it', async () => {
    show();
    expect(screen.queryByRole('toolbar')).toBeNull();
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Ada Lovelace' }));
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Linus Torvalds' }));
    const bar = screen.getByRole('toolbar', { name: 'Bulk actions' });
    expect(bar).toHaveTextContent('2 selected');
    expect(within(bar).getByRole('link', { name: 'Email them' })).toHaveAttribute(
      'href',
      '/admin/emails',
    );
    await userEvent.click(within(bar).getByRole('button', { name: 'Clear' }));
    expect(screen.queryByRole('toolbar')).toBeNull();
  });

  it('selects every row on the page', async () => {
    show();
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select all on this page' }));
    expect(screen.getByRole('toolbar')).toHaveTextContent('3 selected');
  });

  it('signs out the selection after a confirm and reports the count', async () => {
    const actions = show();
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Ada Lovelace' }));
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Linus Torvalds' }));
    await userEvent.click(
      within(screen.getByRole('toolbar')).getByRole('button', { name: 'Sign out everywhere' }),
    );
    const dialog = await screen.findByRole('alertdialog');
    expect(dialog).toHaveTextContent('Sign out 2 users everywhere?');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Sign out 2 users' }));
    await waitFor(() => expect(actions.bulk).toHaveBeenCalledWith([ADA, LINUS], 'sign_out', ''));
    expect(await screen.findByText('Signed out 2 users.')).toBeInTheDocument();
    expect(screen.queryByRole('toolbar')).toBeNull();
  });

  it('suspends the selection with a reason and names who was skipped', async () => {
    const actions = makeActions();
    actions.bulk.mockResolvedValue({ ok: true, done: 1, skipped: [LINUS] } as never);
    show({}, actions);
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Ada Lovelace' }));
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Linus Torvalds' }));
    await userEvent.click(
      within(screen.getByRole('toolbar')).getByRole('button', { name: 'Suspend' }),
    );
    const dialog = await screen.findByRole('dialog', { name: 'Suspend 2 users?' });
    expect(within(dialog).queryByRole('checkbox')).toBeNull();
    await userEvent.type(
      within(dialog).getByLabelText('Reason, for the activity log'),
      'spam ring',
    );
    await userEvent.click(within(dialog).getByRole('button', { name: 'Suspend 2 users' }));
    await waitFor(() =>
      expect(actions.bulk).toHaveBeenCalledWith([ADA, LINUS], 'suspend', 'spam ring'),
    );
    const note = await screen.findByText(/Suspended 1 user\./);
    expect(note).toHaveTextContent('Skipped Linus Torvalds');
    expect(note).toHaveTextContent(
      'Owners, you, other admins and anyone already suspended are left alone.',
    );
  });

  it('keeps the dialog open and shows the API message when the bulk call fails', async () => {
    const actions = makeActions();
    actions.bulk.mockResolvedValue({
      ok: false,
      message: 'Pick between 1 and 200 users.',
    } as never);
    show({}, actions);
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Ada Lovelace' }));
    await userEvent.click(
      within(screen.getByRole('toolbar')).getByRole('button', { name: 'Sign out everywhere' }),
    );
    const dialog = await screen.findByRole('alertdialog');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Sign out 1 user' }));
    expect(await within(dialog).findByText('Pick between 1 and 200 users.')).toBeInTheDocument();
  });
});

describe('row actions', () => {
  async function openMenu(name: string) {
    await userEvent.click(screen.getByRole('button', { name: `Actions for ${name}` }));
    return screen.getByRole('menu', { name: 'User actions' });
  }

  it('offers the board menu', async () => {
    show();
    const menu = await openMenu('Linus Torvalds');
    expect(within(menu).getByRole('menuitem', { name: 'Open' })).toHaveAttribute(
      'href',
      `/admin/users/${LINUS}`,
    );
    for (const name of ['Email Linus', 'Sign out everywhere', 'Suspend…', 'Delete user…']) {
      expect(within(menu).getByRole('menuitem', { name })).toBeInTheDocument();
    }
  });

  it('offers to lift a suspension instead of suspending', async () => {
    const actions = show();
    const menu = await openMenu('jane@globex.example');
    expect(within(menu).queryByRole('menuitem', { name: 'Suspend…' })).toBeNull();
    await userEvent.click(within(menu).getByRole('menuitem', { name: 'Lift suspension' }));
    const dialog = await screen.findByRole('alertdialog');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Lift suspension' }));
    await waitFor(() => expect(actions.unsuspend).toHaveBeenCalledWith(JANE));
  });

  it('suspends one user with a reason and the notify box', async () => {
    const actions = show();
    const menu = await openMenu('Linus Torvalds');
    await userEvent.click(within(menu).getByRole('menuitem', { name: 'Suspend…' }));
    const dialog = await screen.findByRole('dialog', { name: 'Suspend Linus Torvalds?' });
    await userEvent.type(within(dialog).getByLabelText('Reason, for the activity log'), 'abuse');
    await userEvent.click(
      within(dialog).getByRole('checkbox', { name: /Email Linus that the account is suspended/ }),
    );
    await userEvent.click(within(dialog).getByRole('button', { name: 'Suspend user' }));
    await waitFor(() => expect(actions.suspend).toHaveBeenCalledWith(LINUS, 'abuse', true));
  });

  it('deletes only after typing the email, and shows the API refusal', async () => {
    const actions = makeActions();
    actions.remove.mockResolvedValue({
      ok: false,
      message: 'Linus is the only owner of Acme. Make someone else an owner, or delete Acme first.',
      code: 'conflict',
    } as never);
    show({}, actions);
    const menu = await openMenu('Linus Torvalds');
    await userEvent.click(within(menu).getByRole('menuitem', { name: 'Delete user…' }));
    const dialog = await screen.findByRole('alertdialog');
    const confirm = within(dialog).getByRole('button', { name: 'Delete user' });
    expect(confirm).toBeDisabled();
    await userEvent.type(within(dialog).getByRole('textbox'), 'linus@acme.example');
    await userEvent.click(confirm);
    expect(await within(dialog).findByText(/only owner of Acme/)).toBeInTheDocument();
    expect(actions.remove).toHaveBeenCalledWith(LINUS);
  });

  it('emails one user', async () => {
    const actions = show();
    const menu = await openMenu('Linus Torvalds');
    await userEvent.click(within(menu).getByRole('menuitem', { name: 'Email Linus' }));
    const dialog = await screen.findByRole('dialog', { name: 'Email Linus Torvalds' });
    await userEvent.type(within(dialog).getByLabelText('Subject'), 'Hello');
    await userEvent.type(within(dialog).getByLabelText('Message'), 'Hi **there**');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Send email' }));
    await waitFor(() => expect(actions.email).toHaveBeenCalledWith(LINUS, 'Hello', 'Hi **there**'));
    expect(await screen.findByText('Email queued for linus@acme.example.')).toBeInTheDocument();
  });
});

describe('permissions', () => {
  it('a read-only admin sees no selection and only Open and Email in the menu', async () => {
    show({}, makeActions(), ['emails']);
    expect(screen.queryByRole('checkbox')).toBeNull();
    await userEvent.click(screen.getByRole('button', { name: 'Actions for Linus Torvalds' }));
    const menu = screen.getByRole('menu', { name: 'User actions' });
    expect(
      within(menu)
        .getAllByRole('menuitem')
        .map((i) => i.textContent),
    ).toEqual(['Open', 'Email Linus']);
    expect(screen.getByRole('link', { name: 'Export CSV' })).toBeInTheDocument();
  });

  it('without emails the menu has no Email and the bulk bar link is disabled', async () => {
    show({}, makeActions(), ['users']);
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Ada Lovelace' }));
    const bar = screen.getByRole('toolbar');
    expect(within(bar).queryByRole('link', { name: 'Email them' })).toBeNull();
    expect(within(bar).getByRole('button', { name: 'Email them' })).toBeDisabled();
    await userEvent.click(screen.getByRole('button', { name: 'Actions for Ada Lovelace' }));
    expect(screen.queryByRole('menuitem', { name: 'Email Ada' })).toBeNull();
  });
});

it('keeps fixtures honest', () => {
  expect(rows).toHaveLength(3);
});
