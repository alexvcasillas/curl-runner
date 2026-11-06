import { render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { SharedView } = await import('@/components/SharedView');

const items = [
  {
    key: 'acme/payments-api',
    name: 'Payments API',
    team: 'Acme',
    access: 'restricted' as const,
    version: 14,
    updated: '2 hours ago',
    command: 'curl-runner pull https://app.example.com/s/crsl_V8kq2',
    studioHref:
      'curl-runner://open/acme/payments-api?share=crsl_V8kq2&server=https%3A%2F%2Fapi.example.com',
  },
  {
    key: 'initech/billing-sandbox',
    name: 'Billing sandbox',
    team: 'Initech',
    access: 'public' as const,
    version: 6,
    updated: 'Oct 30',
    command: 'curl-runner pull https://app.example.com/s/crsl_Z5tL1',
    studioHref: 'curl-runner://open/initech/billing-sandbox?share=crsl_Z5tL1&server=x',
  },
];

describe('SharedView', () => {
  it('shows each row with its facts', () => {
    render(<SharedView email="jane@globex.example" items={items} />);
    const list = screen.getByRole('region', { name: 'Shared collections' });
    expect(within(list).getByText('Payments API')).toBeInTheDocument();
    expect(within(list).getByText('Restricted')).toBeInTheDocument();
    expect(within(list).getByText('Public')).toBeInTheDocument();
    expect(within(list).getByText(/Shared by Acme/)).toHaveTextContent('v14');
    expect(within(list).getByText(/Shared by Acme/)).toHaveTextContent('updated 2 hours ago');
  });

  it('gives each row Open in studio and a copyable pull command', () => {
    render(<SharedView email="jane@globex.example" items={items} />);
    const links = screen.getAllByRole('link', { name: 'Open in studio' });
    expect(links).toHaveLength(2);
    expect(links[0]).toHaveAttribute('href', items[0].studioHref);
    expect(screen.getByText(items[0].command)).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'Copy the pull command for Payments API' }),
    ).toBeInTheDocument();
  });

  it('names the email in the empty state', () => {
    render(<SharedView email="jane@globex.example" items={[]} />);
    expect(
      screen.getByRole('heading', { name: 'Nothing shared with you yet' }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/When a team shares a collection with jane@globex\.example/),
    ).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /Create a team/ })).toHaveAttribute(
      'href',
      '/new-team',
    );
    expect(screen.queryByRole('link', { name: 'Open in studio' })).toBeNull();
  });
});
