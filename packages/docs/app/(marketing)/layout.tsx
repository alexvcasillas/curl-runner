import type { ReactNode } from 'react';
import { SiteFooter } from '@/components/site/SiteFooter';

export default function MarketingLayout({ children }: { children: ReactNode }) {
  return (
    <div className="flex min-h-screen flex-col bg-ground text-fg">
      <div className="grow">{children}</div>
      <SiteFooter />
    </div>
  );
}
