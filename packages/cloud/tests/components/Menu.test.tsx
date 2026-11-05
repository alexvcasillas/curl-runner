import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useRef, useState } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { Dialog } from '@/components/Dialog';
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

  it('submits its form, by click and by Enter, before the menu unmounts', async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn((e: React.FormEvent) => e.preventDefault());
    render(
      <>
        <form id="sign-out" action="/sign-out" method="post" hidden onSubmit={onSubmit} />
        <Menu label="Account" trigger="Account">
          <MenuItem form="sign-out">Sign out</MenuItem>
        </Menu>
      </>,
    );
    await user.click(screen.getByRole('button', { name: 'Account' }));
    await user.click(screen.getByRole('menuitem', { name: 'Sign out' }));
    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole('menu')).toBeNull();

    await user.click(screen.getByRole('button', { name: 'Account' }));
    await user.keyboard('{Enter}');
    expect(onSubmit).toHaveBeenCalledTimes(2);
  });

  it('returns focus to the trigger after choosing an item', async () => {
    const { user } = setup();
    const trigger = screen.getByRole('button', { name: 'Open' });
    await user.click(trigger);
    await user.click(screen.getByRole('menuitem', { name: 'Two' }));
    expect(trigger).toHaveFocus();
  });

  it('keeps Escape from reaching a dialog around it', async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(
      <Dialog open onClose={onClose} title="Settings">
        <Menu label="Things" trigger="Open">
          <MenuItem>One</MenuItem>
        </Menu>
      </Dialog>,
    );
    await user.click(screen.getByRole('button', { name: 'Open' }));
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('menu')).toBeNull();
    expect(onClose).not.toHaveBeenCalled();
  });
});

describe('Dialog', () => {
  it('tells the parent when the browser closes it', () => {
    const onClose = vi.fn();
    render(<Dialog open onClose={onClose} title="Settings" />);
    const dialog = screen.getByRole('dialog', { name: 'Settings' });
    fireEvent(dialog, new Event('close'));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('opened from a menu item, returns focus to the menu trigger', async () => {
    function Harness() {
      const [open, setOpen] = useState(false);
      return (
        <>
          <Menu label="Row" trigger="Row actions">
            <MenuItem onSelect={() => setOpen(true)}>Delete</MenuItem>
          </Menu>
          <Dialog open={open} onClose={() => setOpen(false)} title="Delete it?">
            <button type="button">Cancel</button>
          </Dialog>
        </>
      );
    }
    const user = userEvent.setup();
    render(<Harness />);
    const trigger = screen.getByRole('button', { name: 'Row actions' });
    await user.click(trigger);
    await user.click(screen.getByRole('menuitem', { name: 'Delete' }));
    expect(screen.getByRole('button', { name: 'Cancel' })).toHaveFocus();
    await user.keyboard('{Escape}');
    expect(trigger).toHaveFocus();
  });

  it('falls back to returnFocusTo when the opener is gone', async () => {
    function Harness() {
      const [open, setOpen] = useState(false);
      const [shown, setShown] = useState(true);
      const fallback = useRef<HTMLButtonElement>(null);
      return (
        <>
          <button type="button" ref={fallback}>
            Fallback
          </button>
          {shown ? (
            <button
              type="button"
              onClick={() => {
                setOpen(true);
              }}
            >
              Opener
            </button>
          ) : null}
          <Dialog open={open} onClose={() => setOpen(false)} returnFocusTo={fallback} title="Gone">
            <button type="button" onClick={() => setShown(false)}>
              Remove opener
            </button>
          </Dialog>
        </>
      );
    }
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole('button', { name: 'Opener' }));
    await user.click(screen.getByRole('button', { name: 'Remove opener' }));
    await user.keyboard('{Escape}');
    expect(screen.getByRole('button', { name: 'Fallback' })).toHaveFocus();
  });
});
