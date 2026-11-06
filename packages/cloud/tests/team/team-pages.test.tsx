import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from '../cookies';
import { info, me } from '../fixtures';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };
vi.mock('next/headers', () => ({
  cookies: async () => jar.current.store,
  headers: async () => new Headers({ 'x-pathname': '/t/acme/settings' }),
}));
vi.mock('next/navigation', () => ({
  redirect: (url: string) => {
    throw new Error(`NEXT_REDIRECT ${url}`);
  },
  notFound: () => {
    throw new Error('NEXT_NOT_FOUND');
  },
  usePathname: () => '/t/acme/settings',
  useRouter: () => ({ push: vi.fn(), replace: vi.fn(), refresh: vi.fn() }),
}));
vi.mock('next/cache', () => ({ revalidatePath: vi.fn() }));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const { default: SettingsPage } = await import('@/app/(app)/t/[team]/settings/page');
const { default: TeamLayout } = await import('@/app/(app)/t/[team]/layout');
const { resetEnv } = await import('@/lib/env');

const fetchMock = vi.fn();
const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

const SUMMARY = {
  plan: 'team',
  interval: null,
  status: null,
  seats: 2,
  past_due_since: null,
  can_manage: true,
  cancel_at: null,
  current_period_end: null,
  plan_source: 'override',
  override_until: '2027-03-31T23:59:59Z',
  has_subscription: false,
};

const TEAM = {
  slug: 'acme',
  name: 'Acme',
  plan: 'team',
  role: 'owner',
  members: [{ user_id: 'u1', email: 'ada@acme.example', name: 'Ada Lovelace', role: 'owner' }],
  pending_invites: [],
  suspended: false,
  suspended_at: null,
};

type Answers = { summary?: object; team?: object; me?: object; info?: object };

function serve({
  summary = SUMMARY,
  team = TEAM,
  me: meBody = me,
  info: infoBody = info,
}: Answers) {
  fetchMock.mockImplementation(async (url: string) => {
    const path = new URL(url).pathname;
    switch (path) {
      case '/v1/me':
        return json(meBody);
      case '/v1/info':
        return json(infoBody);
      case '/v1/teams/acme':
        return json(team);
      case '/v1/teams/acme/collections':
      case '/v1/teams/acme/tokens':
        return json([]);
      case '/v1/teams/acme/billing':
        return json(summary);
      case '/v1/billing/prices':
        return json({
          monthly: { unit_amount: 1000, currency: 'usd' },
          annual: { unit_amount: 9600, currency: 'usd' },
        });
      default:
        throw new Error(`unexpected ${path}`);
    }
  });
}

beforeEach(() => {
  vi.stubEnv('API_INTERNAL_URL', 'http://api.test');
  vi.stubEnv('PUBLIC_APP_URL', 'http://localhost:3000');
  resetEnv();
  fetchMock.mockReset();
  vi.stubGlobal('fetch', fetchMock);
});

async function openDelete() {
  render(await SettingsPage({ params: Promise.resolve({ team: 'acme' }) }));
  await userEvent.click(screen.getByRole('button', { name: 'Delete team' }));
  return screen.findByRole('alertdialog');
}

describe('settings: the subscription hint keys on has_subscription (R11)', () => {
  it('names the subscription for a granted team that Stripe still bills', async () => {
    serve({ summary: { ...SUMMARY, has_subscription: true } });
    expect(await openDelete()).toHaveTextContent('The subscription is cancelled now');
  });

  it('names it for a Free override on top of a live subscription', async () => {
    serve({
      summary: { ...SUMMARY, plan: 'free', has_subscription: true },
      me: { ...me, teams: [{ slug: 'acme', name: 'Acme', role: 'owner', plan: 'free' }] },
    });
    expect(await openDelete()).toHaveTextContent('The subscription is cancelled now');
  });

  it('leaves it out for a grant with nothing to cancel, though plan_source is not stripe', async () => {
    serve({ summary: { ...SUMMARY, has_subscription: false } });
    expect(await openDelete()).not.toHaveTextContent('subscription');
  });

  it('leaves it out when Stripe is the source but the subscription is gone', async () => {
    serve({ summary: { ...SUMMARY, plan_source: 'stripe', has_subscription: false } });
    expect(await openDelete()).not.toHaveTextContent('subscription');
  });
});

describe('the suspended notice', () => {
  async function layout(team: object, infoBody = info) {
    serve({ team, info: infoBody });
    render(
      await TeamLayout({
        children: <p>page body</p>,
        params: Promise.resolve({ team: 'acme' }),
      }),
    );
  }

  it('tells members, without the reason, and still shows the page', async () => {
    await layout({
      ...TEAM,
      role: 'member',
      suspended: true,
      suspended_at: '2026-10-29T10:00:00Z',
      suspended_reason: 'Phishing pages in shared collections',
    });
    const notice = screen.getByRole('alert');
    expect(notice).toHaveTextContent('Acme is suspended.');
    expect(notice).toHaveTextContent('Publishing is off and share links show “no longer works”.');
    expect(notice).toHaveTextContent('Contact support');
    expect(notice).not.toHaveTextContent('Phishing');
    expect(screen.getByText('page body')).toBeInTheDocument();
  });

  it('shows it with billing off too', async () => {
    await layout(
      { ...TEAM, suspended: true, suspended_at: '2026-10-29T10:00:00Z' },
      { ...info, billing: false },
    );
    expect(screen.getByRole('alert')).toHaveTextContent('Acme is suspended.');
  });

  it('shows nothing for a team in good standing', async () => {
    await layout(TEAM);
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.getByText('page body')).toBeInTheDocument();
  });
});
