import Link from 'next/link';
import { adminRoutes } from '@/lib/admin';
import type { EmailsTab } from '@/lib/admin-emails';
import { cx } from '@/lib/cx';

const TABS: { tab: EmailsTab; label: string; href: string }[] = [
  { tab: 'broadcasts', label: 'Broadcasts', href: adminRoutes.emails },
  { tab: 'log', label: 'Delivery log', href: `${adminRoutes.emails}?tab=log` },
];

/** Broadcasts or the delivery log. Each is a page of its own, so the tabs are links. */
export function EmailsTabs({ tab }: { tab: EmailsTab }) {
  return (
    <div
      role="tablist"
      aria-label="Emails view"
      className="box-border inline-flex gap-0.5 rounded-[8px] border border-control p-[3px]"
    >
      {TABS.map((t) => {
        const on = t.tab === tab;
        return (
          <Link
            key={t.tab}
            role="tab"
            aria-selected={on}
            href={t.href}
            className={cx(
              'inline-flex h-[26px] items-center rounded-[5px] px-2.5 text-[12px] no-underline transition-colors duration-[120ms]',
              on ? 'bg-surface-300 text-fg' : 'text-fg-muted hover:text-fg',
            )}
          >
            {t.label}
          </Link>
        );
      })}
    </div>
  );
}
