import { render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';
import { me } from '../fixtures';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };
const pathname = { current: '/admin' };

vi.mock('next/headers', () => ({
  cookies: async () => jar.current.store,
  headers: async () => new Headers({ 'x-pathname': '/admin' }),
}));
vi.mock('next/navigation', () => ({
  redirect: (url: string) => {
    throw new Error(`NEXT_REDIRECT ${url}`);
  },
  notFound: () => {
    throw new Error('NEXT_NOT_FOUND');
  },
  usePathname: () => pathname.current,
  useRouter: () => ({ push: vi.fn() }),
}));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { default: AdminLayout } = await import('@/app/admin/layout');
const { resetEnv } = await import('@/lib/env');

function json(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  });
}
const fetchMock = vi.fn();
function serve(adminMe: Response) {
  fetchMock.mockImplementation(async (url: string) => {
    const path = new URL(url).pathname;
    if (path === '/v1/admin/me') {
      return adminMe;
    }
    if (path === '/v1/me') {
      return json(200, { ...me, admin: true });
    }
    if (path === '/v1/info') {
      return json(200, { version: '2.7.0', sign_in: ['email'], signup: 'closed', billing: true });
    }
    throw new Error(`unexpected ${path}`);
  });
}

beforeEach(() => {
  vi.stubEnv('API_INTERNAL_URL', 'http://api.test');
  vi.stubEnv('PUBLIC_APP_URL', 'http://localhost:3000');
  resetEnv();
  fetchMock.mockReset();
  vi.stubGlobal('fetch', fetchMock);
  pathname.current = '/admin';
});

describe('admin layout', () => {
  it('is a 404 for a non-admin, so nothing shows the panel exists', async () => {
    serve(json(404, { error: 'not_found', message: 'Not found' }));
    await expect(AdminLayout({ children: <p>secret</p> })).rejects.toThrow('NEXT_NOT_FOUND');
  });

  it('shows the owner every item, none read only', async () => {
    serve(
      json(200, {
        role: 'owner',
        permissions: ['users', 'teams', 'billing', 'emails', 'settings', 'admins'],
      }),
    );
    render(await AdminLayout({ children: <p>page body</p> }));
    const nav = screen.getByRole('navigation', { name: 'Admin' });
    expect(within(nav).getAllByRole('link')).toHaveLength(10);
    expect(within(nav).queryByText('Read only')).toBeNull();
    expect(screen.getByText('Owner')).toBeInTheDocument();
    expect(screen.getByText('page body')).toBeInTheDocument();
    expect(within(nav).getByText('curl-runner server 2.7.0')).toBeInTheDocument();
    for (const link of screen.getAllByRole('link', { name: 'Back to the dashboard' })) {
      expect(link).toHaveAttribute('href', '/');
    }
  });

  it('keeps every item for a limited admin and marks the ones they can only read', async () => {
    serve(json(200, { role: 'admin', permissions: ['users'] }));
    render(await AdminLayout({ children: <p>x</p> }));
    const nav = screen.getByRole('navigation', { name: 'Admin' });
    expect(within(nav).getAllByRole('link')).toHaveLength(10);
    const marked = within(nav)
      .getAllByText('Read only')
      .map((el) => el.closest('a')?.textContent);
    expect(marked.join('|')).toMatch(/Teams.*\|Emails.*\|Stripe.*\|Settings.*\|Admins/);
    expect(within(nav).getByRole('link', { name: /^Users/ })).not.toHaveTextContent('Read only');
    expect(screen.queryByText('Owner')).toBeNull();
  });

  it('marks the current page', async () => {
    pathname.current = '/admin/teams/acme';
    serve(json(200, { role: 'owner', permissions: [] }));
    render(await AdminLayout({ children: <p>x</p> }));
    const nav = screen.getByRole('navigation', { name: 'Admin' });
    expect(within(nav).getByRole('link', { name: /^Teams/ })).toHaveAttribute(
      'aria-current',
      'page',
    );
    expect(within(nav).getByRole('link', { name: /^Overview/ })).not.toHaveAttribute(
      'aria-current',
      'page',
    );
  });
});
