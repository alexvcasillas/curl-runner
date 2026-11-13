'use client';

import { SupapulseProvider } from '@supapulse/react';

const config = { apiToken: 'pub_X62LKER4NpQqrkZ8A7oIbJMFR1k6an0u' };

export function PulsoraAnalytics({ children }: { children: React.ReactNode }) {
  return <SupapulseProvider config={config}>{children}</SupapulseProvider>;
}
