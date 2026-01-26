'use client';

import { PulsoraProvider, usePageview } from '@pulsora/react';
import { usePathname } from 'next/navigation';

function PageviewTracker() {
  const pathname = usePathname();
  usePageview({ trigger: pathname });
  return null;
}

export function PulsoraAnalytics({ children }: { children: React.ReactNode }) {
  return (
    <PulsoraProvider config={{ apiToken: 'pub_X62LKER4NpQqrkZ8A7oIbJMFR1k6an0u' }}>
      <PageviewTracker />
      {children}
    </PulsoraProvider>
  );
}
