import { DocsLayout } from 'fumadocs-ui/layouts/docs';
import { RootProvider } from 'fumadocs-ui/provider/next';
import type { ReactNode } from 'react';
import { SiteHeader } from '@/components/site/SiteHeader';
import { source } from '@/lib/source';

export default function Layout({ children }: { children: ReactNode }) {
  return (
    // The root layout already forces the dark theme; Fumadocs only provides search here.
    <RootProvider theme={{ enabled: false }}>
      <div className="flex min-h-screen flex-col bg-ground text-fg">
        <SiteHeader active="docs" />
        <DocsLayout
          tree={source.getPageTree()}
          nav={{ title: 'Documentation', url: '/docs' }}
          themeSwitch={{ enabled: false }}
          sidebar={{ defaultOpenLevel: 1 }}
          containerProps={{ className: 'grow' }}
        >
          {children}
        </DocsLayout>
      </div>
    </RootProvider>
  );
}
