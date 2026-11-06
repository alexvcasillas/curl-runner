'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { can, NAV_GROUPS, navActive, navPerm, type Perm } from '@/lib/admin';
import { cx } from '@/lib/cx';

/**
 * The admin sections. Every item shows for every admin, because every admin can
 * read; ones the admin can't edit say "Read only".
 */
export function AdminNav({
  permissions,
  version,
  onNavigate,
}: {
  permissions: Perm[];
  version: string;
  onNavigate?: () => void;
}) {
  const path = usePathname();
  return (
    <nav
      aria-label="Admin"
      className="box-border flex min-h-full flex-col p-3 lg:min-h-[calc(100dvh-57px)]"
    >
      {NAV_GROUPS.map((g) => (
        <div key={g.label}>
          <div className="px-2.5 pt-3.5 pb-1.5 text-[11px] font-medium tracking-[0.02em] text-fg-subtle uppercase">
            {g.label}
          </div>
          {g.items.map((item) => {
            const current = navActive(item, path);
            const perm = navPerm(item);
            const readOnly = perm !== null && !can(permissions, perm);
            return (
              <Link
                key={item.href}
                href={item.href}
                aria-current={current ? 'page' : undefined}
                onClick={onNavigate}
                className={cx(
                  'flex h-8 items-center gap-2.5 rounded-[6px] px-2.5 text-[13px] no-underline',
                  current
                    ? 'bg-surface-300 text-fg'
                    : 'text-fg-muted hover:bg-surface-100 hover:text-fg',
                )}
              >
                <svg
                  aria-hidden="true"
                  width="15"
                  height="15"
                  viewBox="0 0 16 16"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d={item.d} />
                </svg>
                <span className="flex-1">{item.label}</span>
                {readOnly ? (
                  <span
                    title="You can read this page but not change it."
                    className="text-[11px] text-fg-subtle"
                  >
                    Read only
                  </span>
                ) : null}
              </Link>
            );
          })}
        </div>
      ))}
      <div className="min-h-6 flex-1" />
      <div className="px-2.5 py-2 text-[12px] text-fg-subtle">curl-runner server {version}</div>
    </nav>
  );
}
