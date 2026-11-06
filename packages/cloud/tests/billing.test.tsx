import { act, cleanup, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cookieJar } from './cookies';

const jar = { current: cookieJar({ cr_session: 'crs_ok' }) };
const incoming = { current: new Headers({ 'x-pathname': '/t/acme/billing' }) };

vi.mock('next/headers', () => ({
  cookies: async () => jar.current.store,
  headers: async () => incoming.current,
}));
vi.mock('next/navigation', () => ({
  redirect: (url: string) => {
    throw new Error(`NEXT_REDIRECT ${url}`);
  },
  notFound: () => {
    throw new Error('NEXT_NOT_FOUND');
  },
  useRouter: () => ({ refresh: refresh, replace: replace }),
  usePathname: () => path.current,
}));
const refresh = vi.fn();
const replace = vi.fn();
const path = { current: '/t/acme' };
vi.mock('next/cache', () => ({ revalidatePath: vi.fn() }));
vi.mock('next/link', () => ({
  default: ({ href, children, ...rest }: { href: string; children: React.ReactNode }) => (
    <a href={href} {...rest}>
      {children}
    </a>
  ),
}));

const lib = await import('@/lib/billing');
const actions = await import('@/app/(app)/t/[team]/billing/actions');
const { BillingView } = await import('@/components/BillingView');
const { CheckoutStatus } = await import('@/components/CheckoutStatus');
const { PlanBanner } = await import('@/components/PlanBanner');
const { UpgradeProvider } = await import('@/components/UpgradeProvider');
const { UpgradeLink, RestrictedUpsell } = await import('@/components/RestrictedUpsell');
const { resetEnv } = await import('@/lib/env');

const PRICES = {
  monthly: { unit_amount: 1000, currency: 'usd' },
  annual: { unit_amount: 9600, currency: 'usd' },
};
const FREE = {
  plan: 'free' as const,
  interval: null,
  status: null,
  seats: 2,
  past_due_since: null,
  can_manage: true,
  cancel_at: null,
  current_period_end: null,
  plan_source: 'none',
  override_until: null,
  has_subscription: false,
};
const TEAM = {
  ...FREE,
  plan: 'team' as const,
  interval: 'annual',
  status: 'active',
  seats: 3,
  current_period_end: '2027-11-05T00:00:00Z',
  plan_source: 'stripe',
  has_subscription: true,
};

describe('isStripeUrl', () => {
  it.each([
    ['https://checkout.stripe.com/c/pay/cs_test_1', true],
    ['https://billing.stripe.com/p/session/test_1', true],
    ['http://checkout.stripe.com/c/pay/x', false],
    ['https://evil.example.com/c/pay/x', false],
    ['https://checkout.stripe.com.evil.example/x', false],
    ['https://evil.example/https://checkout.stripe.com/x', false],
    ['https://user@checkout.stripe.com/x', false],
    ['https://checkout.stripe.com:8443/x', false],
    ['https://stripe.com/x', false],
    ['javascript:alert(1)', false],
    ['//checkout.stripe.com/x', false],
    ['', false],
  ])('%s -> %s', (url, ok) => {
    expect(lib.isStripeUrl(url)).toBe(ok);
  });
});

describe('prices', () => {
  it('formats USD from cents', () => {
    expect(lib.dollars(1000, 'usd')).toBe('$10');
    expect(lib.dollars(833, 'usd')).toBe('$8.33');
    expect(lib.dollars(9600, 'usd')).toBe('$96');
  });
  it('quotes the annual price as a monthly equivalent', () => {
    expect(lib.perMonth(PRICES, 'annual')).toBe('$8');
    expect(lib.perMonth(PRICES, 'monthly')).toBe('$10');
    expect(lib.billedNote(PRICES, 'annual')).toBe('billed $96 a year');
    expect(lib.billedNote(PRICES, 'monthly')).toBe('billed monthly');
  });
  it('totals for the seats and counts the free months', () => {
    expect(lib.total(PRICES, 'annual', 2)).toBe('$192 a year');
    expect(lib.total(PRICES, 'monthly', 2)).toBe('$20 a month');
    expect(lib.monthsFree(PRICES)).toBe(2);
    expect(lib.monthsFree({ ...PRICES, annual: { unit_amount: 11_800, currency: 'usd' } })).toBe(0);
  });
  it('dates the card deadline 14 days after the first failure', () => {
    expect(lib.dueBy('2026-11-05T10:00:00Z')).toBe('Nov 19');
  });
});

describe('planBanner', () => {
  const base = { billing: true, restricted: 0, members: 2 };
  it('is off with billing off', () => {
    expect(lib.planBanner({ ...base, billing: false, summary: FREE, restricted: 2 })).toBeNull();
    expect(
      lib.planBanner({ ...base, billing: false, summary: { ...TEAM, past_due_since: 'x' } }),
    ).toBeNull();
  });
  it('shows on Free with restricted collections or more than 3 members', () => {
    expect(lib.planBanner({ ...base, summary: FREE })).toBeNull();
    expect(lib.planBanner({ ...base, summary: FREE, restricted: 1 })).toBe('free');
    expect(lib.planBanner({ ...base, summary: FREE, members: 4 })).toBe('free');
    expect(lib.planBanner({ ...base, summary: FREE, members: 3 })).toBeNull();
  });
  it('does not show on Team, unless the payment failed', () => {
    expect(lib.planBanner({ ...base, summary: TEAM, restricted: 2, members: 5 })).toBeNull();
    expect(
      lib.planBanner({ ...base, summary: { ...TEAM, past_due_since: '2026-11-05T00:00:00Z' } }),
    ).toBe('past_due');
  });
  it('puts past due first', () => {
    expect(
      lib.planBanner({
        ...base,
        restricted: 1,
        summary: { ...FREE, past_due_since: '2026-11-05T00:00:00Z' },
      }),
    ).toBe('past_due');
  });
  it('is off without a summary', () => {
    expect(lib.planBanner({ ...base, summary: null })).toBeNull();
  });
});

describe('billing actions', () => {
  const fetchMock = vi.fn();
  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
    vi.stubEnv('API_INTERNAL_URL', 'http://api.test');
    vi.stubEnv('PUBLIC_APP_URL', 'http://localhost:3000');
    resetEnv();
  });
  const json = (status: number, body: unknown) =>
    new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

  it('starts checkout and redirects to Stripe', async () => {
    fetchMock.mockResolvedValue(json(200, { url: 'https://checkout.stripe.com/c/pay/cs_1' }));
    await expect(actions.startCheckout('acme', 'annual')).rejects.toThrow(
      'NEXT_REDIRECT https://checkout.stripe.com/c/pay/cs_1',
    );
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('http://api.test/v1/teams/acme/billing/checkout');
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body)).toEqual({ interval: 'annual' });
  });

  it('opens the portal and redirects to Stripe', async () => {
    fetchMock.mockResolvedValue(json(200, { url: 'https://billing.stripe.com/p/session/x' }));
    await expect(actions.openPortal('acme')).rejects.toThrow(
      'NEXT_REDIRECT https://billing.stripe.com/p/session/x',
    );
    expect(fetchMock.mock.calls[0][0]).toBe('http://api.test/v1/teams/acme/billing/portal');
  });

  it('refuses to redirect anywhere but Stripe', async () => {
    for (const url of ['https://evil.example/pay', 'http://checkout.stripe.com/x', 'nope']) {
      fetchMock.mockImplementation(async () => json(200, { url }));
      const r = await actions.startCheckout('acme', 'monthly');
      expect(r).toMatchObject({ ok: false });
      const p = await actions.openPortal('acme');
      expect(p).toMatchObject({ ok: false });
    }
  });

  it('rejects an interval the API does not know, without calling it', async () => {
    // biome-ignore lint/suspicious/noExplicitAny: a bad value from the browser
    expect(await actions.startCheckout('acme', 'weekly' as any)).toMatchObject({ ok: false });
    expect(await actions.startCheckout('Not A Slug', 'annual')).toMatchObject({ ok: false });
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('shows the API message on a 409', async () => {
    fetchMock.mockResolvedValue(
      json(409, {
        error: 'conflict',
        message: 'This team already has a subscription. Manage it from the portal.',
      }),
    );
    expect(await actions.startCheckout('acme', 'annual')).toEqual({
      ok: false,
      message: 'This team already has a subscription. Manage it from the portal.',
      code: 'conflict',
    });
  });

  it('shows the API message on a 503', async () => {
    fetchMock.mockResolvedValue(
      json(503, {
        error: 'unavailable',
        message: 'Billing is not reachable right now. Try again in a minute.',
      }),
    );
    expect(await actions.openPortal('acme')).toMatchObject({
      ok: false,
      message: 'Billing is not reachable right now. Try again in a minute.',
    });
  });

  it('gives the non-owner copy on a 403', async () => {
    fetchMock.mockResolvedValue(json(403, { error: 'forbidden', message: 'owners only' }));
    expect(await actions.startCheckout('acme', 'annual')).toMatchObject({
      ok: false,
      message: 'Only team owners can change the plan. Ask a team owner.',
      code: 'forbidden',
    });
  });
});

function view(over: Partial<React.ComponentProps<typeof BillingView>> = {}) {
  const onPortal = vi.fn(async () => ({ ok: true as const }));
  const onCheckout = vi.fn(async (_i: 'monthly' | 'annual') => ({ ok: true as const }));
  const props = {
    team: 'acme',
    teamName: 'Acme',
    owner: true,
    summary: FREE,
    prices: PRICES,
    restricted: 1,
    members: 2,
    pending: 1,
    owners: ['Ada Lovelace', 'Grace Hopper'],
    billingUrl: 'https://cloud.example.com/t/acme/billing',
    checkout: null,
    now: '2026-11-05T00:00:00Z',
    onPortal,
    onCheckout,
    ...over,
  };
  const user = userEvent.setup();
  render(
    <UpgradeProvider
      team={props.team}
      teamName={props.teamName}
      owner={props.owner}
      owners={props.owners}
      members={props.members}
      pending={props.pending}
      prices={props.prices}
      billingUrl={props.billingUrl}
      onCheckout={onCheckout}
    >
      <BillingView {...props} />
    </UpgradeProvider>,
  );
  return { user, onPortal, onCheckout };
}

describe('BillingPage', () => {
  const fetchMock = vi.fn();
  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
    vi.stubEnv('API_INTERNAL_URL', 'http://api.test');
    vi.stubEnv('PUBLIC_APP_URL', 'http://localhost:3000');
    resetEnv();
  });
  const json = (body: unknown) =>
    new Response(JSON.stringify(body), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    });

  it('is a 404 with billing off, without asking for a plan', async () => {
    fetchMock.mockImplementation(async () =>
      json({ version: '2.6.0', sign_in: ['email'], signup: 'open', billing: false }),
    );
    const { default: BillingPage } = await import('@/app/(app)/t/[team]/billing/page');
    await expect(
      BillingPage({ params: Promise.resolve({ team: 'acme' }), searchParams: Promise.resolve({}) }),
    ).rejects.toThrow('NEXT_NOT_FOUND');
    const urls = fetchMock.mock.calls.map((c) => String(c[0]));
    expect(urls.some((u) => u.includes('/billing'))).toBe(false);
  });
});

describe('BillingView', () => {
  it('Free, owner: plan cards with API prices, annual first', () => {
    view();
    expect(screen.getByRole('heading', { name: 'Free plan' })).toBeInTheDocument();
    expect(screen.getByText('2 of 3')).toBeInTheDocument();
    expect(screen.getByText('Team only')).toBeInTheDocument();
    const team = screen
      .getByRole('heading', { name: 'Team', level: 3 })
      .closest('div.card, section, div') as HTMLElement;
    expect(team).toBeTruthy();
    expect(screen.getByText('$8')).toBeInTheDocument();
    expect(screen.getByText('billed $96 a year')).toBeInTheDocument();
    expect(screen.getByText('For Acme today: 2 members, $192 a year.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Manage billing' })).toBeNull();
  });

  it('switches to monthly prices', async () => {
    const { user } = view();
    await user.click(screen.getByRole('radio', { name: 'Monthly' }));
    expect(screen.getByText('$10')).toBeInTheDocument();
    expect(screen.getByText('billed monthly')).toBeInTheDocument();
    expect(screen.getByText('For Acme today: 2 members, $20 a month.')).toBeInTheDocument();
  });

  it('Upgrade to Team opens the dialog, and Continue calls checkout', async () => {
    const { user, onCheckout } = view();
    await user.click(screen.getByRole('button', { name: 'Upgrade to Team' }));
    const dialog = await screen.findByRole('dialog', { name: 'Upgrade Acme to Team' });
    expect(within(dialog).getByText('Annual · $8 per member a month')).toBeInTheDocument();
    expect(
      within(dialog).getByText('Billed $96 per member a year. Two months free.'),
    ).toBeInTheDocument();
    expect(within(dialog).getByText('Monthly · $10 per member a month')).toBeInTheDocument();
    expect(within(dialog).getByText('1, billed when accepted')).toBeInTheDocument();
    expect(within(dialog).getByText('$192 a year')).toBeInTheDocument();
    await user.click(within(dialog).getByRole('radio', { name: /Monthly/ }));
    expect(within(dialog).getByText('$20 a month')).toBeInTheDocument();
    await user.click(within(dialog).getByRole('button', { name: 'Continue to payment' }));
    expect(onCheckout).toHaveBeenCalledWith('monthly');
  });

  it('shows the checkout error in the dialog', async () => {
    const { user, onCheckout } = view();
    onCheckout.mockResolvedValue({
      ok: false,
      message: 'Billing is not reachable right now. Try again in a minute.',
    } as never);
    await user.click(screen.getByRole('button', { name: 'Upgrade to Team' }));
    const dialog = await screen.findByRole('dialog');
    await user.click(within(dialog).getByRole('button', { name: 'Continue to payment' }));
    expect(await within(dialog).findByRole('alert')).toHaveTextContent(
      'Billing is not reachable right now. Try again in a minute.',
    );
    expect(within(dialog).getByRole('button', { name: 'Continue to payment' })).toBeEnabled();
  });

  it('Free, member: read-only', () => {
    view({ owner: false, summary: { ...FREE, can_manage: false } });
    expect(screen.queryByRole('button', { name: 'Upgrade to Team' })).toBeNull();
    expect(screen.getByText('Ask a team owner to upgrade.')).toBeInTheDocument();
  });

  it('Team: active, with Manage billing for the portal', async () => {
    const { user, onPortal } = view({ summary: TEAM, restricted: 1 });
    expect(screen.getByRole('heading', { name: 'Team plan' })).toBeInTheDocument();
    expect(screen.getByText('Active')).toBeInTheDocument();
    expect(screen.getByText('$288 a year')).toBeInTheDocument();
    expect(screen.getByText(/Renews on Nov 5, 2027/)).toBeInTheDocument();
    expect(screen.getByText(/switching to monthly and cancelling/)).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Plans' })).toBeNull();
    await user.click(screen.getByRole('button', { name: 'Manage billing' }));
    expect(onPortal).toHaveBeenCalled();
  });

  it('Team, monthly: next invoice per month', () => {
    view({ summary: { ...TEAM, interval: 'monthly' } });
    expect(screen.getByText('$30 a month')).toBeInTheDocument();
    expect(screen.getByText(/Billed monthly/)).toBeInTheDocument();
    expect(screen.getByText(/switching to annual and cancelling/)).toBeInTheDocument();
  });

  it('Team, member: no portal', () => {
    view({ owner: false, summary: { ...TEAM, can_manage: false } });
    expect(screen.queryByRole('button', { name: 'Manage billing' })).toBeNull();
  });

  it('cancel scheduled: says when it ends and offers Renew plan', async () => {
    const { user, onPortal } = view({ summary: { ...TEAM, cancel_at: '2027-11-05T00:00:00Z' } });
    expect(screen.getByText('Ends Nov 5, 2027')).toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent(
      'The Team plan is cancelled and ends on Nov 5, 2027. After that Acme moves to Free, and nothing is deleted.',
    );
    expect(screen.getByText('None')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Renew plan' }));
    expect(onPortal).toHaveBeenCalled();
  });

  it('past due: red notice with the card deadline and Update card', async () => {
    const { user, onPortal } = view({
      summary: { ...TEAM, status: 'past_due', past_due_since: '2026-11-05T10:00:00Z' },
    });
    expect(screen.getByText('Past due')).toBeInTheDocument();
    const alert = screen.getByRole('alert');
    expect(alert).toHaveTextContent('We couldn’t take the last payment.');
    expect(alert).toHaveTextContent('Update it by Nov 19 to keep the Team plan.');
    await user.click(within(alert).getByRole('button', { name: 'Update card' }));
    expect(onPortal).toHaveBeenCalled();
  });

  it('shows a portal error', async () => {
    const { user, onPortal } = view({ summary: TEAM });
    onPortal.mockResolvedValue({ ok: false, message: 'Nope, try later.' } as never);
    await user.click(screen.getByRole('button', { name: 'Manage billing' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Nope, try later.');
  });

  it('granted: no portal, nothing to pay, and the end comes from override_until', () => {
    view({
      summary: {
        ...TEAM,
        plan_source: 'override',
        interval: null,
        has_subscription: false,
        // A Stripe cancel date must not be read as the end of the grant.
        cancel_at: '2026-12-03T00:00:00Z',
        override_until: '2027-03-31T23:59:59Z',
      },
    });
    expect(screen.getByText('Granted')).toBeInTheDocument();
    expect(screen.getByText(/until Mar 31, 2027. There’s nothing to pay/)).toBeInTheDocument();
    expect(screen.queryByText(/Dec 3/)).toBeNull();
    expect(screen.queryByRole('button', { name: 'Manage billing' })).toBeNull();
  });

  it('granted over a live subscription says Stripe still bills, and keeps the portal', async () => {
    const { user, onPortal } = view({
      summary: {
        ...TEAM,
        plan_source: 'override',
        has_subscription: true,
        override_until: '2027-03-31T23:59:59Z',
      },
    });
    expect(
      screen.getAllByText(
        /Stripe still bills this team. Manage billing to cancel the subscription./,
      ),
    ).toHaveLength(2);
    expect(screen.queryByText(/nothing to pay/)).toBeNull();
    await user.click(screen.getByRole('button', { name: 'Manage billing' }));
    expect(onPortal).toHaveBeenCalled();
  });

  it('granted over a live subscription hides the portal from non-owners', () => {
    view({
      owner: false,
      summary: { ...TEAM, plan_source: 'override', has_subscription: true },
    });
    expect(screen.queryByRole('button', { name: 'Manage billing' })).toBeNull();
    expect(screen.getAllByText(/An owner can cancel it under Manage billing./)).toHaveLength(2);
    expect(screen.queryByText(/Manage billing to cancel/)).toBeNull();
  });

  it('granted over a live subscription labels the amount as the next invoice', () => {
    view({
      summary: { ...TEAM, plan_source: 'override', has_subscription: true, seats: 3 },
    });
    expect(screen.getByText('Next invoice')).toBeInTheDocument();
    expect(screen.queryByText('Cost')).toBeNull();
    // 3 members on the annual price.
    expect(screen.getByText('Next invoice').nextElementSibling).toHaveTextContent('$288 a year');
  });

  it('a Free override over a live subscription keeps the portal and hides Upgrade', async () => {
    const { user, onPortal } = view({
      summary: { ...FREE, plan_source: 'override', has_subscription: true, seats: 2 },
    });
    expect(
      screen.getByText('Stripe still bills this team. Manage billing to cancel the subscription.'),
    ).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Upgrade to Team' })).toBeNull();
    expect(screen.queryByText('Ask a team owner to upgrade.')).toBeNull();
    await user.click(screen.getByRole('button', { name: 'Manage billing' }));
    expect(onPortal).toHaveBeenCalled();
  });

  it('a Free override over a live subscription tells non-owners an owner can cancel', () => {
    view({
      owner: false,
      summary: { ...FREE, plan_source: 'override', has_subscription: true },
    });
    expect(
      screen.getByText(
        'Stripe still bills this team. An owner can cancel it under Manage billing.',
      ),
    ).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Manage billing' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Upgrade to Team' })).toBeNull();
  });

  it('granted with no end date says so', () => {
    view({
      summary: {
        ...TEAM,
        plan_source: 'override',
        interval: null,
        has_subscription: false,
        override_until: null,
      },
    });
    expect(
      screen.getByText(/gave Acme the Team plan with no end date. There’s nothing to pay/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        'The grant has no end date. If it ever ends, Acme moves to Free unless an owner upgrades.',
      ),
    ).toBeInTheDocument();
  });

  it('says Unavailable, not a dash, when prices are missing on Team', () => {
    view({ summary: TEAM, prices: null });
    expect(screen.getByText('Unavailable')).toBeInTheDocument();
  });

  it('copes with prices it could not read', () => {
    view({ prices: null });
    expect(
      screen.getByText('Prices aren’t available right now. Try again in a minute.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Upgrade to Team' })).toBeDisabled();
  });
});

describe('CheckoutStatus', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    refresh.mockReset();
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it('shows nothing without a checkout param', () => {
    const { container } = render(<CheckoutStatus plan="free" checkout={null} teamName="Acme" />);
    expect(container).toBeEmptyDOMElement();
  });

  it('finishing: refreshes every 2 seconds, then stops at 30', () => {
    render(<CheckoutStatus plan="free" checkout="done" teamName="Acme" />);
    expect(screen.getByRole('status')).toHaveTextContent(
      'Finishing up with Stripe. This takes a few seconds, and the page updates on its own.',
    );
    act(() => {
      vi.advanceTimersByTime(6000);
    });
    expect(refresh).toHaveBeenCalledTimes(3);
    act(() => {
      vi.advanceTimersByTime(24_000);
    });
    expect(refresh).toHaveBeenCalledTimes(14);
    expect(screen.getByRole('status')).toHaveTextContent(
      'Payment received. Stripe hasn’t confirmed it to us yet, so the plan updates in a minute. You don’t need to pay again.',
    );
    act(() => {
      vi.advanceTimersByTime(20_000);
    });
    expect(refresh).toHaveBeenCalledTimes(14);
  });

  it('slow: Refresh refreshes once', async () => {
    render(<CheckoutStatus plan="free" checkout="done" teamName="Acme" />);
    act(() => {
      vi.advanceTimersByTime(30_000);
    });
    refresh.mockReset();
    screen.getByRole('button', { name: 'Refresh' }).click();
    expect(refresh).toHaveBeenCalledTimes(1);
  });

  it('thanks once the plan reads team, and stops polling', () => {
    const { rerender } = render(<CheckoutStatus plan="free" checkout="done" teamName="Acme" />);
    act(() => {
      vi.advanceTimersByTime(4000);
    });
    rerender(<CheckoutStatus plan="team" checkout="done" teamName="Acme" />);
    expect(screen.getByRole('status')).toHaveTextContent(
      'Thanks. Acme is on the Team plan. Restricted sharing and unlimited members are on now.',
    );
    refresh.mockReset();
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(refresh).not.toHaveBeenCalled();
  });

  it('drops ?checkout=done once the plan reads team, keeping the thank-you up', () => {
    path.current = '/t/acme/billing';
    replace.mockReset();
    const { rerender } = render(<CheckoutStatus plan="team" checkout="done" teamName="Acme" />);
    expect(replace).toHaveBeenCalledWith('/t/acme/billing');
    rerender(<CheckoutStatus plan="team" checkout={null} teamName="Acme" />);
    expect(screen.getByRole('status')).toHaveTextContent('Thanks. Acme is on the Team plan.');
    // A later load without the param shows nothing.
    cleanup();
    const again = render(<CheckoutStatus plan="team" checkout={null} teamName="Acme" />);
    expect(again.container).toBeEmptyDOMElement();
  });

  it('cancelled: says nothing was charged', () => {
    render(<CheckoutStatus plan="free" checkout="cancelled" teamName="Acme" />);
    expect(screen.getByRole('status')).toHaveTextContent(
      'Checkout cancelled. Nothing was charged. Acme is still on Free.',
    );
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(refresh).not.toHaveBeenCalled();
  });
});

describe('PlanBanner', () => {
  beforeEach(() => {
    path.current = '/t/acme';
  });

  it('stays out of the Billing page, which says it itself', () => {
    path.current = '/t/acme/billing';
    const { container } = render(
      <PlanBanner kind="past_due" teamName="Acme" dueBy="Nov 19" canManage onPortal={vi.fn()} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it('Free: the spec text and an Upgrade button that opens the dialog', async () => {
    const onCheckout = vi.fn(async () => ({ ok: true as const }));
    const user = userEvent.setup();
    render(
      <UpgradeProvider
        team="acme"
        teamName="Acme"
        owner
        owners={['Ada Lovelace']}
        members={2}
        pending={0}
        prices={PRICES}
        billingUrl="https://x.test/t/acme/billing"
        onCheckout={onCheckout}
      >
        <PlanBanner kind="free" teamName="Acme" dueBy={null} canManage onPortal={vi.fn()} />
      </UpgradeProvider>,
    );
    const banner = screen.getByRole('status');
    expect(banner).toHaveTextContent('Acme is on the Free plan now. Nothing was deleted.');
    expect(banner).toHaveTextContent(
      'Restricted collections keep serving the people already on their lists. To publish to them, add people, or add members past 3, upgrade or make those collections public or off.',
    );
    await user.click(within(banner).getByRole('button', { name: 'Upgrade to Team' }));
    expect(await screen.findByRole('dialog', { name: 'Upgrade Acme to Team' })).toBeInTheDocument();
  });

  it('past due: the red text and Update card', async () => {
    const onPortal = vi.fn(async () => ({ ok: true as const }));
    const user = userEvent.setup();
    render(
      <PlanBanner kind="past_due" teamName="Acme" dueBy="Nov 19" canManage onPortal={onPortal} />,
    );
    const banner = screen.getByRole('alert');
    expect(banner).toHaveTextContent('We couldn’t take Acme’s last payment.');
    expect(banner).toHaveTextContent(
      'Update the card by Nov 19 to keep the Team plan. After that, Acme moves to Free and nothing is deleted.',
    );
    await user.click(within(banner).getByRole('button', { name: 'Update card' }));
    expect(onPortal).toHaveBeenCalled();
  });

  it('past due, member: no Update card', () => {
    render(
      <PlanBanner
        kind="past_due"
        teamName="Acme"
        dueBy="Nov 19"
        canManage={false}
        onPortal={vi.fn()}
      />,
    );
    expect(screen.queryByRole('button', { name: 'Update card' })).toBeNull();
  });
});

describe('UpgradeDialog entry points', () => {
  const provider = (owner: boolean, children: React.ReactNode, onCheckout = vi.fn()) =>
    render(
      <UpgradeProvider
        team="acme"
        teamName="Acme"
        owner={owner}
        owners={['Ada Lovelace', 'Grace Hopper']}
        members={2}
        pending={1}
        prices={PRICES}
        billingUrl="https://cloud.example.com/t/acme/billing"
        onCheckout={onCheckout}
      >
        {children}
      </UpgradeProvider>,
    );

  it('Restricted: the reason is about sharing', async () => {
    const user = userEvent.setup();
    provider(true, <RestrictedUpsell team="acme" teamName="Acme" billing />);
    await user.click(screen.getByRole('button', { name: 'Upgrade Acme' }));
    const dialog = await screen.findByRole('dialog', { name: 'Upgrade Acme to Team' });
    expect(dialog).toHaveTextContent('Restricted sharing is on the Team plan.');
  });

  it('Members: the reason is the 3 member limit', async () => {
    const user = userEvent.setup();
    provider(true, <UpgradeLink team="acme" billing />);
    await user.click(screen.getByRole('button', { name: 'Upgrade to Team' }));
    const dialog = await screen.findByRole('dialog', { name: 'Upgrade Acme to Team' });
    expect(dialog).toHaveTextContent('The Free plan allows 3 members, counting pending invites.');
  });

  it('billing off: the link renders nothing, and Restricted stays plain text', () => {
    const first = provider(true, <UpgradeLink team="acme" billing={false} />);
    expect(screen.queryByRole('button', { name: 'Upgrade to Team' })).toBeNull();
    expect(screen.queryByRole('link')).toBeNull();
    first.unmount();
    provider(true, <RestrictedUpsell team="acme" teamName="Acme" billing={false} />);
    expect(screen.queryByRole('button')).toBeNull();
    expect(screen.queryByRole('link')).toBeNull();
  });

  it('non-owner: read-only, names the owners and has a billing link to copy', async () => {
    const user = userEvent.setup();
    const onCheckout = vi.fn();
    provider(false, <RestrictedUpsell team="acme" teamName="Acme" billing />, onCheckout);
    await user.click(screen.getByRole('button', { name: 'Upgrade Acme' }));
    const dialog = await screen.findByRole('dialog', {
      name: 'Restricted sharing needs the Team plan',
    });
    expect(dialog).toHaveTextContent('Only team owners can change Acme’s plan.');
    expect(dialog).toHaveTextContent('Ask Ada Lovelace or Grace Hopper to upgrade.');
    expect(dialog).toHaveTextContent('https://cloud.example.com/t/acme/billing');
    expect(within(dialog).queryByRole('button', { name: 'Continue to payment' })).toBeNull();
    await user.click(within(dialog).getByRole('button', { name: 'Got it' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(onCheckout).not.toHaveBeenCalled();
  });

  it('without a provider the link goes to Billing', () => {
    render(<UpgradeLink team="acme" billing />);
    expect(screen.getByRole('link', { name: 'Upgrade to Team' })).toHaveAttribute(
      'href',
      '/t/acme/billing',
    );
  });
});
