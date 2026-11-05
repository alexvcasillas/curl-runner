import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { Shell } from '@/components/Shell';
import { info, me } from '../fixtures';

function tabs() {
  return within(screen.getByRole('navigation', { name: 'Team' }));
}

describe('Shell', () => {
  it('renders the team tabs and marks the active one', () => {
    render(
      <Shell me={me} info={info} team="acme" tab="members">
        <p>page</p>
      </Shell>,
    );
    const links = tabs().getAllByRole('link');
    expect(links.map((l) => l.textContent)).toEqual([
      'Collections',
      'Tokens',
      'Members',
      'Billing',
      'Settings',
    ]);
    expect(links.map((l) => l.getAttribute('href'))).toEqual([
      '/t/acme',
      '/t/acme/tokens',
      '/t/acme/members',
      '/t/acme/billing',
      '/t/acme/settings',
    ]);
    expect(tabs().getByRole('link', { name: 'Members' })).toHaveAttribute('aria-current', 'page');
    expect(tabs().getByRole('link', { name: 'Tokens' })).not.toHaveAttribute('aria-current');
    expect(screen.getByRole('main')).toHaveTextContent('page');
  });

  it('hides Billing and the plan pill when billing is off', () => {
    render(
      <Shell me={me} info={{ ...info, billing: false }} team="acme" tab="collections">
        <p>page</p>
      </Shell>,
    );
    expect(tabs().queryByRole('link', { name: 'Billing' })).toBeNull();
    expect(tabs().getAllByRole('link')).toHaveLength(4);
    expect(screen.queryByText('Team')).toBeNull();
  });

  it('shows the current team and its plan in the switcher', () => {
    render(
      <Shell me={me} info={info} team="acme" tab="collections">
        <p>page</p>
      </Shell>,
    );
    const trigger = screen.getByRole('button', { name: 'Switch team, current team Acme' });
    expect(trigger).toHaveTextContent('Team');
  });

  it('has no tabs off a team page', () => {
    render(
      <Shell me={me} info={info} team={null} tab={null}>
        <p>page</p>
      </Shell>,
    );
    expect(screen.queryByRole('navigation', { name: 'Team' })).toBeNull();
  });

  it('lists the teams in the switcher, the current one checked', async () => {
    const user = userEvent.setup();
    render(
      <Shell me={me} info={info} team="acme" tab="collections">
        <p>page</p>
      </Shell>,
    );
    await user.click(screen.getByRole('button', { name: 'Switch team, current team Acme' }));
    const menu = screen.getByRole('menu', { name: 'Teams' });
    const radios = within(menu).getAllByRole('menuitemradio');
    expect(radios.map((r) => r.getAttribute('aria-checked'))).toEqual(['true', 'false']);
    expect(radios[1]).toHaveAttribute('href', '/t/globex');
    expect(within(menu).getByRole('menuitem', { name: 'New team' })).toHaveAttribute(
      'href',
      '/new',
    );
    expect(radios[0]).toHaveFocus();
  });

  it('shows Admin in the account menu only for admins', async () => {
    const user = userEvent.setup();
    const { unmount } = render(
      <Shell me={me} info={info} team="acme" tab="collections">
        <p>page</p>
      </Shell>,
    );
    await user.click(screen.getByRole('button', { name: 'Account menu for ada@acme.com' }));
    const menu = screen.getByRole('menu', { name: 'Account' });
    expect(within(menu).queryByRole('menuitem', { name: 'Admin' })).toBeNull();
    expect(within(menu).getByRole('menuitem', { name: 'Sign out' })).toHaveAttribute(
      'type',
      'submit',
    );
    unmount();

    render(
      <Shell me={{ ...me, admin: true }} info={info} team="acme" tab="collections">
        <p>page</p>
      </Shell>,
    );
    await user.click(screen.getByRole('button', { name: 'Account menu for ada@acme.com' }));
    expect(screen.getByRole('menuitem', { name: 'Admin' })).toHaveAttribute('href', '/admin');
  });
});
