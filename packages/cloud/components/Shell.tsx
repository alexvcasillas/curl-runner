import Link from 'next/link';
import { external, routes, type TeamTab } from '@/lib/routes';
import type { Info, Me } from '@/lib/types';
import { AccountMenu } from './AccountMenu';
import { ButtonLink } from './ButtonLink';
import { Tabs } from './Tabs';
import { TeamSwitcher } from './TeamSwitcher';

const SIGN_OUT_FORM = 'sign-out';

const TABS: { tab: TeamTab; label: string }[] = [
  { tab: 'collections', label: 'Collections' },
  { tab: 'tokens', label: 'Tokens' },
  { tab: 'members', label: 'Members' },
  { tab: 'billing', label: 'Billing' },
  { tab: 'settings', label: 'Settings' },
];

export type ShellProps = {
  /** `GET /v1/me`. */
  me: Me;
  /** `GET /v1/info`. Billing off hides the Billing tab and every plan pill. */
  info: Pick<Info, 'billing'>;
  /** The team being viewed. Null on pages that aren't about a team (Account, Shared with me). */
  team: string | null;
  /** The active team tab. */
  tab: TeamTab | null;
  /** A banner above the page: one at a time, the most urgent. */
  banner?: React.ReactNode;
  /** The admins' announcement, across the very top of the page. */
  announcement?: React.ReactNode;
  children: React.ReactNode;
};

/**
 * The signed-in layout: the mark, a slash and the team switcher, then Shared
 * with me, Docs and the account menu. Team pages get tabs underneath. On a phone
 * the tabs scroll sideways and the right side folds into a menu button.
 */
export function Shell({ me, info, team, tab, banner, announcement, children }: ShellProps) {
  const current = me.teams.find((t) => t.slug === team) ?? null;
  const tabs = current
    ? TABS.filter((t) => t.tab !== 'billing' || info.billing).map((t) => ({
        href: routes.teamTab(current.slug, t.tab),
        label: t.label,
        current: t.tab === tab,
      }))
    : null;

  return (
    <div className="flex min-h-dvh flex-col bg-bg">
      {announcement}
      <header className="border-b border-hairline bg-bg">
        <div className="flex min-h-14 flex-wrap items-center gap-2 px-4 max-sm:min-h-[52px] max-sm:flex-nowrap max-sm:px-3">
          <Link
            href={current ? routes.team(current.slug) : routes.home}
            aria-label="curl-runner Cloud home"
            className="grid size-8 shrink-0 place-items-center rounded-[6px]"
          >
            <span
              aria-hidden="true"
              className="grid size-6 place-items-center rounded-[6px] bg-primary font-mono text-[11px] font-medium text-on-primary"
            >
              &gt;_
            </span>
          </Link>
          <span
            aria-hidden="true"
            className="text-[22px] font-light text-control max-sm:text-[20px]"
          >
            /
          </span>
          <TeamSwitcher teams={me.teams} current={current} billing={info.billing} />
          <div className="flex-1" />
          <ButtonLink href={routes.shared} variant="ghost" className="max-sm:hidden">
            Shared with me
          </ButtonLink>
          <ButtonLink href={external.docs} external variant="ghost" className="max-sm:hidden">
            Docs
          </ButtonLink>
          <AccountMenu me={me} signOutForm={SIGN_OUT_FORM} className="max-sm:hidden" />
          <AccountMenu me={me} signOutForm={SIGN_OUT_FORM} variant="phone" className="sm:hidden" />
        </div>
        {tabs ? <Tabs label="Team" items={tabs} /> : null}
      </header>
      <form id={SIGN_OUT_FORM} action={routes.signOut} method="post" hidden />
      <main className="mx-auto box-border flex w-full max-w-[1200px] flex-1 flex-col gap-6 px-6 pt-8 pb-16 max-sm:px-4 max-sm:pt-5">
        {banner}
        {children}
      </main>
    </div>
  );
}
