import { DocsSidebar } from '@/components/docs-sidebar';
import { SiteHeader } from '@/components/site-header';

export default function DocsLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen bg-background">
      <SiteHeader />
      <div className="container flex-1 items-start md:grid md:grid-cols-[220px_minmax(0,1fr)] md:gap-6 lg:grid-cols-[240px_minmax(0,1fr)] lg:gap-10 px-4 sm:px-6 lg:px-8">
        <aside className="fixed top-14 z-30 -ml-2 hidden h-[calc(100vh-3.5rem)] w-full shrink-0 md:sticky md:block">
          <DocsSidebar className="h-full py-6 pr-6 lg:py-8" />
        </aside>
        <div className="w-full min-w-0">{children}</div>
      </div>
    </div>
  );
}
