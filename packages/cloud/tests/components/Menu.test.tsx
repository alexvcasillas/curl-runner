import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Menu, MenuItem } from '@/components/Menu';

function setup() {
  const onSelect = vi.fn();
  render(
    <>
      <Menu label="Things" trigger="Open">
        <MenuItem onSelect={onSelect}>One</MenuItem>
        <MenuItem>Two</MenuItem>
        <MenuItem>Three</MenuItem>
      </Menu>
      <button type="button">Outside</button>
    </>,
  );
  return { onSelect, user: userEvent.setup() };
}

describe('Menu', () => {
  it('opens on the first item and moves with arrows, Home and End', async () => {
    const { user } = setup();
    const trigger = screen.getByRole('button', { name: 'Open' });
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
    trigger.focus();
    await user.keyboard('{Enter}');
    expect(trigger).toHaveAttribute('aria-expanded', 'true');
    const [one, two, three] = screen.getAllByRole('menuitem');
    expect(one).toHaveFocus();
    await user.keyboard('{ArrowDown}');
    expect(two).toHaveFocus();
    await user.keyboard('{End}');
    expect(three).toHaveFocus();
    await user.keyboard('{ArrowDown}');
    expect(one).toHaveFocus();
    await user.keyboard('{ArrowUp}');
    expect(three).toHaveFocus();
    await user.keyboard('{Home}');
    expect(one).toHaveFocus();
  });

  it('opens on the last item with ArrowUp', async () => {
    const { user } = setup();
    screen.getByRole('button', { name: 'Open' }).focus();
    await user.keyboard('{ArrowUp}');
    expect(screen.getByRole('menuitem', { name: 'Three' })).toHaveFocus();
  });

  it('closes on Escape and returns focus to the trigger', async () => {
    const { user } = setup();
    const trigger = screen.getByRole('button', { name: 'Open' });
    await user.click(trigger);
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('menu')).toBeNull();
    expect(trigger).toHaveFocus();
  });

  it('runs an item and closes', async () => {
    const { user, onSelect } = setup();
    await user.click(screen.getByRole('button', { name: 'Open' }));
    await user.keyboard('{Enter}');
    expect(onSelect).toHaveBeenCalledOnce();
    expect(screen.queryByRole('menu')).toBeNull();
  });

  it('closes on a click outside', async () => {
    const { user } = setup();
    await user.click(screen.getByRole('button', { name: 'Open' }));
    await user.click(screen.getByRole('button', { name: 'Outside' }));
    expect(screen.queryByRole('menu')).toBeNull();
  });
});
