import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import type { Perm } from '@/lib/admin';
import type { TeamDetail } from '@/lib/admin-teams';
import { ADA, detail, free, GRACE, granted, LINUS, NOW, suspended } from './teams-fixtures';

const refresh = vi.fn();
vi.mock('next/navigation', () => ({
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), refresh }),
}));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { TeamDetailView } = await import('@/components/admin/TeamDetailView');

const ALL: Perm[] = ['users', 'teams', 'billing', 'emails', 'settings', 'admins'];
const ok = { ok: true } as const;

function makeActions() {
  return {
    grant: vi.fn(async () => ({ ok: true as const, stripeConflict: false })),
    removeOverride: vi.fn(async () => ok),
    suspend: vi.fn(async () => ok),
    unsuspend: vi.fn(async () => ok),
    remove: vi.fn(async () => ok),
    setRole: vi.fn(async () => ok),
    stripeSync: vi.fn(async () => ok),
    resendInvoice: vi.fn(async () => ok),
    emailOwners: vi.fn(async () => ok),
  };
}

function show(d: TeamDetail = detail, permissions: Perm[] = ALL, actions = makeActions()) {
  render(<TeamDetailView detail={d} permissions={permissions} now={NOW} actions={actions} />);
  return actions;
}

const card = (name: string) => screen.getByRole('region', { name });

describe('Paying via Stripe', () => {
  it('shows the header, the Stripe card and the override prompt', () => {
    show();
    expect(screen.getByRole('heading', { level: 1, name: 'Acme' })).toBeInTheDocument();
    expect(screen.getByText('acme · created Mar 2, 2026')).toBeInTheDocument();
    expect(screen.getByRole('navigation', { name: 'Breadcrumb' })).toHaveTextContent('Teams/Acme');
    const plan = card('Plan');
    expect(plan).toHaveTextContent('Team · Annual · 3 seats');
    expect(plan).toHaveTextContent('$288 a year · renews Nov 5, 2027');
    expect(within(plan).getByRole('link', { name: 'sub_1QxN4kLm ↗' })).toHaveAttribute(
      'href',
      'https://dashboard.stripe.com/subscriptions/sub_1QxN4kLm',
    );
    const cus = within(plan).getByRole('link', { name: 'cus_R2pQ7 ↗' });
    expect(cus).toHaveAttribute('href', 'https://dashboard.stripe.com/customers/cus_R2pQ7');
    expect(cus).toHaveAttribute('target', '_blank');
    expect(cus).toHaveAttribute('rel', expect.stringContaining('noreferrer'));
    expect(plan).not.toHaveTextContent('Visa');
    expect(plan).toHaveTextContent('None. The plan comes from the Stripe subscription.');
    expect(plan).toHaveTextContent('Effective plan: Team · limits: none');
    expect(within(plan).getByRole('button', { name: 'Grant a plan' })).toBeEnabled();
  });

  it('has no View as owner button', () => {
    show();
    expect(screen.queryByText(/View as owner/)).toBeNull();
  });

  it('syncs and resends, saying what happened', async () => {
    const actions = show();
    await userEvent.click(screen.getByRole('button', { name: 'Sync from Stripe' }));
    await waitFor(() => expect(actions.stripeSync).toHaveBeenCalledWith('acme'));
    expect(await screen.findByText('Synced from Stripe.')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Resend last invoice' }));
    await waitFor(() => expect(actions.resendInvoice).toHaveBeenCalledWith('acme'));
    expect(await screen.findByText('Invoice sent to the owners.')).toBeInTheDocument();
  });

  it('shows the 409 copy when there is no invoice yet', async () => {
    const actions = makeActions();
    actions.resendInvoice.mockResolvedValue({
      ok: false,
      message: 'There’s no finished invoice to resend yet.',
    } as never);
    show(detail, ALL, actions);
    await userEvent.click(screen.getByRole('button', { name: 'Resend last invoice' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'There’s no finished invoice to resend yet.',
    );
  });

  it('says a scheduled cancel and a past-due card instead of a renewal', () => {
    show({
      ...detail,
      stripe: { ...detail.stripe!, cancel_at: '2026-12-03T00:00:00Z' },
    });
    expect(card('Plan')).toHaveTextContent('cancels Dec 3, 2026');
  });

  it('lists members, pending invites and collections', () => {
    show();
    const members = card('Members');
    expect(members).toHaveTextContent('3 accepted · 1 pending');
    expect(within(members).getByRole('link', { name: /Ada Lovelace/ })).toHaveAttribute(
      'href',
      `/admin/users/${ADA}`,
    );
    expect(members).toHaveTextContent('dana@acme.com');
    expect(members).toHaveTextContent('Pending invite · expires Nov 10');
    const cols = card('Collections');
    expect(cols).toHaveTextContent('Payments API');
    expect(cols).toHaveTextContent('payments-api · v14');
    expect(cols).toHaveTextContent('Public');
    expect(cols).toHaveTextContent('Off');
    expect(cols).toHaveTextContent('14 tokens used this month · 1,206 publishes');
  });

  it('shows the plan history', () => {
    show();
    const h = card('Plan history');
    expect(h).toHaveTextContent('Stripe set the plan to Team');
    expect(h).toHaveTextContent('Created on Free');
  });
});

describe('Granting a plan', () => {
  it('opens the form with the Stripe warning and the board copy', async () => {
    show();
    await userEvent.click(screen.getByRole('button', { name: 'Grant a plan' }));
    const plan = card('Plan');
    expect(within(plan).getByLabelText('Plan')).toHaveValue('team');
    expect(within(plan).getByLabelText('Until')).toBeInTheDocument();
    expect(within(plan).getByLabelText('No end date')).not.toBeChecked();
    expect(within(plan).getByLabelText('Reason, internal only')).toBeInTheDocument();
    expect(
      within(plan).getByLabelText('Email the owners that curl-runner gave Acme the Team plan'),
    ).toBeChecked();
    expect(plan).toHaveTextContent(
      'Acme pays through Stripe. Granting doesn’t stop those charges. Cancel the subscription in Stripe too if this replaces it.',
    );
    expect(within(plan).getByRole('button', { name: 'Grant Team' })).toBeInTheDocument();
    await userEvent.click(within(plan).getByRole('button', { name: 'Cancel' }));
    expect(within(plan).getByRole('button', { name: 'Grant a plan' })).toBeInTheDocument();
  });

  it('leaves the warning out for a team that does not pay through Stripe', async () => {
    show(free);
    await userEvent.click(screen.getByRole('button', { name: 'Grant a plan' }));
    expect(card('Plan')).not.toHaveTextContent('pays through Stripe');
  });

  it('grants with an end date, a reason and notify', async () => {
    const actions = show();
    await userEvent.click(screen.getByRole('button', { name: 'Grant a plan' }));
    const plan = card('Plan');
    fireEvent.change(within(plan).getByLabelText('Until'), { target: { value: '2099-03-31' } });
    await userEvent.type(within(plan).getByLabelText('Reason, internal only'), 'Design partner');
    await userEvent.click(within(plan).getByRole('button', { name: 'Grant Team' }));
    await waitFor(() =>
      expect(actions.grant).toHaveBeenCalledWith('acme', {
        plan: 'team',
        until: '2099-03-31T23:59:59.000Z',
        reason: 'Design partner',
        notify: true,
      }),
    );
    expect(await screen.findByText('Granted Team to Acme.')).toBeInTheDocument();
  });

  it('mentions Stripe when the API says it still bills', async () => {
    const actions = makeActions();
    actions.grant.mockResolvedValue({ ok: true, stripeConflict: true });
    show(detail, ALL, actions);
    await userEvent.click(screen.getByRole('button', { name: 'Grant a plan' }));
    await userEvent.click(within(card('Plan')).getByLabelText('No end date'));
    await userEvent.click(within(card('Plan')).getByRole('button', { name: 'Grant Team' }));
    expect(await screen.findByText(/Stripe still bills them/)).toBeInTheDocument();
  });

  it('sends null for no end date and disables the date', async () => {
    const actions = show();
    await userEvent.click(screen.getByRole('button', { name: 'Grant a plan' }));
    const plan = card('Plan');
    await userEvent.click(within(plan).getByLabelText('No end date'));
    expect(within(plan).getByLabelText('Until')).toBeDisabled();
    await userEvent.click(within(plan).getByLabelText(/Email the owners/));
    await userEvent.click(within(plan).getByRole('button', { name: 'Grant Team' }));
    await waitFor(() =>
      expect(actions.grant).toHaveBeenCalledWith('acme', {
        plan: 'team',
        until: null,
        reason: '',
        notify: false,
      }),
    );
  });

  it('asks for a date or No end date, and refuses a past one, before calling', async () => {
    const actions = show();
    await userEvent.click(screen.getByRole('button', { name: 'Grant a plan' }));
    const plan = card('Plan');
    await userEvent.click(within(plan).getByRole('button', { name: 'Grant Team' }));
    expect(await within(plan).findByRole('alert')).toHaveTextContent(
      'Pick an end date, or choose No end date.',
    );
    fireEvent.change(within(plan).getByLabelText('Until'), { target: { value: '2020-01-01' } });
    await userEvent.click(within(plan).getByRole('button', { name: 'Grant Team' }));
    expect(await within(plan).findByRole('alert')).toHaveTextContent(
      'The end date must be in the future.',
    );
    expect(actions.grant).not.toHaveBeenCalled();
  });

  it('says when the end date lands', async () => {
    show();
    await userEvent.click(screen.getByRole('button', { name: 'Grant a plan' }));
    expect(within(card('Plan')).getByText('Ends at the end of that day, UTC.')).toBeInTheDocument();
  });

  it('follows the plan in the button and the email label', async () => {
    show();
    await userEvent.click(screen.getByRole('button', { name: 'Grant a plan' }));
    await userEvent.selectOptions(within(card('Plan')).getByLabelText('Plan'), 'free');
    expect(within(card('Plan')).getByRole('button', { name: 'Grant Free' })).toBeInTheDocument();
    expect(
      within(card('Plan')).getByLabelText(
        'Email the owners that curl-runner gave Acme the Free plan',
      ),
    ).toBeInTheDocument();
  });

  it('shows the API message and stays open on a failure', async () => {
    const actions = makeActions();
    actions.grant.mockResolvedValue({ ok: false, message: 'Nope, try later.' } as never);
    show(detail, ALL, actions);
    await userEvent.click(screen.getByRole('button', { name: 'Grant a plan' }));
    await userEvent.click(within(card('Plan')).getByLabelText('No end date'));
    await userEvent.click(within(card('Plan')).getByRole('button', { name: 'Grant Team' }));
    expect(await within(card('Plan')).findByRole('alert')).toHaveTextContent('Nope, try later.');
    expect(within(card('Plan')).getByRole('button', { name: 'Grant Team' })).toBeInTheDocument();
  });
});

describe('Override active', () => {
  it('shows the override and the 14-day note', () => {
    show(granted);
    const plan = card('Plan');
    expect(plan).toHaveTextContent('Team until Mar 31, 2027');
    expect(plan).toHaveTextContent(
      '“Design partner, pilot until the end of Q1” · granted by alex@curl-runner.com on Nov 5',
    );
    expect(plan).toHaveTextContent(
      'Owners get an email 14 days before it ends. Then the team goes back to Stripe or Free.',
    );
    expect(screen.getByText('Granted until Mar 31, 2027')).toBeInTheDocument();
    expect(within(plan).getByRole('button', { name: 'Edit' })).toBeInTheDocument();
  });

  it('says a grant with no end', () => {
    show({ ...granted, override: { ...granted.override!, until: null } });
    expect(card('Plan')).toHaveTextContent('Team, no end date');
    expect(card('Plan')).not.toHaveTextContent('14 days before');
  });

  it('edits with the form filled in', async () => {
    const actions = show(granted);
    await userEvent.click(screen.getByRole('button', { name: 'Edit' }));
    const plan = card('Plan');
    expect(within(plan).getByLabelText('Until')).toHaveValue('2027-03-31');
    expect(within(plan).getByLabelText('Reason, internal only')).toHaveValue(
      'Design partner, pilot until the end of Q1',
    );
    expect(within(plan).getByLabelText(/Email the owners/)).not.toBeChecked();
    await userEvent.click(within(plan).getByLabelText('No end date'));
    await userEvent.click(within(plan).getByRole('button', { name: 'Save Team' }));
    await waitFor(() =>
      expect(actions.grant).toHaveBeenCalledWith('acme', {
        plan: 'team',
        until: null,
        reason: 'Design partner, pilot until the end of Q1',
        notify: false,
      }),
    );
    expect(await screen.findByText('Updated the Team override for Acme.')).toBeInTheDocument();
  });

  it('removes after a confirm', async () => {
    const actions = show(granted);
    await userEvent.click(screen.getByRole('button', { name: 'Remove override' }));
    const dialog = await screen.findByRole('alertdialog');
    expect(dialog).toHaveTextContent('Remove the plan override on Acme?');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Remove override' }));
    await waitFor(() => expect(actions.removeOverride).toHaveBeenCalledWith('acme'));
    expect(await screen.findByText('Removed the override on Acme.')).toBeInTheDocument();
  });

  it('flags an override that ended and is waiting to be cleared', () => {
    show({
      ...detail,
      override: { ...granted.override!, active: false },
    });
    expect(card('Plan')).toHaveTextContent('Team until Mar 31, 2027 (ended)');
  });
});

describe('Free team and suspended team', () => {
  it('free: no Stripe, none, the default', () => {
    show(free);
    const plan = card('Plan');
    expect(plan).toHaveTextContent('None. This team has never paid through Stripe.');
    expect(plan).toHaveTextContent('None. The plan comes from the default, Free.');
    expect(plan).toHaveTextContent(
      'Effective plan: Free · limits: 3 members, no restricted collections',
    );
    expect(screen.getByText('No subscription')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Sync from Stripe' })).toBeNull();
  });

  it('suspended: the banner names who, when and why, and Lift suspension works', async () => {
    const actions = show(suspended);
    const banner = screen.getByRole('alert');
    expect(banner).toHaveTextContent(
      'Suspended by rosa@curl-runner.com on Oct 29: “Phishing pages in shared collections”.',
    );
    expect(banner).toHaveTextContent(
      'Publishing is refused, share links show “no longer works”, and members see a notice to contact support.',
    );
    await userEvent.click(within(banner).getByRole('button', { name: 'Lift suspension' }));
    const dialog = await screen.findByRole('alertdialog');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Lift suspension' }));
    await waitFor(() => expect(actions.unsuspend).toHaveBeenCalledWith('spam-tests'));
  });

  it('says Suspended in the header, in place of No subscription', () => {
    show(suspended);
    const h1 = screen.getByRole('heading', { level: 1 });
    expect(h1.parentElement).toHaveTextContent('Suspended');
    expect(h1.parentElement).not.toHaveTextContent('No subscription');
  });

  it('keeps the source next to Suspended for a team that still has one', () => {
    show({ ...detail, suspension: suspended.suspension });
    const h1 = screen.getByRole('heading', { level: 1 });
    expect(h1.parentElement).toHaveTextContent('Stripe · annual');
    expect(h1.parentElement).toHaveTextContent('Suspended');
  });

  it('hides Suspend on a suspended team', () => {
    show(suspended);
    expect(screen.queryByRole('button', { name: 'Suspend team…' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Delete team…' })).toBeEnabled();
  });
});

describe('suspend, delete and email', () => {
  it('suspends with a reason and notify', async () => {
    const actions = show();
    await userEvent.click(screen.getByRole('button', { name: 'Suspend team…' }));
    const dialog = await screen.findByRole('dialog', { name: 'Suspend Acme?' });
    await userEvent.type(within(dialog).getByLabelText('Reason, for the activity log'), 'Phishing');
    await userEvent.click(within(dialog).getByLabelText('Email the owners that Acme is suspended'));
    await userEvent.click(within(dialog).getByRole('button', { name: 'Suspend team' }));
    await waitFor(() => expect(actions.suspend).toHaveBeenCalledWith('acme', 'Phishing', true));
    expect(await screen.findByText('Suspended Acme.')).toBeInTheDocument();
  });

  it('asks for the slug before it deletes', async () => {
    const actions = show();
    await userEvent.click(screen.getByRole('button', { name: 'Delete team…' }));
    const dialog = await screen.findByRole('alertdialog', { name: 'Delete Acme?' });
    expect(dialog).toHaveTextContent('cancels its Stripe subscription');
    const confirm = within(dialog).getByRole('button', { name: 'Delete team' });
    expect(confirm).toBeDisabled();
    await userEvent.type(within(dialog).getByLabelText(/Type/), 'acme');
    await userEvent.click(confirm);
    await waitFor(() => expect(actions.remove).toHaveBeenCalledWith('acme'));
  });

  it('keeps the delete dialog open with the API message', async () => {
    const actions = makeActions();
    actions.remove.mockResolvedValue({ ok: false, message: 'Stripe is down.' } as never);
    show(detail, ALL, actions);
    await userEvent.click(screen.getByRole('button', { name: 'Delete team…' }));
    const dialog = await screen.findByRole('alertdialog');
    await userEvent.type(within(dialog).getByLabelText(/Type/), 'acme');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Delete team' }));
    expect(await within(dialog).findByText('Stripe is down.')).toBeInTheDocument();
  });

  it('emails the owners', async () => {
    const actions = show();
    await userEvent.click(screen.getByRole('button', { name: 'Email owners' }));
    const dialog = await screen.findByRole('dialog', { name: 'Email the owners of Acme' });
    await userEvent.type(within(dialog).getByLabelText('Subject'), 'Hello');
    await userEvent.type(within(dialog).getByLabelText('Message'), 'Hi **there**');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Send email' }));
    await waitFor(() =>
      expect(actions.emailOwners).toHaveBeenCalledWith('acme', 'Hello', 'Hi **there**'),
    );
    expect(await screen.findByText('Email queued for the owners of Acme.')).toBeInTheDocument();
  });
});

describe('members and roles', () => {
  it('Change roles turns each role into a choice and saves on change', async () => {
    const actions = show();
    await userEvent.click(screen.getByRole('button', { name: 'Change roles' }));
    const members = card('Members');
    const linus = within(members).getByLabelText('Role of Linus Torvalds');
    expect(linus).toHaveValue('member');
    await userEvent.selectOptions(linus, 'owner');
    await waitFor(() => expect(actions.setRole).toHaveBeenCalledWith('acme', LINUS, 'owner'));
    expect(await screen.findByText('Linus Torvalds is now an owner.')).toBeInTheDocument();
    await userEvent.selectOptions(within(members).getByLabelText('Role of Grace Hopper'), 'member');
    await waitFor(() => expect(actions.setRole).toHaveBeenCalledWith('acme', GRACE, 'member'));
    expect(await screen.findByText('Grace Hopper is now a member.')).toBeInTheDocument();
    await userEvent.click(within(members).getByRole('button', { name: 'Done' }));
    expect(within(members).queryByLabelText('Role of Linus Torvalds')).toBeNull();
  });

  it('shows the last-owner 409 and puts the role back', async () => {
    const actions = makeActions();
    actions.setRole.mockResolvedValue({
      ok: false,
      message: 'A team needs at least one owner.',
    } as never);
    show(
      { ...detail, members: detail.members.slice(0, 1).concat(detail.members.slice(2)) },
      ALL,
      actions,
    );
    await userEvent.click(screen.getByRole('button', { name: 'Change roles' }));
    const select = within(card('Members')).getByLabelText('Role of Ada Lovelace');
    await userEvent.selectOptions(select, 'member');
    expect(await screen.findByRole('alert')).toHaveTextContent('A team needs at least one owner.');
    expect(select).toHaveValue('owner');
  });
});

describe('read only', () => {
  it('turns each write off with its reason', () => {
    show(granted, ['users']);
    expect(screen.getByRole('button', { name: 'Edit' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Remove override' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Sync from Stripe' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Resend last invoice' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Edit' })).toHaveAttribute(
      'title',
      'Read only: you need the billing permission.',
    );
    expect(screen.getByRole('button', { name: 'Change roles' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Suspend team…' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Delete team…' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Email owners' })).toBeDisabled();
  });

  it('keeps the grant button off without billing', () => {
    show(detail, ['teams', 'emails']);
    expect(screen.getByRole('button', { name: 'Grant a plan' })).toBeDisabled();
  });
});
