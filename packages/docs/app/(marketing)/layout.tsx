import type { ReactNode } from 'react';
import { SiteFooter } from '@/components/site/SiteFooter';
import { SiteHeader } from '@/components/site/SiteHeader';

export default function MarketingLayout({ children }: { children: ReactNode }) {
  return (
    <div className="flex min-h-screen flex-col bg-ground text-fg">
      <SiteHeader />
      <main className="grow">{children}</main>
      <SiteFooter />
    </div>
  );
}
