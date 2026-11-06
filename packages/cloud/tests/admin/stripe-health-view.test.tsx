import { act, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { health, NOW, stripe } from './admin-fixtures';

const refresh = vi.fn();
vi.mock('next/navigation', () => ({ useRouter: () => ({ push: vi.fn(), refresh }) }));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { StripeView } = await import('@/components/admin/StripeView');
const { HealthView } = await import('@/components/admin/HealthView');

const tile = (label: string) =>
  within(screen.getByRole('region', { name: 'Key numbers' })).getByText(label)
    .parentElement as HTMLElement;

describe('stripe', () => {
  const show = () => render(<StripeView stats={stripe} granted={{ teams: 6, seats: 38 }} />);

  it('shows the revenue numbers, in dollars from cents', () => {
    show();
    expect(tile('MRR')).toHaveTextContent('$2,416');
    expect(tile('MRR')).toHaveTextContent('+$264 this month');
    expect(tile('ARR')).toHaveTextContent('$28,992');
    expect(tile('Paying teams')).toHaveTextContent('41');
    expect(tile('Billed seats')).toHaveTextContent('145');
    expect(tile('Billed seats')).toHaveTextContent('Average 3.5 per team');
    expect(tile('Past due')).toHaveTextContent('1');
    expect(tile('Past due')).toHaveTextContent('$70 at risk');
  });

  it('leaves out a churn rate the server does not give', () => {
    show();
    expect(screen.queryByText('1.8%')).toBeNull();
    expect(tile('Cancelled')).toHaveTextContent('1');
  });

  it('draws the chart with a fallback table and net new', () => {
    show();
    const chart = screen.getByRole('img', { name: /MRR rising from \$1,380 to \$2,424/ });
    expect(chart.tagName.toLowerCase()).toBe('svg');
    expect(screen.getByRole('table', { name: /MRR per day/, hidden: true })).toBeInTheDocument();
    expect(screen.getByText(/Net new this month/)).toHaveTextContent('+$264');
  });

  it('lists granted teams apart from revenue', () => {
    show();
    const mix = screen.getByRole('region', { name: 'Plan mix' });
    expect(within(mix).getByText('Granted by an admin')).toBeInTheDocument();
    expect(mix).toHaveTextContent('38 seats');
    expect(within(mix).getByText(/not revenue/i)).toBeInTheDocument();
    expect(mix).toHaveTextContent('214 in 24 h');
  });

  it('lists events with a Stripe link that opens safely', () => {
    show();
    const events = screen.getByRole('region', { name: 'Recent subscription events' });
    expect(events).toHaveTextContent('Acme');
    expect(events).toHaveTextContent('+$16');
    expect(events).toHaveTextContent('−$10');
    expect(within(events).getByText('Payment failed')).toBeInTheDocument();
    const link = within(events).getByRole('link', { name: 'evt_1QxN4' });
    expect(link).toHaveAttribute('href', 'https://dashboard.stripe.com/events/evt_1QxN4');
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', expect.stringContaining('noreferrer'));
  });

  it('opens the Stripe dashboard in a new tab', () => {
    show();
    const link = screen.getByRole('link', { name: 'Open Stripe dashboard' });
    expect(link).toHaveAttribute('href', 'https://dashboard.stripe.com');
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', expect.stringContaining('noreferrer'));
  });

  it('says billing is off, with the granted count', () => {
    render(<StripeView stats={null} granted={{ teams: 3, seats: 9 }} />);
    expect(screen.getByText('Billing is off on this server')).toBeInTheDocument();
    expect(screen.getByText(/3 teams/)).toBeInTheDocument();
    expect(screen.queryByRole('region', { name: 'Key numbers' })).toBeNull();
  });
});

describe('health', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    refresh.mockReset();
  });
  afterEach(() => vi.useRealTimers());

  const show = () => render(<HealthView health={health} now={NOW.toISOString()} />);

  it('shows the numbers the server has, and none it does not', () => {
    show();
    expect(tile('Server')).toHaveTextContent('2.7.0');
    expect(tile('Server')).toHaveTextContent('up 4 days');
    expect(tile('API p95')).toHaveTextContent('84 ms');
    expect(tile('API p95')).toHaveTextContent('1,200 requests in 5 min');
    expect(tile('Postgres')).toHaveTextContent('1.4 GB');
    expect(tile('Postgres')).toHaveTextContent('12 of 50 connections · migration 4');
    expect(tile('Email queue')).toHaveTextContent('3');
    expect(tile('Email queue')).toHaveTextContent('Oldest 40 s · 1 failed today');
    expect(tile('Errors (5xx)')).toHaveTextContent('3');
    expect(screen.queryByText('Backups')).toBeNull();
    expect(screen.queryByText(/sha-/)).toBeNull();
  });

  it('says the counters reset on restart', () => {
    show();
    expect(screen.getAllByText(/reset(s)? (on|when the server) restart/i).length).toBeGreaterThan(
      0,
    );
  });

  it('lists jobs with their status', () => {
    show();
    const jobs = screen.getByRole('region', { name: 'Background jobs' });
    expect(jobs).toHaveTextContent('Email outbox');
    expect(jobs).toHaveTextContent('Ran 4 s ago');
    expect(within(jobs).getByText('OK')).toBeInTheDocument();
    expect(within(jobs).getByText('Failed')).toBeInTheDocument();
    expect(within(jobs).getByText('Stale')).toBeInTheDocument();
    expect(jobs).toHaveTextContent('Hasn’t run since the server started');
  });

  it('lists webhooks and rate limits', () => {
    show();
    expect(screen.getByRole('region', { name: 'Stripe webhooks' })).toHaveTextContent(
      'customer.subscription.updated',
    );
    expect(screen.getByRole('region', { name: 'Stripe webhooks' })).toHaveTextContent(
      '0 bad signatures · 2 duplicates skipped · last 2 min ago',
    );
    const limits = screen.getByRole('region', { name: 'Rate limits' });
    expect(limits).toHaveTextContent('email-start:ip');
    expect(limits).toHaveTextContent('Email sign-in starts, per IP');
    expect(limits).toHaveTextContent('41');
  });

  it('refreshes every 30 seconds and stops when it goes away', () => {
    const { unmount } = show();
    expect(screen.getByText(/refreshes every 30 s/)).toBeInTheDocument();
    expect(refresh).not.toHaveBeenCalled();
    act(() => {
      vi.advanceTimersByTime(30_000);
    });
    expect(refresh).toHaveBeenCalledTimes(1);
    act(() => {
      vi.advanceTimersByTime(60_000);
    });
    expect(refresh).toHaveBeenCalledTimes(3);
    unmount();
    act(() => {
      vi.advanceTimersByTime(60_000);
    });
    expect(refresh).toHaveBeenCalledTimes(3);
  });
});
