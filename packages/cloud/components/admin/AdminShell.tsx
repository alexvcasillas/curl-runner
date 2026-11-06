'use client';

import Link from 'next/link';
import { useEffect, useState } from 'react';
import { type AdminMe, adminRoutes } from '@/lib/admin';
import type { Me } from '@/lib/types';
import { Avatar } from '../Avatar';
import { ButtonLink } from '../ButtonLink';
import { MenuIcon } from '../icons';
import { Pill } from '../Pill';
import { AdminNav } from './AdminNav';
import { CommandSearch } from './CommandSearch';

/**
 * The admin layout: a header with search, a section nav and the page. It's built
 * for 1024px and up; below that the nav folds behind a Menu button, so nothing
 * scrolls sideways.
 */
export function AdminShell({
  me,
  admin,
  version,
  children,
}: {
  me: Me;
  admin: AdminMe;
  version: string;
  children: React.ReactNode;
}) {
  const [navOpen, setNavOpen] = useState(false);
  useEffect(() => {
    if (!navOpen) {
      return;
    }
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && setNavOpen(false);
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [navOpen]);

  return (
    <div className="min-h-dvh bg-bg">
      <header className="flex min-h-14 flex-wrap items-center gap-2 border-b border-hairline px-4 max-sm:px-3">
        <button
          type="button"
          aria-label="Sections"
          aria-expanded={navOpen}
          aria-controls="admin-nav"
          onClick={() => setNavOpen((o) => !o)}
          className="grid size-8 shrink-0 cursor-pointer place-items-center rounded-[6px] border-0 bg-transparent text-fg lg:hidden"
        >
          <MenuIcon size={18} />
        </button>
        <Link
          href={adminRoutes.overview}
          aria-label="Admin home"
          className="grid size-8 shrink-0 place-items-center rounded-[6px]"
        >
          <span
            aria-hidden="true"
            className="grid size-6 place-items-center rounded-[6px] bg-primary font-mono text-[11px] font-medium text-on-primary"
          >
            &gt;_
          </span>
        </Link>
        <span aria-hidden="true" className="text-[22px] font-light text-control">
          /
        </span>
        <span className="font-medium">Admin</span>
        <Pill size="sm">{admin.role === 'owner' ? 'Owner' : 'Admin'}</Pill>
        <div className="flex-1" />
        <CommandSearch />
        <ButtonLink href="/" variant="ghost" className="max-sm:hidden">
          Back to the dashboard
        </ButtonLink>
        <Avatar name={me.user.name || me.user.email} size={32} className="max-sm:hidden" />
      </header>
      <div className="relative flex">
        <div
          id="admin-nav"
          className={`relative box-border w-[232px] shrink-0 border-r border-hairline max-lg:absolute max-lg:z-10 max-lg:bg-bg max-lg:shadow-menu lg:block ${navOpen ? 'max-lg:block' : 'max-lg:hidden'}`}
        >
          <AdminNav
            permissions={admin.permissions}
            version={version}
            onNavigate={() => setNavOpen(false)}
          />
          <div className="border-t border-hairline p-3 sm:hidden">
            <ButtonLink href="/" variant="ghost">
              Back to the dashboard
            </ButtonLink>
          </div>
        </div>
        <main className="box-border flex min-w-0 max-w-[1200px] flex-1 flex-col gap-6 p-8 max-lg:p-4">
          {children}
        </main>
      </div>
    </div>
  );
}
