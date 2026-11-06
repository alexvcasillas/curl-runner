import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { Perm } from '@/lib/admin';
import { admins, MARCO, NOW, ROSA } from './admin-fixtures';

const { AdminsView } = await import('@/components/admin/AdminsView');

const ALL: Perm[] = ['users', 'teams', 'billing', 'emails', 'settings', 'admins'];
const ok = { ok: true } as const;

function show(perms: Perm[] = ALL) {
  const actions = {
    add: vi.fn(async () => ok),
    update: vi.fn(async () => ok),
    remove: vi.fn(async () => ok),
  };
  render(
    <AdminsView admins={admins} permissions={perms} now={NOW.toISOString()} actions={actions} />,
  );
  return actions;
}

describe('the list', () => {
  it('shows role, permissions, last active and who added them', () => {
    show();
    const rows = screen.getAllByRole('row').slice(1);
    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveTextContent('Ada');
    expect(rows[0]).toHaveTextContent('Owner');
    expect(rows[0]).toHaveTextContent('Every permission');
    expect(rows[0]).toHaveTextContent('Environment');
    expect(rows[0]).not.toHaveTextContent('Edit permissions');
    expect(rows[1]).toHaveTextContent('Rosa Méndez');
    expect(rows[1]).toHaveTextContent('Users, teams, emails');
    expect(rows[1]).toHaveTextContent('Ada, Nov 2');
    expect(rows[2]).toHaveTextContent('marco@example.com');
    expect(rows[2]).toHaveTextContent('Never');
  });

  it('never says invite or pending', () => {
    show();
    expect(screen.queryByText(/invite/i)).toBeNull();
    expect(screen.queryByText(/not accepted|pending/i)).toBeNull();
    expect(screen.getByRole('button', { name: 'Add admin' })).toBeInTheDocument();
  });
});

describe('add admin', () => {
  it('takes an email and permissions, with no email going out', async () => {
    const a = show();
    await userEvent.click(screen.getByRole('button', { name: 'Add admin' }));
    const dialog = await screen.findByRole('dialog');
    expect(dialog).toHaveTextContent('Add an admin');
    expect(within(dialog).getAllByRole('checkbox')).toHaveLength(6);
    await userEvent.type(within(dialog).getByLabelText('Email'), 'sam@example.com');
    await userEvent.click(within(dialog).getByRole('checkbox', { name: 'Manage users' }));
    await userEvent.click(within(dialog).getByRole('checkbox', { name: 'Send emails' }));
    await userEvent.click(within(dialog).getByRole('button', { name: 'Add admin' }));
    await waitFor(() => expect(a.add).toHaveBeenCalledWith('sam@example.com', ['users', 'emails']));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  });

  it('keeps the dialog open with the API’s copy for an unknown email or an owner', async () => {
    const a = show();
    a.add.mockResolvedValueOnce({
      ok: false,
      message: 'No account with that email yet. Ask them to sign in once.',
    } as never);
    await userEvent.click(screen.getByRole('button', { name: 'Add admin' }));
    const dialog = await screen.findByRole('dialog');
    await userEvent.type(within(dialog).getByLabelText('Email'), 'who@example.com');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Add admin' }));
    expect(await within(dialog).findByText(/No account with that email yet/)).toBeInTheDocument();
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });
});

describe('edit and remove', () => {
  it('opens with the current permissions ticked and saves the change', async () => {
    const a = show();
    const row = screen.getAllByRole('row')[2];
    await userEvent.click(within(row).getByRole('button', { name: /Edit permissions/ }));
    const dialog = await screen.findByRole('dialog');
    expect(dialog).toHaveTextContent('Permissions for Rosa Méndez');
    expect(dialog).toHaveTextContent(
      'Rosa sees the change on her next page load.'.replace('her', 'their'),
    );
    expect(within(dialog).getByRole('checkbox', { name: 'Manage users' })).toBeChecked();
    expect(within(dialog).getByRole('checkbox', { name: 'Manage plans' })).not.toBeChecked();
    await userEvent.click(within(dialog).getByRole('checkbox', { name: 'Manage plans' }));
    await userEvent.click(within(dialog).getByRole('button', { name: 'Save permissions' }));
    await waitFor(() =>
      expect(a.update).toHaveBeenCalledWith(ROSA, ['users', 'teams', 'billing', 'emails']),
    );
  });

  it('removes after a confirmation that says the account stays', async () => {
    const a = show();
    const row = screen.getAllByRole('row')[3];
    await userEvent.click(within(row).getByRole('button', { name: /Edit permissions/ }));
    await userEvent.click(await screen.findByRole('button', { name: 'Remove admin' }));
    const confirm = await screen.findByRole('alertdialog');
    expect(confirm).toHaveTextContent('keeps their account and their teams');
    await userEvent.click(within(confirm).getByRole('button', { name: 'Remove admin' }));
    await waitFor(() => expect(a.remove).toHaveBeenCalledWith(MARCO));
  });
});

describe('without the admins permission', () => {
  it('reads the list and cannot change it', () => {
    show(['users']);
    expect(screen.getByRole('button', { name: 'Add admin' })).toBeDisabled();
    expect(screen.queryByRole('button', { name: /Edit permissions/ })).toBeNull();
  });
});
