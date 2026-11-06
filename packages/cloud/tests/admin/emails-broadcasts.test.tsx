import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Perm } from '@/lib/admin';
import { type Broadcast, scheduledLabel } from '@/lib/admin-emails';
import { draft, ID, ID2, NOW, preview, scheduled, sent } from './emails-fixtures';

const push = vi.fn();
const refresh = vi.fn();
vi.mock('next/navigation', () => ({ useRouter: () => ({ push, replace: vi.fn(), refresh }) }));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { BroadcastsView } = await import('@/components/admin/BroadcastsView');

function makeActions() {
  return {
    create: vi.fn(async () => ({ ok: true as const, broadcast: { ...draft, id: ID2 } })),
    save: vi.fn(
      async (_id: string, input: { subject: string; body_md: string; audience: string }) => ({
        ok: true as const,
        broadcast: { ...draft, ...input, subject: input.subject.trim() } as Broadcast,
      }),
    ),
    test: vi.fn(async () => ({ ok: true as const, to: 'ada@example.com' })),
    send: vi.fn(async (_id: string, at?: string | null) => ({
      ok: true as const,
      broadcast: (at
        ? { ...draft, status: 'scheduled', scheduled_at: at }
        : { ...draft, status: 'sending', sent_count: 0 }) as Broadcast,
    })),
    preview: vi.fn(async () => ({ ok: true as const, preview })),
    count: vi.fn(async () => ({ ok: true as const, count: 214 })),
  };
}

function show(
  over: Partial<React.ComponentProps<typeof BroadcastsView>> = {},
  permissions: Perm[] = ['emails'],
  actions = makeActions(),
) {
  render(
    <BroadcastsView
      items={[draft, sent, scheduled]}
      selected={draft}
      initialCount={286}
      now={NOW}
      permissions={permissions}
      actions={actions}
      {...over}
    />,
  );
  return actions;
}

/** Lets the debounced autosave and preview run. */
const settle = (ms = 2000) => act(() => vi.advanceTimersByTimeAsync(ms));

beforeEach(() => {
  push.mockReset();
  refresh.mockReset();
  vi.useFakeTimers({ shouldAdvanceTime: true });
});
afterEach(() => {
  vi.useRealTimers();
});

describe('the list', () => {
  it('shows every broadcast with its state and marks the open one', async () => {
    show();
    await settle(500);
    const list = screen.getByRole('region', { name: 'Broadcasts' });
    expect(
      within(list).getByRole('link', { name: /The curl-runner Cloud dashboard is live/ }),
    ).toHaveAttribute('aria-current', 'page');
    expect(list).toHaveTextContent('Draft · Team owners');
    expect(list).toHaveTextContent('Sent Nov 5 · 1,148 delivered · 3 failed');
    expect(list).toHaveTextContent('Scheduled Nov 9, 10:00 UTC · Team owners');
    expect(
      within(list).getByRole('link', { name: /Welcome to the private preview/ }),
    ).toHaveAttribute('href', `/admin/emails?b=${ID2}`);
  });

  it('starts a new draft and opens it', async () => {
    const actions = show();
    await settle(500);
    await userEvent.click(screen.getByRole('button', { name: 'New' }));
    await waitFor(() => expect(actions.create).toHaveBeenCalled());
    expect(push).toHaveBeenCalledWith(`/admin/emails?b=${ID2}`);
  });

  it('invites the first broadcast when there are none', () => {
    show({ items: [], selected: null, initialCount: null });
    expect(screen.getByText('No broadcasts yet')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Write a broadcast' })).toBeEnabled();
  });
});

describe('the editor', () => {
  it('offers exactly the five audiences the server has', async () => {
    show();
    await settle(500);
    const group = screen.getByRole('radiogroup', { name: 'Audience' });
    expect(
      within(group)
        .getAllByRole('radio')
        .map((r) => r.textContent),
    ).toEqual(['Everyone', 'Team owners', 'Paying teams', 'Free teams', 'Granted teams']);
    expect(within(group).getByRole('radio', { name: 'Team owners' })).toBeChecked();
    for (const gone of ['One team', 'A list', 'Retry']) {
      expect(screen.queryByText(gone)).toBeNull();
    }
    expect(screen.getByText('286 people: owners of every team.')).toBeInTheDocument();
  });

  it('autosaves after a pause and says so', async () => {
    const actions = show();
    await settle(500);
    await userEvent.type(screen.getByLabelText('Subject'), '!');
    expect(actions.save).not.toHaveBeenCalled();
    await settle();
    expect(actions.save).toHaveBeenCalledTimes(1);
    expect(actions.save).toHaveBeenCalledWith(ID, {
      subject: 'The curl-runner Cloud dashboard is live!',
      body_md: draft.body_md,
      audience: 'team_owners',
    });
    expect(screen.getByText(/Draft saved/)).toBeInTheDocument();
  });

  it('saves an audience change and re-counts it', async () => {
    const actions = show();
    await settle(500);
    actions.count.mockResolvedValue({ ok: true, count: 47 } as never);
    await userEvent.click(screen.getByRole('radio', { name: 'Paying teams' }));
    await settle();
    expect(actions.save).toHaveBeenCalledWith(
      ID,
      expect.objectContaining({ audience: 'paying_teams' }),
    );
    expect(actions.count).toHaveBeenCalledWith('paying_teams');
    expect(screen.getByRole('button', { name: 'Send to 47 people' })).toBeInTheDocument();
  });

  it('shows the save failure and keeps the draft', async () => {
    const actions = show();
    actions.save.mockResolvedValue({
      ok: false,
      message: 'The subject can be at most 200 characters.',
    } as never);
    await settle(500);
    await userEvent.type(screen.getByLabelText('Subject'), 'x');
    await settle();
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'The subject can be at most 200 characters.',
    );
    expect(screen.getByLabelText('Subject')).toHaveValue(
      'The curl-runner Cloud dashboard is livex',
    );
  });
});

describe('the preview', () => {
  it('shows the server’s HTML in a frame with no scripts', async () => {
    const actions = show();
    await settle(500);
    expect(actions.preview).toHaveBeenCalledWith(ID, '');
    const frame = await screen.findByTitle('Email preview');
    expect(frame.tagName).toBe('IFRAME');
    expect(frame).toHaveAttribute('sandbox', '');
    expect(frame).toHaveAttribute('srcdoc', preview.html);
    expect(screen.getByText('As Ada Lovelace')).toBeInTheDocument();
    expect(screen.getByText(`Subject: ${preview.subject}`)).toBeInTheDocument();
  });

  it('previews as another user after a pause', async () => {
    const actions = show();
    await settle(500);
    actions.preview.mockClear();
    await userEvent.type(screen.getByLabelText('Preview as'), 'linus@acme.com');
    expect(actions.preview).not.toHaveBeenCalled();
    await settle(800);
    expect(actions.preview).toHaveBeenCalledTimes(1);
    expect(actions.preview).toHaveBeenCalledWith(ID, 'linus@acme.com');
  });

  it('refreshes only once the edit is saved', async () => {
    const actions = show();
    await settle(500);
    actions.preview.mockClear();
    await userEvent.type(screen.getByLabelText('Message, in Markdown'), ' more');
    await settle(300);
    expect(actions.save).not.toHaveBeenCalled();
    expect(actions.preview).not.toHaveBeenCalled();
    // React runs the effect after a save when act() ends, so step in two.
    await settle(1000);
    await settle(1000);
    expect(actions.save).toHaveBeenCalledTimes(1);
    expect(actions.preview).toHaveBeenCalledTimes(1);
    expect(actions.save.mock.invocationCallOrder[0]).toBeLessThan(
      actions.preview.mock.invocationCallOrder[0],
    );
  });

  it('says who would be skipped', async () => {
    const actions = makeActions();
    actions.preview.mockResolvedValue({
      ok: true,
      preview: { ...preview, in_audience: false, opted_out: true },
    });
    show({}, ['emails'], actions);
    await settle(500);
    expect(await screen.findByText(/isn’t in this audience/)).toBeInTheDocument();
    expect(screen.getByText(/ada@example.com turned off product emails/)).toBeInTheDocument();
  });

  it('shows the server’s message for an unknown user', async () => {
    const actions = makeActions();
    show({}, ['emails'], actions);
    await settle(500);
    actions.preview.mockResolvedValue({
      ok: false,
      message: 'No user with that id or email.',
    } as never);
    await userEvent.type(screen.getByLabelText('Preview as'), 'nobody@x.dev');
    await settle(800);
    expect(await screen.findByText('No user with that id or email.')).toBeInTheDocument();
  });
});

describe('the test send', () => {
  it('saves first, sends, and says where it went', async () => {
    const actions = show();
    await settle(500);
    await userEvent.type(screen.getByLabelText('Subject'), '!');
    await userEvent.click(screen.getByRole('button', { name: 'Send test to me' }));
    await waitFor(() => expect(actions.test).toHaveBeenCalledWith(ID));
    expect(actions.save.mock.invocationCallOrder[0]).toBeLessThan(
      actions.test.mock.invocationCallOrder[0],
    );
    expect(await screen.findByText(/Test sent to ada@example.com/)).toBeInTheDocument();
  });
});

describe('sending now', () => {
  it('confirms with the exact count and audience, then sends', async () => {
    const actions = show();
    await settle(500);
    actions.count.mockResolvedValue({ ok: true, count: 214 } as never);
    await userEvent.click(screen.getByRole('button', { name: 'Send to 286 people' }));
    const dialog = await screen.findByRole('alertdialog', { name: 'Send to 214 people now?' });
    expect(dialog).toHaveTextContent(
      '“The curl-runner Cloud dashboard is live”, to team owners. You can’t unsend it.',
    );
    expect(actions.send).not.toHaveBeenCalled();
    await userEvent.click(within(dialog).getByRole('button', { name: 'Send now' }));
    await waitFor(() => expect(actions.send).toHaveBeenCalledWith(ID, null));
    expect(await screen.findByText(/Sending to 214 people/)).toBeInTheDocument();
    expect(screen.getByLabelText('Subject')).toBeDisabled();
    expect(screen.getByRole('region', { name: 'Broadcasts' })).toHaveTextContent('Sending');
    expect(refresh).toHaveBeenCalled();
  });

  it('cancels without sending', async () => {
    const actions = show();
    await settle(500);
    await userEvent.click(screen.getByRole('button', { name: 'Send to 286 people' }));
    const dialog = await screen.findByRole('alertdialog');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
    expect(actions.send).not.toHaveBeenCalled();
  });

  it('keeps the dialog open with the API’s message on a failure', async () => {
    const actions = show();
    actions.send.mockResolvedValue({
      ok: false,
      message: 'This broadcast is already going out, so it can’t change.',
    } as never);
    await settle(500);
    await userEvent.click(screen.getByRole('button', { name: 'Send to 286 people' }));
    const dialog = await screen.findByRole('alertdialog');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Send now' }));
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('already going out');
  });

  it('will not send to nobody', async () => {
    const actions = makeActions();
    show({ initialCount: 0 }, ['emails'], actions);
    await settle(500);
    expect(screen.getByRole('button', { name: 'Send to 0 people' })).toBeDisabled();
  });

  it('will not send an empty draft', async () => {
    show({
      selected: { ...draft, subject: '', body_md: '' },
      items: [{ ...draft, subject: '', body_md: '' }],
    });
    await settle(500);
    expect(screen.getByRole('button', { name: 'Send to 286 people' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Send test to me' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Schedule' })).toBeDisabled();
  });
});

describe('scheduling', () => {
  it('asks for a date and time, shows the zone and sends the UTC instant', async () => {
    const actions = show();
    await settle(500);
    await userEvent.click(screen.getByRole('button', { name: 'Schedule' }));
    const dialog = await screen.findByRole('dialog', { name: 'Schedule this broadcast' });
    expect(dialog).toHaveTextContent(/Time zone: .+/);
    expect(dialog).toHaveTextContent('Stored and sent in UTC');
    const input = within(dialog).getByLabelText('Send at');
    expect(input).toHaveAttribute('type', 'datetime-local');
    await userEvent.type(input, '2099-03-31T10:00');
    expect(dialog).toHaveTextContent(scheduledLabel(new Date('2099-03-31T10:00').toISOString()));
    await userEvent.click(within(dialog).getByRole('button', { name: 'Schedule' }));
    await waitFor(() =>
      expect(actions.send).toHaveBeenCalledWith(ID, new Date('2099-03-31T10:00').toISOString()),
    );
    expect(await screen.findByText(/Scheduled for Mar 31/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Reschedule' })).toBeInTheDocument();
  });

  it('rejects a past time before calling the API', async () => {
    const actions = show();
    await settle(500);
    await userEvent.click(screen.getByRole('button', { name: 'Schedule' }));
    const dialog = await screen.findByRole('dialog', { name: 'Schedule this broadcast' });
    await userEvent.type(within(dialog).getByLabelText('Send at'), '2020-01-01T10:00');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Schedule' }));
    expect(await within(dialog).findByRole('alert')).toHaveTextContent(
      'Pick a time in the future.',
    );
    expect(actions.send).not.toHaveBeenCalled();
  });

  it('shows a scheduled broadcast as editable, with its time', async () => {
    show({ selected: scheduled, items: [scheduled] });
    await settle(500);
    expect(screen.getByText(/Scheduled for Nov 9, 10:00 UTC/)).toBeInTheDocument();
    expect(screen.getByLabelText('Subject')).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Reschedule' })).toBeInTheDocument();
  });
});

describe('a sent broadcast', () => {
  it('is read-only and reports delivery', async () => {
    show({ selected: sent, items: [sent] });
    await settle(500);
    expect(
      screen.getByText(/Sent Nov 5/, { selector: '[role="status"] *, [role="status"]' }),
    ).toBeInTheDocument();
    expect(screen.getByLabelText('Subject')).toBeDisabled();
    expect(screen.getByLabelText('Message, in Markdown')).toBeDisabled();
    expect(screen.queryByRole('button', { name: /^Send to/ })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Schedule' })).toBeNull();
    expect(screen.getByRole('status')).toHaveTextContent('1,148 delivered · 3 failed');
  });
});

describe('without the emails permission', () => {
  it('reads and previews but changes nothing', async () => {
    const actions = show({}, ['teams']);
    await settle(500);
    expect(screen.getByLabelText('Subject')).toBeDisabled();
    expect(screen.getByRole('button', { name: 'New' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Send test to me' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Send to 286 people' })).toBeDisabled();
    expect(await screen.findByTitle('Email preview')).toBeInTheDocument();
    expect(actions.save).not.toHaveBeenCalled();
  });
});
