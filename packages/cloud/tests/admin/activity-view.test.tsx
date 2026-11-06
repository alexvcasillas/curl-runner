import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ActivityQuery } from '@/lib/admin-activity';
import { ADA, activity, ROSA } from './admin-fixtures';

const push = vi.fn();
vi.mock('next/navigation', () => ({
  useRouter: () => ({ push, replace: vi.fn(), refresh: vi.fn() }),
}));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { ActivityView } = await import('@/components/admin/ActivityView');

const people = [
  { id: ADA, label: 'Ada' },
  { id: ROSA, label: 'Rosa Méndez' },
];
const base: ActivityQuery = {
  admin: undefined,
  group: 'all',
  since: '7d',
  cursor: undefined,
  trail: [],
};

function show(over: Partial<React.ComponentProps<typeof ActivityView>> = {}) {
  render(<ActivityView page={activity} people={people} query={base} pageSize={50} {...over} />);
}

beforeEach(() => push.mockReset());

describe('activity log', () => {
  it('lists time in UTC, admin, action with target, and IP', () => {
    show();
    const rows = screen.getAllByRole('row').slice(1);
    expect(rows[0]).toHaveTextContent('Nov 5 14:08');
    expect(rows[0]).toHaveTextContent('Ada');
    expect(rows[0]).toHaveTextContent('granted a plan to umbrella');
    expect(rows[0]).toHaveTextContent('83.44.12.9');
    expect(rows[2]).toHaveTextContent('Deleted admin');
    expect(screen.getByText('Showing 1–3 of 412')).toBeInTheDocument();
  });

  it('expands an entry into readable pairs, and collapses it again', async () => {
    show();
    const open = screen.getByRole('button', { name: /Details for granted a plan to umbrella/ });
    expect(open).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(open);
    expect(open).toHaveAttribute('aria-expanded', 'true');
    const detail = screen.getByRole('group', { name: /Details for granted a plan to umbrella/ });
    expect(within(detail).getByText('Until')).toBeInTheDocument();
    expect(within(detail).getByText('2027-03-31')).toBeInTheDocument();
    expect(within(detail).getByText('Yes')).toBeInTheDocument();
    await userEvent.click(open);
    expect(screen.queryByRole('group', { name: /Details for/ })).toBeNull();
  });

  it('shows markup in a detail as plain text', async () => {
    show();
    await userEvent.click(
      screen.getByRole('button', { name: /Details for disabled the share link/ }),
    );
    expect(screen.getByText('<img src=x onerror=alert(1)>')).toBeInTheDocument();
    expect(document.querySelector('img')).toBeNull();
  });

  it('turns a filter into a URL, and resets paging', async () => {
    show({ query: { ...base, trail: ['-'], cursor: 'c9' } });
    await userEvent.selectOptions(screen.getByLabelText('Admin'), ROSA);
    expect(push).toHaveBeenLastCalledWith(`/admin/activity?admin=${ROSA}`);
    await userEvent.selectOptions(screen.getByLabelText('Action'), 'suspension');
    expect(push).toHaveBeenLastCalledWith('/admin/activity?group=suspension');
    await userEvent.selectOptions(screen.getByLabelText('Period'), 'all');
    expect(push).toHaveBeenLastCalledWith('/admin/activity?since=all');
  });

  it('pages with a cursor trail', () => {
    show();
    expect(screen.getByRole('button', { name: 'Previous' })).toBeDisabled();
    expect(screen.getByRole('link', { name: 'Next' })).toHaveAttribute(
      'href',
      '/admin/activity?cursor=cur2&trail=-',
    );
  });

  it('exports with the current filters', () => {
    show({ query: { ...base, group: 'email', since: '30d' } });
    expect(screen.getByRole('link', { name: 'Export CSV' })).toHaveAttribute(
      'href',
      '/admin/activity/export?group=email&since=30d',
    );
  });

  it('says so when nothing matches', () => {
    show({ page: { items: [], next_cursor: null, total: 0 } });
    expect(screen.getByText('No activity matches')).toBeInTheDocument();
  });
});
