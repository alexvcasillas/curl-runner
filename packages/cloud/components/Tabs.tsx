import Link from 'next/link';
import { cx } from '@/lib/cx';

export type TabItem = { href: string; label: string; current?: boolean };

export type TabsProps = {
  /** Names the navigation: "Team", "Collection". */
  label: string;
  items: TabItem[];
  className?: string;
};

/** Navigation tabs under a header. They scroll sideways on a phone. */
export function Tabs({ label, items, className }: TabsProps) {
  return (
    <nav
      aria-label={label}
      className={cx(
        'flex gap-0.5 overflow-x-auto px-3 [scrollbar-width:none] max-sm:px-1 [&::-webkit-scrollbar]:hidden',
        className,
      )}
    >
      {items.map((tab) => (
        <Link
          key={tab.href}
          href={tab.href}
          aria-current={tab.current ? 'page' : undefined}
          className={cx(
            '-mb-px flex h-[46px] shrink-0 items-center border-b px-3 text-[13px] whitespace-nowrap transition-colors duration-[120ms] max-sm:h-11',
            'focus-visible:shadow-[inset_0_0_0_2px_#52a8ff]',
            tab.current
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
