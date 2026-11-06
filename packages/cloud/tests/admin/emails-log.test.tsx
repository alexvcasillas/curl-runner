import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { logRows, NOW } from './emails-fixtures';

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

const { DeliveryLogView } = await import('@/components/admin/DeliveryLogView');
const { EmailsTabs } = await import('@/components/admin/EmailsTabs');

function show(over: Partial<React.ComponentProps<typeof DeliveryLogView>> = {}) {
  render(
    <DeliveryLogView
      page={{ items: logRows, next_cursor: null }}
      q=""
      status="all"
      kind="all"
      trail={[]}
      now={NOW}
      {...over}
    />,
  );
}

beforeEach(() => {
  push.mockReset();
  replace.mockReset();
});
afterEach(() => {
  vi.useRealTimers();
});

describe('delivery log', () => {
  it('renders a row per email with the board columns', () => {
    show();
    const table = screen.getByRole('table', { name: 'Delivery log' });
    for (const h of ['Queued', 'To', 'Email', 'Status', 'Detail']) {
      expect(within(table).getByRole('columnheader', { name: h })).toBeInTheDocument();
    }
    const invite = within(table).getByRole('row', { name: /jane@globex.example/ });
    expect(invite).toHaveTextContent('13:04:12');
    expect(invite).toHaveTextContent('Customer invite');
    expect(invite).toHaveTextContent('Sent');
    const failed = within(table).getByRole('row', { name: /old@initech.example/ });
    expect(failed).toHaveTextContent('Payment failed');
    expect(failed).toHaveTextContent('Failed');
    expect(failed).toHaveTextContent('550 Mailbox does not exist');
    const waiting = within(table).getByRole('row', { name: /sam@umbrella.example/ });
    expect(waiting).toHaveTextContent('Waiting');
    expect(waiting).toHaveTextContent('Attempt 2 of 8 failed');
    // Retry is not built.
    expect(screen.queryByRole('button', { name: 'Retry' })).toBeNull();
  });

  it('shows the subject the server sends, which hides sign-in codes', () => {
    show();
    const row = screen.getByRole('row', { name: /ada@acme.example/ });
    expect(row).toHaveTextContent('Sign-in code');
    expect(row).toHaveTextContent('Your curl-runner sign-in code');
    expect(row).not.toHaveTextContent(/\b\d{6}\b/);
  });

  it('says what the footer promises, truthfully', () => {
    show();
    expect(screen.getByRole('region', { name: 'Delivery log' })).toHaveTextContent(
      'Sign-in codes never show here in full. Failed emails retry up to 8 times, waiting longer each time.',
    );
  });

  it('filters by status and kind through the url', async () => {
    show({ q: 'ada' });
    await userEvent.click(screen.getByRole('radio', { name: 'Failed' }));
    expect(push).toHaveBeenCalledWith('/admin/emails?tab=log&q=ada&status=failed');
    await userEvent.selectOptions(screen.getByLabelText('Kind'), 'broadcast');
    expect(push).toHaveBeenLastCalledWith('/admin/emails?tab=log&q=ada&kind=broadcast');
  });

  it('searches by recipient after a pause', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    show();
    const box = screen.getByRole('searchbox', { name: 'Search by recipient' });
    await userEvent.type(box, 'jane');
    expect(replace).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(400);
    expect(replace).toHaveBeenCalledWith('/admin/emails?tab=log&q=jane');
  });

  it('pages with a cursor trail, with no total', () => {
    show({ page: { items: logRows, next_cursor: 'c2' } });
    expect(screen.getByRole('link', { name: 'Next' })).toHaveAttribute(
      'href',
      '/admin/emails?tab=log&cursor=c2&trail=-',
    );
    expect(screen.getByRole('button', { name: 'Previous' })).toBeDisabled();
    expect(screen.queryByText(/^Showing/)).toBeNull();
  });

  it('goes back through the trail', () => {
    show({ cursor: 'c2', trail: ['-'], page: { items: logRows, next_cursor: 'c3' } });
    expect(screen.getByRole('link', { name: 'Previous' })).toHaveAttribute(
      'href',
      '/admin/emails?tab=log',
    );
  });

  it('says when nothing matches', () => {
    show({ page: { items: [], next_cursor: null }, status: 'failed' });
    expect(screen.getByText('No emails match')).toBeInTheDocument();
  });
});

describe('tabs', () => {
  it('marks the open tab and links to the other', () => {
    render(<EmailsTabs tab="log" />);
    expect(screen.getByRole('tab', { name: 'Delivery log' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
    expect(screen.getByRole('tab', { name: 'Broadcasts' })).toHaveAttribute(
      'href',
      '/admin/emails',
    );
    expect(screen.getByRole('tab', { name: 'Delivery log' })).toHaveAttribute(
      'href',
      '/admin/emails?tab=log',
    );
  });
});
