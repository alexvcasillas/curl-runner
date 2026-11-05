'use client';

import { cx } from '@/lib/cx';
import { external, routes } from '@/lib/routes';
import type { Me } from '@/lib/types';
import { Avatar } from './Avatar';
import { MenuIcon, ShieldIcon } from './icons';
import { Menu, MenuHeader, MenuItem, MenuSeparator } from './Menu';

export type AccountMenuProps = {
  me: Me;
  /** The id of the sign-out form, which posts to `/sign-out`. */
  signOutForm: string;
  /**
   * `avatar`: the round initials on wide screens.
   * `phone`: a menu button that also holds Shared with me and Docs.
   */
  variant?: 'avatar' | 'phone';
  className?: string;
};

/** The signed-in person's menu: account, docs, the studio, admin and sign out. */
export function AccountMenu({ me, signOutForm, variant = 'avatar', className }: AccountMenuProps) {
  const who = me.user.name || me.user.email;
  const phone = variant === 'phone';
  return (
    <Menu
      label="Account"
      align="end"
      width={260}
      className={className}
      triggerLabel={phone ? 'Open menu' : `Account menu for ${me.user.email}`}
      triggerClassName={(open) =>
        phone
          ? cx(
              'grid size-11 cursor-pointer place-items-center rounded-[6px] border-0 bg-transparent text-fg',
              open && 'bg-surface-300',
            )
          : cx(
              'cursor-pointer rounded-full border-0 bg-transparent p-0',
              open && 'shadow-[0_0_0_2px_#2e2e2e]',
            )
      }
      trigger={phone ? <MenuIcon size={18} /> : <Avatar name={who} size={32} />}
    >
      <MenuHeader title={who} subtitle={me.user.name ? me.user.email : undefined} />
      <MenuSeparator />
      <MenuItem href={routes.account}>Account</MenuItem>
      <MenuItem href={routes.shared}>Shared with me</MenuItem>
      <MenuItem href={external.docs} external>
        Docs
      </MenuItem>
      <MenuItem href={external.download} external>
        Download the studio
      </MenuItem>
      {me.admin ? (
        <>
          <MenuSeparator />
          <MenuItem href={routes.admin} icon={<ShieldIcon />}>
            Admin
          </MenuItem>
        </>
      ) : null}
      <MenuSeparator />
      <MenuItem form={signOutForm}>Sign out</MenuItem>
    </Menu>
  );
}
