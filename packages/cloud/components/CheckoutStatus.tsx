'use client';

import { useRouter } from 'next/navigation';
import { useEffect, useRef, useState } from 'react';
import type { Plan } from '@/lib/types';
import { Button } from './Button';
import { Notice } from './Notice';
import { Spinner } from './Spinner';

const EVERY = 2000;
const GIVE_UP = 30_000;

/**
 * What the Billing page says after Checkout sends the person back. A redirect
 * alone proves nothing: the plan changes when Stripe's webhook lands, so this
 * refreshes every 2 seconds for up to 30 until the plan reads Team.
 */
export function CheckoutStatus({
  plan,
  checkout,
  teamName,
}: {
  plan: Plan;
  checkout: 'done' | 'cancelled' | null;
  teamName: string;
}) {
  const router = useRouter();
  const waiting = checkout === 'done' && plan !== 'team';
  const [slow, setSlow] = useState(false);
  // Held in a ref so a new router object never restarts the 30 seconds.
  const refresh = useRef(() => router.refresh());
  refresh.current = () => router.refresh();

  useEffect(() => {
    if (!waiting || slow) {
      return;
    }
    const poll = setInterval(() => refresh.current(), EVERY);
    const stop = setTimeout(() => {
      clearInterval(poll);
      setSlow(true);
    }, GIVE_UP);
    return () => {
      clearInterval(poll);
      clearTimeout(stop);
    };
  }, [waiting, slow]);

  if (checkout === 'done' && plan === 'team') {
    return (
      <Notice tone="ok" role="status" className="px-4 py-3 text-[13px] leading-5">
        Thanks. {teamName} is on the Team plan. Restricted sharing and unlimited members are on now.
      </Notice>
    );
  }
  if (waiting && !slow) {
    return (
      <Notice tone="info" role="status" className="px-4 py-3 text-[13px] leading-5">
        <span className="flex items-center gap-2.5">
          <Spinner />
          Finishing up with Stripe. This takes a few seconds, and the page updates on its own.
        </span>
      </Notice>
    );
  }
  if (waiting) {
    return (
      <Notice tone="info" role="status" className="px-4 py-3 text-[13px] leading-5">
        <span className="flex flex-wrap items-center gap-3">
          <span className="min-w-[200px] flex-1">
            Payment received. Stripe hasn’t confirmed it to us yet, so the plan updates in a minute.
            You don’t need to pay again.
          </span>
          <Button size="sm" onClick={() => router.refresh()}>
            Refresh
          </Button>
        </span>
      </Notice>
    );
  }
  if (checkout === 'cancelled' && plan === 'free') {
    return (
      <Notice tone="info" role="status" className="px-4 py-3 text-[13px] leading-5">
        Checkout cancelled. Nothing was charged. {teamName} is still on Free.
      </Notice>
    );
  }
  return null;
}
