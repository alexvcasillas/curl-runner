import { render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { NOW, overview } from './fixtures';

vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { OverviewView } = await import('@/components/admin/OverviewView');

const ALL = ['users', 'teams', 'billing', 'emails', 'settings', 'admins'] as const;
const show = (o = overview, perms: readonly string[] = ALL) =>
  render(<OverviewView overview={o} permissions={[...perms] as never} now={NOW} />);

function tile(label: string) {
  const el = within(screen.getByRole('region', { name: 'Key numbers' })).getByText(label);
  return el.parentElement as HTMLElement;
}

describe('overview', () => {
  it('renders every stat from the server', () => {
    show();
    expect(tile('Users')).toHaveTextContent('1,248');
    expect(tile('Users')).toHaveTextContent('+214 in 30 days');
    expect(tile('Teams')).toHaveTextContent('312');
    expect(tile('Teams')).toHaveTextContent('+38 in 30 days');
    expect(tile('Collections')).toHaveTextContent('1,904');
    expect(tile('Collections')).toHaveTextContent('611 shared publicly, 284 restricted');
    expect(tile('Customers with access')).toHaveTextContent('5,730');
    expect(tile('MRR')).toHaveTextContent('$2,416');
    expect(tile('Paying teams')).toHaveTextContent('41');
    expect(tile('Paying teams')).toHaveTextContent('29 annual, 12 monthly');
    expect(tile('Plan overrides')).toHaveTextContent('6');
    expect(tile('Plan overrides')).toHaveTextContent('2 end within 30 days');
    expect(tile('Past due')).toHaveTextContent('1');
    expect(tile('Past due')).toHaveTextContent('Initech, 6 days left');
  });

  it('leaves out figures the server cannot give', () => {
    show();
    expect(screen.queryByText(/97%/)).toBeNull();
    expect(screen.queryByText(/this month/)).toBeNull();
  });

  it('drops the revenue tiles when billing is off', () => {
    show({ ...overview, billing: false });
    const region = screen.getByRole('region', { name: 'Key numbers' });
    expect(within(region).queryByText('MRR')).toBeNull();
    expect(within(region).queryByText('Paying teams')).toBeNull();
    expect(within(region).queryByText('Past due')).toBeNull();
    expect(screen.queryByText(/Stripe webhooks/)).toBeNull();
    expect(within(region).getByText('Users')).toBeInTheDocument();
  });

  it('says so when Stripe cannot be read', () => {
    show({ ...overview, counts: { ...overview.counts, mrr_cents: null, paying_teams: null } });
    expect(tile('MRR')).toHaveTextContent('Unavailable');
    expect(tile('Paying teams')).toHaveTextContent('Unavailable');
  });

  it('shows a calm past due tile with none', () => {
    show({ ...overview, past_due: { teams: 0, next: null } });
    expect(tile('Past due')).toHaveTextContent('0');
    expect(tile('Past due')).toHaveTextContent('None');
  });

  it('fills the status strip', () => {
    show();
    const strip = screen.getByRole('region', { name: 'Server health' });
    expect(strip).toHaveTextContent('p95 84 ms');
    expect(strip).toHaveTextContent('3 waiting, 0 failed today');
    expect(strip).toHaveTextContent('last 2 minutes ago');
    expect(strip).toHaveTextContent('Sign-up closed');
    expect(within(strip).getByRole('link', { name: 'Health' })).toHaveAttribute(
      'href',
      '/admin/health',
    );
  });

  it('flags failed email and a quiet API', () => {
    show({
      ...overview,
      p95_ms: null,
      outbox: { waiting: 0, failed_today: 2 },
      last_webhook_at: null,
    });
    const strip = screen.getByRole('region', { name: 'Server health' });
    expect(strip).toHaveTextContent('no traffic yet');
    expect(strip).toHaveTextContent('0 waiting, 2 failed today');
    expect(strip).toHaveTextContent('none yet');
  });

  it('draws both charts with a table fallback and a title per point', () => {
    show();
    const growth = screen.getByRole('img', { name: /Sign-ups .* new teams/ });
    expect(growth.querySelectorAll('title').length).toBeGreaterThanOrEqual(30);
    expect(screen.getByText('Sign-ups · 214')).toBeInTheDocument();
    expect(screen.getByText('New teams · 38')).toBeInTheDocument();
    const bars = screen.getByRole('img', { name: /Publishes per day over 14 days/ });
    expect(bars.querySelectorAll('rect[data-bar]')).toHaveLength(14);
    expect(screen.getAllByRole('table', { hidden: true }).length).toBe(2);
    expect(screen.getByText(/3,300 today/)).toBeInTheDocument();
  });

  it('lists recent admin activity', () => {
    show();
    const card = screen.getByRole('region', { name: 'Recent admin activity' });
    expect(card).toHaveTextContent('ada granted a plan to umbrella');
    expect(card).toHaveTextContent('8 minutes ago');
    expect(card).toHaveTextContent('rosa.mendez disabled the share link of spam-tests');
    expect(card).toHaveTextContent('Yesterday');
    expect(within(card).getByRole('link', { name: 'Activity log' })).toHaveAttribute(
      'href',
      '/admin/activity',
    );
  });

  it('says so when nothing has happened yet', () => {
    show({ ...overview, recent_events: [] });
    expect(screen.getByText('No admin activity yet.')).toBeInTheDocument();
  });

  it('links the quick actions for an owner', () => {
    show();
    const card = screen.getByRole('region', { name: 'Quick actions' });
    expect(within(card).getByRole('link', { name: /Grant a plan/ })).toHaveAttribute(
      'href',
      '/admin/teams',
    );
    expect(within(card).getByRole('link', { name: /New email/ })).toHaveAttribute(
      'href',
      '/admin/emails',
    );
    expect(within(card).getByRole('link', { name: /Open sign-up/ })).toHaveAttribute(
      'href',
      '/admin/settings',
    );
    expect(within(card).getByRole('link', { name: /Add an admin/ })).toHaveAttribute(
      'href',
      '/admin/admins',
    );
  });

  it('fills the tile grid evenly with billing off', () => {
    const { container } = show({ ...overview, billing: false });
    const grid = container.querySelector('[data-tiles]') as HTMLElement;
    expect(grid.dataset.tiles).toBe('5');
    expect(grid.className).toContain('lg:grid-cols-3');
    const last = grid.lastElementChild as HTMLElement;
    expect(last.className).toContain('lg:col-span-2');
    expect(last.className).toContain('sm:col-span-2');
  });

  it('needs no stretched tile with billing on', () => {
    const { container } = show();
    const grid = container.querySelector('[data-tiles]') as HTMLElement;
    expect(grid.className).toContain('lg:grid-cols-4');
    expect((grid.lastElementChild as HTMLElement).className).not.toContain('col-span');
  });

  it('shows an empty state for a publishes series with no days', () => {
    show({ ...overview, publishes_14d: [] });
    expect(screen.getByText('No publishes yet.')).toBeInTheDocument();
  });

  it('disables quick actions the admin cannot use', () => {
    show(overview, ['users']);
    const card = screen.getByRole('region', { name: 'Quick actions' });
    expect(within(card).queryByRole('link', { name: /Grant a plan/ })).toBeNull();
    expect(within(card).getAllByText('Read only').length).toBe(4);
  });
});
