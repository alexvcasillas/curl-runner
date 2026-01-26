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
    <PulsoraProvider config={{ apiToken: 'pub_xOBWAHbZhCDJvbzN0Yytltl2osCI2zAP' }}>
      <PageviewTracker />
      {children}
    </PulsoraProvider>
  );
}
