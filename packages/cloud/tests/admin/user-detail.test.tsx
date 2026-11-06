import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { Perm } from '@/lib/admin';
import { ADA, detail, NOW, SESSION_A, SESSION_B, suspended } from './users-fixtures';

vi.mock('next/navigation', () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), refresh: vi.fn() }),
}));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { UserDetailView } = await import('@/components/admin/UserDetailView');

const ALL: Perm[] = ['users', 'teams', 'billing', 'emails', 'settings', 'admins'];
const ok = { ok: true } as const;

function makeActions() {
  return {
    suspend: vi.fn(async () => ok),
    unsuspend: vi.fn(async () => ok),
    signOut: vi.fn(async () => ok),
    remove: vi.fn(async () => ok),
    email: vi.fn(async () => ok),
    revokeSession: vi.fn(async () => ok),
    unlink: vi.fn(async () => ok),
  };
}

function show(d = detail, permissions: Perm[] = ALL, actions = makeActions()) {
  render(<UserDetailView detail={d} permissions={permissions} now={NOW} actions={actions} />);
  return actions;
}

const card = (name: string) => screen.getByRole('region', { name });

describe('user detail, active', () => {
  it('shows the header, the facts and the actions', () => {
    show();
    expect(screen.getByRole('heading', { level: 1, name: 'Ada Lovelace' })).toBeInTheDocument();
    expect(screen.getByText('Active')).toBeInTheDocument();
    expect(
      screen.getByText('ada@acme.example · joined Mar 2, 2026 · last active 6 minutes ago'),
    ).toBeInTheDocument();
    expect(screen.queryByText(/Madrid/)).toBeNull();
    for (const name of ['Email Ada', 'Sign out everywhere', 'Suspend']) {
      expect(screen.getByRole('button', { name })).toBeEnabled();
    }
    expect(screen.queryByRole('button', { name: 'Lift suspension' })).toBeNull();
    expect(screen.getByRole('navigation', { name: 'Breadcrumb' })).toHaveTextContent(
      'Users/Ada Lovelace',
    );
  });

  it('lists teams with role, join date and plan', () => {
    show();
    const teams = card('Teams');
    const acme = within(teams).getByRole('link', { name: /Acme/ });
    expect(acme).toHaveAttribute('href', '/admin/teams/acme');
    expect(acme).toHaveTextContent('Owner · joined Mar 2, 2026');
    expect(acme).toHaveTextContent('Team · Stripe');
    const globex = within(teams).getByRole('link', { name: /Globex Partners/ });
    expect(globex).toHaveTextContent('Member · joined Jul 9, 2026');
    expect(globex).toHaveTextContent('Free');
  });

  it('says a granted plan is granted', () => {
    const d = structuredClone(detail);
    d.teams[0].plan_source = 'override';
    show(d);
    expect(within(card('Teams')).getByRole('link', { name: /Acme/ })).toHaveTextContent(
      'Team · Granted',
    );
  });

  it('offers Unlink on providers but not on email', async () => {
    const actions = show();
    const methods = card('Sign-in methods');
    const emailRow = within(methods).getByText('Email code').closest('li') as HTMLElement;
    expect(emailRow).toHaveTextContent('ada@acme.example');
    expect(within(emailRow).queryByRole('button')).toBeNull();
    const gh = within(methods).getByText('GitHub').closest('li') as HTMLElement;
    expect(gh).toHaveTextContent('ada@acme.example · id 4021733');
    expect(methods).toHaveTextContent(
      'Unlinking makes the next GitHub sign-in create a new link by email again.',
    );
    await userEvent.click(within(gh).getByRole('button', { name: 'Unlink GitHub' }));
    const dialog = await screen.findByRole('alertdialog');
    expect(dialog).toHaveTextContent('Unlink GitHub from Ada Lovelace?');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Unlink' }));
    await waitFor(() => expect(actions.unlink).toHaveBeenCalledWith(ADA, 'github'));
  });

  it('lists devices and revokes one', async () => {
    const actions = show();
    const devices = card('Devices');
    expect(devices).toHaveTextContent('Chrome on macOS');
    expect(devices).toHaveTextContent('Web · last used 6 minutes ago');
    expect(devices).toHaveTextContent('CLI or studio');
    expect(devices).toHaveTextContent('Device · last used yesterday');
    await userEvent.click(within(devices).getByRole('button', { name: 'Revoke CLI or studio' }));
    await waitFor(() => expect(actions.revokeSession).toHaveBeenCalledWith(ADA, SESSION_B));
    expect(SESSION_A).toBeTruthy();
  });

  it('signs out every device from the card after a confirm', async () => {
    const actions = show();
    await userEvent.click(within(card('Devices')).getByRole('button', { name: 'Sign out all' }));
    const dialog = await screen.findByRole('alertdialog');
    expect(dialog).toHaveTextContent('Sign out Ada Lovelace everywhere?');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Sign out everywhere' }));
    await waitFor(() => expect(actions.signOut).toHaveBeenCalledWith(ADA));
  });

  it('says there are no devices', () => {
    show({ ...detail, sessions: [] });
    expect(card('Devices')).toHaveTextContent('No signed-in devices.');
    expect(within(card('Devices')).getByRole('button', { name: 'Sign out all' })).toBeDisabled();
  });

  it('lists what is shared with them', () => {
    show();
    const shared = card('Shared with Ada');
    expect(shared).toHaveTextContent('Billing sandbox');
    expect(shared).toHaveTextContent('Initech · public link');
    expect(shared).toHaveTextContent('Collections from other teams');
  });

  it('says when nothing is shared and when a team is the only link', () => {
    show({ ...detail, shared: [], teams: [] });
    expect(card('Shared with Ada')).toHaveTextContent('Nothing is shared with Ada.');
    expect(card('Teams')).toHaveTextContent('Not on a team.');
  });

  it('lists recent admin events with the ip', () => {
    show();
    const events = card('Recent events');
    expect(events).toHaveTextContent('Nov 5 13:02');
    expect(events).toHaveTextContent(
      'alex@example.com suspended Ada Lovelace: “Chargeback dispute”',
    );
    expect(events).toHaveTextContent('83.44.12.9');
    expect(events).toHaveTextContent(
      'alex@example.com signed out several users: including Ada Lovelace',
    );
    expect(within(events).getByRole('link', { name: 'Admin actions on Ada' })).toHaveAttribute(
      'href',
      '/admin/activity',
    );
  });
});

describe('suspend dialog', () => {
  it('suspends with a reason and notify', async () => {
    const actions = show();
    await userEvent.click(screen.getByRole('button', { name: 'Suspend' }));
    const dialog = await screen.findByRole('dialog', { name: 'Suspend Ada Lovelace?' });
    expect(dialog).toHaveTextContent(
      'Ada can’t sign in, and every device is signed out now. Teams, tokens and collections keep working, so CI isn’t broken.',
    );
    await userEvent.type(
      within(dialog).getByLabelText('Reason, for the activity log'),
      'Chargeback dispute',
    );
    await userEvent.click(
      within(dialog).getByRole('checkbox', { name: 'Email Ada that the account is suspended' }),
    );
    await userEvent.click(within(dialog).getByRole('button', { name: 'Suspend user' }));
    await waitFor(() =>
      expect(actions.suspend).toHaveBeenCalledWith(ADA, 'Chargeback dispute', true),
    );
  });

  it('defaults to no email, and blocks a reason over 500 characters', async () => {
    const actions = show();
    await userEvent.click(screen.getByRole('button', { name: 'Suspend' }));
    const dialog = await screen.findByRole('dialog', { name: 'Suspend Ada Lovelace?' });
    expect(within(dialog).getByRole('checkbox')).not.toBeChecked();
    const reason = within(dialog).getByLabelText('Reason, for the activity log');
    expect(reason).toHaveAttribute('maxlength', '500');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Suspend user' }));
    await waitFor(() => expect(actions.suspend).toHaveBeenCalledWith(ADA, '', false));
  });

  it('shows the API message and stays open', async () => {
    const actions = makeActions();
    actions.suspend.mockResolvedValue({
      ok: false,
      message: 'You can’t do that to your own account.',
    } as never);
    show(detail, ALL, actions);
    await userEvent.click(screen.getByRole('button', { name: 'Suspend' }));
    const dialog = await screen.findByRole('dialog', { name: 'Suspend Ada Lovelace?' });
    await userEvent.click(within(dialog).getByRole('button', { name: 'Suspend user' }));
    expect(
      await within(dialog).findByText('You can’t do that to your own account.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });
});

describe('user detail, suspended', () => {
  it('shows the banner with who, when and why, and lifts it', async () => {
    const actions = show(suspended);
    const banner = screen.getByRole('alert');
    expect(banner).toHaveTextContent(
      'Suspended by alex@example.com on Nov 5: “Chargeback dispute, waiting on bank”',
    );
    expect(banner).toHaveTextContent(
      'Ada can’t sign in, and every session was signed out. Teams and collections are untouched.',
    );
    expect(screen.getAllByText('Suspended').length).toBeGreaterThan(0);
    expect(screen.queryByRole('button', { name: 'Suspend' })).toBeNull();
    await userEvent.click(within(banner).getByRole('button', { name: 'Lift suspension' }));
    const dialog = await screen.findByRole('alertdialog');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Lift suspension' }));
    await waitFor(() => expect(actions.unsuspend).toHaveBeenCalledWith(ADA));
  });

  it('copes with a suspender whose account is gone and no reason', () => {
    const d = structuredClone(suspended);
    d.user.suspension = { at: '2026-11-05T13:02:00Z', reason: null, by: null };
    show(d);
    expect(screen.getByRole('alert')).toHaveTextContent('Suspended on Nov 5.');
  });
});

describe('delete user', () => {
  it('needs the email typed, then deletes', async () => {
    const actions = show();
    const zone = card('Delete user');
    expect(zone).toHaveTextContent(
      'Removes the account, sign-ins, devices and access-list matches. Teams stay with their other owners.',
    );
    await userEvent.click(within(zone).getByRole('button', { name: 'Delete Ada…' }));
    const dialog = await screen.findByRole('alertdialog');
    const confirm = within(dialog).getByRole('button', { name: 'Delete user' });
    expect(confirm).toBeDisabled();
    await userEvent.type(within(dialog).getByRole('textbox'), 'ada@acme.example');
    await userEvent.click(confirm);
    await waitFor(() => expect(actions.remove).toHaveBeenCalledWith(ADA));
  });

  it('shows the sole-owner block in the card, with the API copy', async () => {
    const message =
      'Ada is the only owner of Acme. Make someone else an owner, or delete Acme first.';
    const actions = makeActions();
    actions.remove.mockResolvedValue({ ok: false, message, code: 'conflict' } as never);
    show(detail, ALL, actions);
    const zone = card('Delete user');
    await userEvent.click(within(zone).getByRole('button', { name: 'Delete Ada…' }));
    const dialog = await screen.findByRole('alertdialog');
    await userEvent.type(within(dialog).getByRole('textbox'), 'ada@acme.example');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Delete user' }));
    await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull());
    expect(within(card('Delete user')).getByText(message)).toBeInTheDocument();
  });
});

describe('email dialog', () => {
  it('sends Markdown to the user and confirms', async () => {
    const actions = show();
    await userEvent.click(screen.getByRole('button', { name: 'Email Ada' }));
    const dialog = await screen.findByRole('dialog', { name: 'Email Ada Lovelace' });
    expect(dialog).toHaveTextContent('Markdown works');
    await userEvent.type(within(dialog).getByLabelText('Subject'), 'About your account');
    await userEvent.type(within(dialog).getByLabelText('Message'), 'Hello **Ada**');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Send email' }));
    await waitFor(() =>
      expect(actions.email).toHaveBeenCalledWith(ADA, 'About your account', 'Hello **Ada**'),
    );
    expect(await screen.findByText('Email queued for ada@acme.example.')).toBeInTheDocument();
  });
});

describe('permissions', () => {
  it('a read-only admin cannot change anything', () => {
    show(detail, []);
    for (const name of [
      'Email Ada',
      'Sign out everywhere',
      'Suspend',
      'Delete Ada…',
      'Sign out all',
    ]) {
      expect(screen.getByRole('button', { name })).toBeDisabled();
    }
    expect(screen.getByRole('button', { name: 'Unlink GitHub' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Revoke Chrome on macOS' })).toBeDisabled();
  });

  it('users without emails can act but not email', () => {
    show(detail, ['users']);
    expect(screen.getByRole('button', { name: 'Email Ada' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Suspend' })).toBeEnabled();
  });

  it('emails without users can email only', () => {
    show(detail, ['emails']);
    expect(screen.getByRole('button', { name: 'Email Ada' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Suspend' })).toBeDisabled();
  });

  it('a suspended page hides lift for a read-only admin as disabled', () => {
    show(suspended, []);
    expect(screen.getByRole('button', { name: 'Lift suspension' })).toBeDisabled();
  });
});
