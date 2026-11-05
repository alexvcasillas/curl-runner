'use client';

import { cx } from '@/lib/cx';
import { routes } from '@/lib/routes';
import type { MeTeam } from '@/lib/types';
import { Avatar } from './Avatar';
import { PlanBadge } from './Badge';
import { ChevronsUpDownIcon, FolderIcon, PlusIcon } from './icons';
import { Menu, MenuGroup, MenuItem, MenuLabel, MenuSeparator } from './Menu';

export type TeamSwitcherProps = {
  teams: MeTeam[];
  /** The team being viewed, if any. */
  current: MeTeam | null;
  /** Plan pills show only when the server has billing. */
  billing: boolean;
};

/** The team name and plan after the mark. Opens the list of teams. */
export function TeamSwitcher({ teams, current, billing }: TeamSwitcherProps) {
  return (
    <Menu
      label="Teams"
      width={300}
      triggerLabel={current ? `Switch team, current team ${current.name}` : 'Switch team'}
      triggerClassName={(open) =>
        cx(
          'flex h-8 min-w-0 cursor-pointer items-center gap-2 rounded-[6px] border-0 px-2 text-fg transition-colors duration-[120ms] hover:bg-surface-200 max-sm:h-11 max-sm:gap-1.5 max-sm:px-1.5',
          open && 'bg-surface-300 hover:bg-surface-300',
        )
      }
      trigger={
        <>
          {current ? (
            <>
              <Avatar kind="team" name={current.name} size={20} className="max-sm:hidden" />
              <span className="truncate font-medium">{current.name}</span>
              {billing ? (
                <span className="max-sm:hidden">
                  <PlanBadge plan={current.plan} />
                </span>
              ) : null}
            </>
          ) : (
            <span className="truncate text-fg-muted">Teams</span>
          )}
          <ChevronsUpDownIcon className="shrink-0 text-fg-muted" />
        </>
      }
    >
      {teams.length > 0 ? <MenuLabel>Teams</MenuLabel> : null}
      <MenuGroup label="Teams">
        {teams.map((t) => {
          const on = t.slug === current?.slug;
          return (
            <MenuItem
              key={t.slug}
              href={routes.team(t.slug)}
              checked={on}
              icon={<Avatar kind="team" name={t.name} size={22} active={on} />}
              end={billing ? <PlanBadge plan={t.plan} /> : undefined}
            >
              {t.name}
            </MenuItem>
          );
        })}
      </MenuGroup>
      {teams.length > 0 ? <MenuSeparator /> : null}
      <MenuItem href={routes.shared} icon={<FolderIcon />}>
        Shared with me
      </MenuItem>
      <MenuItem href={routes.newTeam} icon={<PlusIcon />}>
        New team
      </MenuItem>
    </Menu>
  );
}
