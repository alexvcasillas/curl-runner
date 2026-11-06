import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const push = vi.fn();
vi.mock('next/navigation', () => ({ useRouter: () => ({ push }), usePathname: () => '/admin' }));

const { CommandSearch } = await import('@/components/admin/CommandSearch');

const hits = {
  users: [{ id: 'u1', email: 'sam@umbrella.dev', name: 'Sam Reyes' }],
  teams: [{ slug: 'umbrella', name: 'Umbrella', plan: 'team' }],
  collections: [{ team: 'umbrella', slug: 'payments-api', name: 'Payments API' }],
};
const fetchMock = vi.fn();
const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

beforeEach(() => {
  push.mockReset();
  fetchMock.mockReset();
  fetchMock.mockImplementation(async () => json(hits));
  vi.stubGlobal('fetch', fetchMock);
});

async function open(user = userEvent.setup()) {
  render(<CommandSearch />);
  await user.keyboard('{Meta>}k{/Meta}');
  return { user, input: await screen.findByRole('combobox') };
}

describe('command search', () => {
  it('opens with Cmd+K and with Ctrl+K', async () => {
    const user = userEvent.setup();
    render(<CommandSearch />);
    expect(screen.queryByRole('combobox')).toBeNull();
    await user.keyboard('{Control>}k{/Control}');
    expect(await screen.findByRole('combobox')).toBeInTheDocument();
  });

  it('opens from the button', async () => {
    const user = userEvent.setup();
    render(<CommandSearch />);
    await user.click(screen.getByRole('button', { name: /Find users, teams, links/ }));
    expect(await screen.findByRole('combobox')).toBeInTheDocument();
  });

  it('waits 200ms and sends one request for a burst of typing', async () => {
    const { user, input } = await open();
    await user.type(input, 'umb');
    expect(fetchMock).not.toHaveBeenCalled();
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1), { timeout: 1000 });
    expect(fetchMock.mock.calls[0][0]).toBe('/admin/search?q=umb');
  });

  it('groups results under Users, Teams and Collections', async () => {
    const { user, input } = await open();
    await user.type(input, 'umb');
    expect(await screen.findByText('sam@umbrella.dev')).toBeInTheDocument();
    const groups = screen.getAllByRole('group').map((g) => g.getAttribute('aria-label'));
    expect(groups).toEqual(['Users', 'Teams', 'Collections']);
    const hrefs = screen.getAllByRole('option').map((o) => o.getAttribute('href'));
    expect(hrefs).toEqual([
      '/admin/users/u1',
      '/admin/teams/umbrella',
      '/admin/collections?q=payments-api',
    ]);
  });

  it('moves with the arrow keys and opens the active result with Enter', async () => {
    const { user, input } = await open();
    await user.type(input, 'umb');
    await screen.findByText('sam@umbrella.dev');
    await user.keyboard('{ArrowDown}');
    expect(screen.getAllByRole('option')[1]).toHaveAttribute('aria-selected', 'true');
    await user.keyboard('{Enter}');
    expect(push).toHaveBeenCalledWith('/admin/teams/umbrella');
  });

  it('wraps around at the ends', async () => {
    const { user, input } = await open();
    await user.type(input, 'umb');
    await screen.findByText('sam@umbrella.dev');
    await user.keyboard('{ArrowUp}');
    expect(screen.getAllByRole('option')[2]).toHaveAttribute('aria-selected', 'true');
  });

  it('keeps the previous results while the next search runs', async () => {
    const { user, input } = await open();
    await user.type(input, 'umb');
    await screen.findByText('sam@umbrella.dev');
    let release: (r: Response) => void = () => {};
    fetchMock.mockImplementation(
      () =>
        new Promise<Response>((r) => {
          release = r;
        }),
    );
    await user.type(input, 'r');
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    expect(screen.getByText('sam@umbrella.dev')).toBeInTheDocument();
    expect(screen.queryByText('Searching…')).toBeNull();
    release(json({ users: [], teams: [], collections: [] }));
    expect(await screen.findByText('Nothing found for “umbr”.')).toBeInTheDocument();
  });

  it('says when nothing matches', async () => {
    fetchMock.mockImplementation(async () => json({ users: [], teams: [], collections: [] }));
    const { user, input } = await open();
    await user.type(input, 'zzz');
    expect(await screen.findByText('Nothing found for “zzz”.')).toBeInTheDocument();
  });

  it('says when the search failed', async () => {
    fetchMock.mockImplementation(async () => json({ error: 'unavailable' }, 503));
    const { user, input } = await open();
    await user.type(input, 'umb');
    expect(
      await screen.findByText('Search is not available right now. Try again.'),
    ).toBeInTheDocument();
  });

  it('does not search for an empty box', async () => {
    const { user, input } = await open();
    await user.type(input, '  ');
    await new Promise((r) => setTimeout(r, 300));
    expect(fetchMock).not.toHaveBeenCalled();
    expect(screen.getByText('Type to search users, teams and collections.')).toBeInTheDocument();
  });
});
