import { perMonth } from './billing';
import type { BillingSummary, Plan, Prices } from './types';

const plural = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;

/** The members table footer: what the seats mean on this plan. */
export function seatNote(plan: Plan, billing: boolean, members: number, pending: number): string {
  if (plan === 'free') {
    const who =
      pending > 0
        ? `${plural(members, 'member')} and ${plural(pending, 'pending invite')}`
        : plural(members, 'member');
    return `${who}: ${members + pending} of 3 on the Free plan.`;
  }
  return billing
    ? `${plural(members, 'member')} on the Team plan, billed per member. Customers are never seats.`
    : `${plural(members, 'member')}. Customers are never seats.`;
}

/**
 * What a new member adds to the bill, for the invite form. Only with billing on,
 * on the Team plan, with a subscription interval and the prices to quote.
 */
export function seatCostNote(
  billing: boolean,
  plan: Plan,
  prices: Prices | null,
  summary: BillingSummary | null,
): string | null {
  if (!billing || plan !== 'team' || !prices || !summary) {
    return null;
  }
  const { interval } = summary;
  if (interval !== 'monthly' && interval !== 'annual') {
    return null;
  }
  return `On the Team plan, each member who accepts adds ${perMonth(prices, interval)} a month on the ${interval} plan, prorated on the next invoice. Pending invites aren’t billed.`;
}
