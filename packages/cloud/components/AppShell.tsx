'use client';

import { usePathname } from 'next/navigation';
import { parseLocation } from '@/lib/routes';
import type { Info, Me } from '@/lib/types';
import { Shell } from './Shell';

/**
 * The signed-in layout. A layout doesn't re-render when you move between its
 * pages, so the active team and tab come from the browser's path here.
 */
export function AppShell({
  me,
  info,
  children,
}: {
  me: Me;
  info: Pick<Info, 'billing'>;
  children: React.ReactNode;
}) {
  const { team, tab } = parseLocation(usePathname());
  return (
    <Shell me={me} info={info} team={team} tab={tab}>
      {children}
    </Shell>
  );
}
