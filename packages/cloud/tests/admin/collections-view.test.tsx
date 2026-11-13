import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Perm } from '@/lib/admin';
import { NOW, page, rows, stats } from './collections-fixtures';

const push = vi.fn();
const replace = vi.fn();
const refresh = vi.fn();
vi.mock('next/navigation', () => ({ useRouter: () => ({ push, replace, refresh }) }));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { CollectionsView } = await import('@/components/admin/CollectionsView');

const ok = { ok: true } as const;
function show(
  over: Partial<React.ComponentProps<typeof CollectionsView>> = {},
  permissions: Perm[] = ['teams'],
) {
  const actions = {
    disable: vi.fn(async () => ok),
    enable: vi.fn(async () => ok),
    unlist: vi.fn(async () => ok),
  };
  render(
    <CollectionsView
      page={page}
      stats={stats}
      q=""
      filter="all"
      trail={[]}
      pageSize={50}
      now={NOW}
      permissions={permissions}
      actions={actions}
      {...over}
    />,
  );
  return actions;
}

beforeEach(() => {
  push.mockReset();
  replace.mockReset();
  refresh.mockReset();
});
afterEach(() => {
  vi.useRealTimers();
});

describe('collections list', () => {
  it('renders a row per collection with the board columns', () => {
    show();
    const table = screen.getByRole('table', { name: 'Collections' });
    for (const h of ['Collection', 'Team', 'Access', 'Versions', 'Size', 'Last publish']) {
      expect(within(table).getByRole('columnheader', { name: h })).toBeInTheDocument();
    }
    // R13: signed-in opens only.
    expect(
      within(table).getByRole('columnheader', { name: 'Signed-in opens (30d)' }),
    ).toBeInTheDocument();
    expect(within(table).queryByText('Opens 30d')).toBeNull();
    const pay = within(table).getByRole('row', { name: /Payments API/ });
    expect(pay).toHaveTextContent('payments-api · git');
    expect(pay).toHaveTextContent('Public');
    expect(pay).toHaveTextContent('14');
    expect(pay).toHaveTextContent('84 KB');
    expect(pay).toHaveTextContent('612');
    expect(pay).toHaveTextContent('2 hours ago');
    expect(within(pay).getByRole('link', { name: 'acme' })).toHaveAttribute(
      'href',
      '/admin/teams/acme',
    );
    expect(within(pay).getByRole('button', { name: 'Disable link' })).toBeEnabled();
    const off = within(table).getByRole('row', { name: /Internal admin/ });
    expect(off).toHaveTextContent('Off');
    expect(off).toHaveTextContent('Never');
    expect(within(off).queryByRole('button', { name: 'Disable link' })).toBeNull();
    expect(within(off).getByRole('link', { name: 'Versions' })).toHaveAttribute(
      'href',
      '/admin/teams/acme',
    );
  });

  it('offers Enable link on a disabled one, with a pill', () => {
    show();
    const row = screen.getByRole('row', { name: /Old link/ });
    expect(row).toHaveTextContent('Link disabled');
    expect(within(row).getByRole('button', { name: 'Enable link' })).toBeEnabled();
    expect(within(row).queryByRole('button', { name: 'Disable link' })).toBeNull();
  });

  it('shows the stats footer and the range', () => {
    show();
    const footer = screen.getByRole('region', { name: 'Collections' });
    expect(footer).toHaveTextContent('1,904 collections');
    expect(footer).toHaveTextContent('2.1 GB of YAML, de-duplicated to 640 MB');
    expect(footer).toHaveTextContent('Showing 1–6 of 6');
    expect(screen.getByRole('button', { name: 'Previous' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Next' })).toBeDisabled();
  });

  it('pages with a cursor trail', () => {
    show({ page: { ...page, next_cursor: 'c2', total: 120 } });
    expect(screen.getByRole('link', { name: 'Next' })).toHaveAttribute(
      'href',
      '/admin/collections?cursor=c2&trail=-',
    );
  });

  it('exports with the current search and filter', () => {
    show({ q: 'spam', filter: 'disabled' });
    expect(screen.getByRole('link', { name: 'Export CSV' })).toHaveAttribute(
      'href',
      '/admin/collections/export?q=spam&filter=disabled',
    );
  });

  it('filters by pushing the url', async () => {
    show({ q: 'x' });
    await userEvent.click(screen.getByRole('radio', { name: 'Restricted' }));
    expect(push).toHaveBeenCalledWith('/admin/collections?q=x&filter=restricted');
    expect(screen.getByRole('radio', { name: 'Disabled by admin' })).toBeInTheDocument();
  });

  it('has a Listed filter that passes through to the url and the export', async () => {
    show({ q: 'x' });
    await userEvent.click(screen.getByRole('radio', { name: 'Listed' }));
    expect(push).toHaveBeenCalledWith('/admin/collections?q=x&filter=listed');
  });

  it('exports the Listed filter', () => {
    show({ filter: 'listed' });
    expect(screen.getByRole('link', { name: 'Export CSV' })).toHaveAttribute(
      'href',
      '/admin/collections/export?filter=listed',
    );
  });

  it('searches after a pause and on Enter', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    show();
    const box = screen.getByRole('searchbox', { name: 'Search collections' });
    expect(box).toHaveAttribute('placeholder', 'Search by name, slug or team');
    await userEvent.type(box, 'spam');
    expect(replace).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(400);
    expect(replace).toHaveBeenCalledWith('/admin/collections?q=spam');
    await userEvent.type(box, 's{Enter}');
    expect(push).toHaveBeenCalledWith('/admin/collections?q=spams');
  });

  it('says when nothing matches', () => {
    show({ page: { items: [], next_cursor: null, total: 0 }, q: 'zzz' });
    expect(screen.getByText('No collections match')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Clear search' })).toHaveAttribute(
      'href',
      '/admin/collections',
    );
  });
});

describe('disabling a link', () => {
  const open = async (name = /free-gift-cards/) => {
    await userEvent.click(
      within(screen.getByRole('row', { name })).getByRole('button', { name: 'Disable link' }),
    );
    return screen.findByRole('dialog', { name: 'Disable the share link of free-gift-cards?' });
  };

  it('asks for a reason, in the board copy, with the opens as context', async () => {
    show();
    const dialog = await open();
    expect(dialog).toHaveTextContent(
      'Anyone opening it sees “This link no longer works”. The team keeps the collection and its versions, and can’t turn the link back on.',
    );
    expect(dialog).toHaveTextContent('412 signed-in opens in the last 30 days.');
    expect(within(dialog).getByLabelText('Reason, shown to the team’s owners')).toHaveValue('');
  });

  it('needs a reason, then disables and says so', async () => {
    const actions = show();
    const dialog = await open();
    await userEvent.click(within(dialog).getByRole('button', { name: 'Disable link' }));
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('Add a reason');
    expect(actions.disable).not.toHaveBeenCalled();
    await userEvent.type(
      within(dialog).getByLabelText('Reason, shown to the team’s owners'),
      'Phishing pages',
    );
    await userEvent.click(within(dialog).getByRole('button', { name: 'Disable link' }));
    await waitFor(() =>
      expect(actions.disable).toHaveBeenCalledWith(
        'spam-tests',
        'free-gift-cards',
        'Phishing pages',
      ),
    );
    expect(
      await screen.findByText('Disabled the share link of free-gift-cards.'),
    ).toBeInTheDocument();
    expect(refresh).toHaveBeenCalled();
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('keeps the dialog open with the API’s message when it fails', async () => {
    const actions = show();
    actions.disable.mockResolvedValue({
      ok: false,
      message: 'That link is already disabled.',
    } as never);
    const dialog = await open();
    await userEvent.type(within(dialog).getByLabelText('Reason, shown to the team’s owners'), 'x');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Disable link' }));
    expect(await within(dialog).findByRole('alert')).toHaveTextContent(
      'That link is already disabled.',
    );
  });

  it('cancels without calling anything', async () => {
    const actions = show();
    const dialog = await open();
    await userEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
    expect(actions.disable).not.toHaveBeenCalled();
  });
});

describe('unlisting', () => {
  const open = async () => {
    await userEvent.click(
      within(screen.getByRole('row', { name: /Listed API/ })).getByRole('button', {
        name: 'Unlist',
      }),
    );
    return screen.findByRole('dialog', { name: 'Unlist Listed API?' });
  };

  it('pills listed rows and offers Unlist only there', () => {
    show();
    const row = screen.getByRole('row', { name: /Listed API/ });
    expect(within(row).getByText('Listed')).toBeInTheDocument();
    const pay = screen.getByRole('row', { name: /Payments API/ });
    expect(within(pay).queryByText('Listed')).toBeNull();
    expect(within(pay).queryByRole('button', { name: 'Unlist' })).toBeNull();
    expect(screen.getAllByRole('button', { name: 'Unlist' })).toHaveLength(1);
  });

  it('asks for a reason in the board copy and calls the action', async () => {
    const actions = show();
    const dialog = await open();
    expect(dialog).toHaveTextContent(
      'It leaves the public gallery. The share link keeps working. The team can list it again.',
    );
    await userEvent.click(within(dialog).getByRole('button', { name: 'Unlist' }));
    expect(await within(dialog).findByRole('alert')).toHaveTextContent('Add a reason');
    expect(actions.unlist).not.toHaveBeenCalled();
    await userEvent.type(within(dialog).getByLabelText('Reason'), 'Not an API collection');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Unlist' }));
    await waitFor(() =>
      expect(actions.unlist).toHaveBeenCalledWith('acme', 'listed-api', 'Not an API collection'),
    );
    expect(await screen.findByText('Unlisted Listed API from the gallery.')).toBeInTheDocument();
    expect(refresh).toHaveBeenCalled();
  });

  it('shows the server message on a 409 and stays open', async () => {
    const actions = show();
    actions.unlist.mockResolvedValue({
      ok: false,
      code: 'conflict',
      message: 'That collection is not listed.',
    } as never);
    const dialog = await open();
    await userEvent.type(within(dialog).getByLabelText('Reason'), 'why');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Unlist' }));
    expect(await within(dialog).findByRole('alert')).toHaveTextContent(
      'That collection is not listed.',
    );
    expect(refresh).not.toHaveBeenCalled();
  });

  it('cancels without calling anything', async () => {
    const actions = show();
    const dialog = await open();
    await userEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
    expect(actions.unlist).not.toHaveBeenCalled();
  });

  it('is disabled without the teams permission', () => {
    show({}, ['emails']);
    expect(screen.getByRole('button', { name: 'Unlist' })).toBeDisabled();
  });
});

describe('enabling a link', () => {
  it('enables it and says so', async () => {
    const actions = show();
    await userEvent.click(screen.getByRole('button', { name: 'Enable link' }));
    await waitFor(() => expect(actions.enable).toHaveBeenCalledWith('initech', 'old-link'));
    expect(await screen.findByText('Enabled the share link of Old link.')).toBeInTheDocument();
    expect(refresh).toHaveBeenCalled();
  });

  it('shows the failure', async () => {
    const actions = show();
    actions.enable.mockResolvedValue({ ok: false, message: 'That link isn’t disabled.' } as never);
    await userEvent.click(screen.getByRole('button', { name: 'Enable link' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('That link isn’t disabled.');
  });
});

describe('without the teams permission', () => {
  it('reads everything but disables the writes', () => {
    show({}, ['emails']);
    expect(screen.getByRole('table', { name: 'Collections' })).toBeInTheDocument();
    for (const b of screen.getAllByRole('button', { name: /Disable link|Enable link/ })) {
      expect(b).toBeDisabled();
    }
    expect(rows.length).toBeGreaterThan(0);
  });
});
