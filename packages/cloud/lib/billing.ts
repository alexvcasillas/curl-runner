import { shortDate } from './time';
import type { BillingSummary, Price, Prices } from './types';

export type Interval = 'monthly' | 'annual';

export const isInterval = (v: unknown): v is Interval => v === 'monthly' || v === 'annual';

const STRIPE_HOSTS = ['checkout.stripe.com', 'billing.stripe.com'];

/**
 * Whether a URL the API gave us is one of Stripe's own pages. Checked before any
 * redirect, so a bad API answer can't send someone to another site.
 */
export function isStripeUrl(value: string): boolean {
  let u: URL;
  try {
    u = new URL(value);
  } catch {
    return false;
  }
  return (
    u.protocol === 'https:' &&
    STRIPE_HOSTS.includes(u.hostname) &&
    u.port === '' &&
    u.username === '' &&
    u.password === ''
  );
}

/** "$10", "$8.33". USD or whichever currency Stripe holds, from cents. */
export function dollars(cents: number, currency: string): string {
  const whole = Math.abs(cents % 100) < 0.005;
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: currency.toUpperCase(),
    minimumFractionDigits: whole ? 0 : 2,
    maximumFractionDigits: 2,
  }).format(cents / 100);
}

const price = (prices: Prices, interval: Interval): Price =>
  interval === 'annual' ? prices.annual : prices.monthly;

/** What a member costs a month. Annual shows its monthly equivalent. */
export function perMonth(prices: Prices, interval: Interval): string {
  const p = price(prices, interval);
  return dollars(interval === 'annual' ? p.unit_amount / 12 : p.unit_amount, p.currency);
}

/** "billed $96 a year" or "billed monthly". */
export function billedNote(prices: Prices, interval: Interval): string {
  return interval === 'annual'
    ? `billed ${dollars(prices.annual.unit_amount, prices.annual.currency)} a year`
    : 'billed monthly';
}

/** "$192 a year": what `seats` members cost on an interval. */
export function total(prices: Prices, interval: Interval, seats: number): string {
  const p = price(prices, interval);
  return `${dollars(p.unit_amount * seats, p.currency)} ${interval === 'annual' ? 'a year' : 'a month'}`;
}

/** Whole months the annual price saves over twelve monthly ones. */
export function monthsFree(prices: Prices): number {
  const { monthly, annual } = prices;
  if (monthly.unit_amount <= 0) {
    return 0;
  }
  return Math.max(0, Math.floor(12 - annual.unit_amount / monthly.unit_amount));
}

const DAY = 86_400_000;

/** When the card has to be fixed by: 14 days after the first failure. */
export function dueBy(pastDueSince: string): string {
  return shortDate(new Date(new Date(pastDueSince).getTime() + 14 * DAY).toISOString());
}

export type BannerKind = 'past_due' | 'free';

/**
 * The one banner a team page shows, the most urgent first. Billing off has no
 * plan to talk about, so none.
 */
export function planBanner(input: {
  billing: boolean;
  summary: BillingSummary | null;
  /** Restricted collections the team still has. */
  restricted: number;
  /** Accepted members. */
  members: number;
}): BannerKind | null {
  const { billing, summary, restricted, members } = input;
  if (!billing || !summary) {
    return null;
  }
  if (summary.past_due_since) {
    return 'past_due';
  }
  if (summary.plan === 'free' && (restricted > 0 || members > 3)) {
    return 'free';
  }
  return null;
}
