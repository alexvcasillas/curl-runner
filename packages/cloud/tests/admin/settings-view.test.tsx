import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Perm } from '@/lib/admin';
import type { SettingsData } from '@/lib/admin-settings';
import { settings } from './admin-fixtures';

vi.mock('next/navigation', () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), refresh: vi.fn() }),
}));

const { SettingsView } = await import('@/components/admin/SettingsView');
const { ToastProvider } = await import('@/components/Toast');

const ALL: Perm[] = ['users', 'teams', 'billing', 'emails', 'settings', 'admins'];
const ok = { ok: true } as const;

function makeActions() {
  return {
    signup: vi.fn(async () => ok),
    plans: vi.fn(async () => ok),
    banner: vi.fn(async () => ok),
    publishing: vi.fn(async () => ok),
    testEmail: vi.fn(async () => ({ ok: true as const, to: 'ada@example.com' })),
  };
}

function show(over: Partial<SettingsData> = {}, perms: Perm[] = ALL, actions = makeActions()) {
  render(
    <ToastProvider>
      <SettingsView
        settings={{ ...settings, ...over }}
        env={{ signIn: ['email', 'github'], billing: false, owners: 1 }}
        permissions={perms}
        actions={actions}
      />
    </ToastProvider>,
  );
  return actions;
}

const section = (name: string) => screen.getByRole('region', { name });

describe('sign-up', () => {
  it('shows the saved mode, and the domains only for Allowed domains', async () => {
    show();
    expect(screen.getByRole('radio', { name: /Closed/ })).toBeChecked();
    expect(screen.queryByLabelText('Add a domain')).toBeNull();
    await userEvent.click(screen.getByRole('radio', { name: /Allowed domains/ }));
    expect(screen.getByLabelText('Add a domain')).toBeInTheDocument();
  });

  it('adds and removes chips, then saves only this section', async () => {
    const a = show({ signup_mode: 'domains', signup_domains: ['acme.com'] });
    expect(screen.getByRole('button', { name: 'Remove acme.com' })).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText('Add a domain'), ' Globex.com{Enter}');
    expect(screen.getByRole('button', { name: 'Remove globex.com' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Remove acme.com' }));
    await userEvent.click(
      within(section('Who can create teams')).getByRole('button', { name: 'Save' }),
    );
    await waitFor(() => expect(a.signup).toHaveBeenCalledWith('domains', ['globex.com']));
    expect(a.plans).not.toHaveBeenCalled();
    expect(a.banner).not.toHaveBeenCalled();
    expect(await screen.findByText(/Sign-up settings saved/)).toBeInTheDocument();
  });

  it('shows the API’s message when it refuses', async () => {
    const a = makeActions();
    a.signup.mockResolvedValueOnce({ ok: false, message: 'x is not a valid domain.' } as never);
    show({ signup_mode: 'domains' }, ALL, a);
    await userEvent.click(
      within(section('Who can create teams')).getByRole('button', { name: 'Save' }),
    );
    expect(await screen.findByText('x is not a valid domain.')).toBeInTheDocument();
  });
});

describe('plans', () => {
  it('says lowering a limit never removes anyone, and saves the numbers', async () => {
    const a = show();
    const card = section('Plans');
    expect(
      within(card).getByText(/Lowering one never removes members or collections\./),
    ).toBeInTheDocument();
    const limit = within(card).getByLabelText('Free plan: members');
    await userEvent.clear(limit);
    await userEvent.type(limit, '2');
    await userEvent.selectOptions(within(card).getByLabelText('Plan for new teams'), 'team');
    await userEvent.click(within(card).getByRole('button', { name: 'Save' }));
    await waitFor(() =>
      expect(a.plans).toHaveBeenCalledWith({
        defaultPlan: 'team',
        memberLimit: 2,
        restricted: false,
        graceDays: 14,
      }),
    );
  });
});

describe('banner', () => {
  it('saves message, tone, audience and link; the switch off clears the message', async () => {
    const a = show();
    const card = section('Announcement banner');
    await userEvent.click(within(card).getByRole('switch', { name: 'Show the banner' }));
    await userEvent.type(within(card).getByLabelText('Message, one sentence'), 'Hello there');
    await userEvent.click(within(card).getByRole('radio', { name: 'Warning' }));
    await userEvent.click(within(card).getByRole('radio', { name: 'Everyone' }));
    await userEvent.type(within(card).getByLabelText('Link (optional)'), 'https://a.com');
    await userEvent.click(within(card).getByRole('button', { name: 'Save' }));
    await waitFor(() =>
      expect(a.banner).toHaveBeenCalledWith({
        message: 'Hello there',
        tone: 'warning',
        audience: 'everyone',
        link: 'https://a.com',
      }),
    );
    await userEvent.click(within(card).getByRole('switch', { name: 'Show the banner' }));
    await userEvent.click(within(card).getByRole('button', { name: 'Save' }));
    await waitFor(() =>
      expect(a.banner).toHaveBeenLastCalledWith(expect.objectContaining({ message: '' })),
    );
  });
});

describe('pause publishing', () => {
  it('asks first, with the server’s copy, then pauses', async () => {
    const a = show();
    await userEvent.click(screen.getByRole('switch', { name: 'Pause publishing' }));
    const dialog = await screen.findByRole('alertdialog');
    expect(dialog).toHaveTextContent(
      'Publishing is paused for maintenance. Try again in a few minutes.',
    );
    expect(a.publishing).not.toHaveBeenCalled();
    await userEvent.click(within(dialog).getByRole('button', { name: 'Pause publishing' }));
    await waitFor(() => expect(a.publishing).toHaveBeenCalledWith(true));
  });

  it('resumes without a dialog', async () => {
    const a = show({ publishing_paused: true });
    await userEvent.click(screen.getByRole('switch', { name: 'Pause publishing' }));
    await waitFor(() => expect(a.publishing).toHaveBeenCalledWith(false));
    expect(screen.queryByRole('alertdialog')).toBeNull();
  });
});

describe('environment', () => {
  it('is read-only, shows allowed values, and never a secret', () => {
    show();
    const env = section('From the environment');
    expect(within(env).getByText('https://cloud.curl-runner.com')).toBeInTheDocument();
    expect(within(env).getByText('Set · GitHub sign-in on')).toBeInTheDocument();
    expect(within(env).getAllByText('Not set').length).toBeGreaterThan(0);
    expect(within(env).getAllByText('Hidden').length).toBeGreaterThan(0);
    expect(within(env).getByText('1 owner admin who can’t be removed here')).toBeInTheDocument();
    expect(within(env).queryAllByRole('textbox')).toHaveLength(0);
  });

  it('sends a test email and says where', async () => {
    const a = show();
    await userEvent.click(screen.getByRole('button', { name: 'Send test email to me' }));
    await waitFor(() => expect(a.testEmail).toHaveBeenCalled());
    expect(await screen.findByText(/Test email sent to ada@example.com/)).toBeInTheDocument();
  });
});

describe('without the settings permission', () => {
  it('reads everything and changes nothing', () => {
    show({}, ['users']);
    expect(screen.getAllByText(/Read only/).length).toBeGreaterThan(0);
    for (const b of screen.getAllByRole('button', { name: 'Save' })) {
      expect(b).toBeDisabled();
    }
    expect(screen.getByRole('switch', { name: 'Pause publishing' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Send test email to me' })).toBeDisabled();
  });
});

beforeEach(() => undefined);
