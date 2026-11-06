import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NOW, page } from './teams-fixtures';

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

const { TeamsView } = await import('@/components/admin/TeamsView');

function show(over: Partial<React.ComponentProps<typeof TeamsView>> = {}) {
  render(<TeamsView page={page} q="" filter="all" trail={[]} pageSize={50} now={NOW} {...over} />);
}

beforeEach(() => {
  push.mockReset();
  replace.mockReset();
});
afterEach(() => {
  vi.useRealTimers();
});

describe('teams list', () => {
  it('renders a row per team with the board columns', () => {
    show();
    const table = screen.getByRole('table', { name: 'Teams' });
    const acme = within(table).getByRole('row', { name: /Acme/ });
    expect(acme).toHaveTextContent('acme · since Mar 2');
    expect(acme).toHaveTextContent('ada@acme.com');
    expect(acme).toHaveTextContent('Team');
    expect(acme).toHaveTextContent('Stripe · annual');
    expect(acme).toHaveTextContent('$24');
    expect(within(acme).getByRole('link', { name: 'Acme' })).toHaveAttribute(
      'href',
      '/admin/teams/acme',
    );
    expect(within(table).getByRole('row', { name: /Initech/ })).toHaveTextContent(
      'Stripe · past due, 9 days left',
    );
    expect(within(table).getByRole('row', { name: /Umbrella/ })).toHaveTextContent(
      'Granted until Mar 31, 2027',
    );
    expect(within(table).getByRole('row', { name: /Stark Labs/ })).toHaveTextContent(
      'Granted, no end',
    );
    expect(within(table).getByRole('row', { name: /Globex Partners/ })).toHaveTextContent('None');
    const spam = within(table).getByRole('row', { name: /spam-tests/ });
    expect(spam).toHaveTextContent('Suspended');
    expect(spam).toHaveTextContent('No owner');
    expect(spam).toHaveTextContent('–');
  });

  it('shows the position, total and a cursor trail', () => {
    show();
    expect(screen.getByText('Showing 1–6 of 312')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Next' })).toHaveAttribute(
      'href',
      '/admin/teams?cursor=CURSOR_2&trail=-',
    );
    expect(screen.getByRole('button', { name: 'Previous' })).toBeDisabled();
  });

  it('goes back one page through the trail', () => {
    show({ cursor: 'CURSOR_3', trail: ['-', 'CURSOR_2'], page: { ...page, next_cursor: null } });
    expect(screen.getByRole('link', { name: 'Previous' })).toHaveAttribute(
      'href',
      '/admin/teams?cursor=CURSOR_2&trail=-',
    );
    expect(screen.getByText('Showing 101–106 of 312')).toBeInTheDocument();
  });

  it('filters by the board chips and keeps the search', async () => {
    show({ q: 'ac' });
    const group = screen.getByRole('radiogroup', { name: 'Filter teams' });
    expect(
      within(group)
        .getAllByRole('radio')
        .map((r) => r.textContent),
    ).toEqual(['All', 'Paying', 'Granted', 'Past due', 'Free', 'Suspended']);
    expect(within(group).getByRole('radio', { name: 'All' })).toBeChecked();
    await userEvent.click(within(group).getByRole('radio', { name: 'Granted' }));
    expect(push).toHaveBeenCalledWith('/admin/teams?q=ac&filter=granted');
    await userEvent.click(within(group).getByRole('radio', { name: 'Past due' }));
    expect(push).toHaveBeenCalledWith('/admin/teams?q=ac&filter=past_due');
  });

  it('searches on Enter, and as they type after a pause', async () => {
    show({ filter: 'free' });
    const box = screen.getByRole('searchbox', { name: 'Search teams' });
    expect(box).toHaveAttribute('placeholder', 'Search by name, slug or owner');
    await userEvent.type(box, 'umb{Enter}');
    expect(push).toHaveBeenCalledWith('/admin/teams?q=umb&filter=free');
    await waitFor(() => expect(replace).toHaveBeenCalledWith('/admin/teams?q=umb&filter=free'));
  });

  it('links Export CSV to the route with the current filters', () => {
    show({ q: 'ac', filter: 'stripe' });
    expect(screen.getByRole('link', { name: 'Export CSV' })).toHaveAttribute(
      'href',
      '/admin/teams/export?q=ac&filter=stripe',
    );
  });

  it('says so when nothing matches, and when there are no teams', () => {
    show({ page: { items: [], next_cursor: null, total: 0 }, q: 'zzz' });
    expect(screen.getByText('No teams match')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Clear search' })).toHaveAttribute(
      'href',
      '/admin/teams',
    );
  });
});
