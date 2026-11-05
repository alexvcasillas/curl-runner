import Link from 'next/link';
import { cx } from '@/lib/cx';
import { routes } from '@/lib/routes';

export type CollectionTab = 'versions' | 'inputs' | 'settings' | 'sharing';

const TABS: { id: CollectionTab; label: string }[] = [
  { id: 'versions', label: 'Versions' },
  { id: 'inputs', label: 'Inputs' },
  { id: 'settings', label: 'Settings' },
  { id: 'sharing', label: 'Sharing' },
];

function href(team: string, slug: string, tab: CollectionTab): string {
  if (tab === 'versions') {
    return routes.collection(team, slug);
  }
  return tab === 'sharing' ? routes.sharing(team, slug) : routes.collection(team, slug, tab);
}

/** The collection's sections. Sharing is its own page. */
export function CollectionTabs({
  team,
  slug,
  current,
}: {
  team: string;
  slug: string;
  current: CollectionTab;
}) {
  return (
    <nav
      aria-label="Collection"
      className="flex flex-wrap border-b border-hairline max-sm:flex-nowrap max-sm:overflow-x-auto"
    >
      {TABS.map((tab) => (
        <Link
          key={tab.id}
          href={href(team, slug, tab.id)}
          aria-current={tab.id === current ? 'page' : undefined}
          className={cx(
            '-mb-px mr-5 flex h-9 shrink-0 items-center border-b px-0.5 text-[13px] transition-colors duration-[120ms] max-sm:h-11',
            tab.id === current
              ? 'border-fg text-fg hover:text-fg'
              : 'border-transparent text-fg-muted hover:text-fg',
          )}
        >
          {tab.label}
        </Link>
      ))}
    </nav>
  );
}
