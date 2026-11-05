'use client';

import Link from 'next/link';
import { createContext, useCallback, useContext, useEffect, useId, useRef, useState } from 'react';
import { cx } from '@/lib/cx';
import { MenuCheckIcon } from './icons';

const ITEM_SELECTOR =
  '[role="menuitem"]:not([aria-disabled="true"]),[role="menuitemradio"]:not([aria-disabled="true"])';

const MenuContext = createContext<{ close: (refocus: boolean) => void } | null>(null);

export type MenuProps = {
  /** Names the menu: "Teams", "Account". */
  label: string;
  /** The trigger button's contents. */
  trigger: React.ReactNode;
  /** The trigger's accessible name, when its contents don't say it. */
  triggerLabel?: string;
  /** Classes for the trigger, or a function of whether the menu is open. */
  triggerClassName?: string | ((open: boolean) => string);
  /** Which edge of the trigger the menu lines up with. */
  align?: 'start' | 'end';
  /** Menu width in px. Never wider than the screen less 24px. */
  width?: number;
  children: React.ReactNode;
  className?: string;
};

/**
 * A menu button. Enter, Space or ↓ opens it on the first item, ↑ on the last;
 * arrows, Home and End move; Escape closes and returns focus to the trigger;
 * Tab or a click outside closes it.
 */
export function Menu({
  label,
  trigger,
  triggerLabel,
  triggerClassName,
  align = 'start',
  width = 260,
  children,
  className,
}: MenuProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const focusOnOpen = useRef<'first' | 'last'>('first');
  const menuId = useId();

  const items = useCallback(
    () => Array.from(menuRef.current?.querySelectorAll<HTMLElement>(ITEM_SELECTOR) ?? []),
    [],
  );

  const close = useCallback((refocus: boolean) => {
    setOpen(false);
    if (refocus) {
      triggerRef.current?.focus();
    }
  }, []);

  useEffect(() => {
    if (!open) {
      return;
    }
    const list = items();
    (focusOnOpen.current === 'last' ? list[list.length - 1] : list[0])?.focus();
    function onPointerDown(e: PointerEvent) {
      if (!rootRef.current?.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener('pointerdown', onPointerDown);
    return () => document.removeEventListener('pointerdown', onPointerDown);
  }, [open, items]);

  function openAt(where: 'first' | 'last') {
    focusOnOpen.current = where;
    setOpen(true);
  }

  function onTriggerKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      openAt(e.key === 'ArrowUp' ? 'last' : 'first');
    }
  }

  function onMenuKeyDown(e: React.KeyboardEvent) {
    const list = items();
    const index = list.indexOf(document.activeElement as HTMLElement);
    let next: number | null = null;
    switch (e.key) {
      case 'ArrowDown':
        next = index < 0 || index === list.length - 1 ? 0 : index + 1;
        break;
      case 'ArrowUp':
        next = index <= 0 ? list.length - 1 : index - 1;
        break;
      case 'Home':
        next = 0;
        break;
      case 'End':
        next = list.length - 1;
        break;
      case 'Escape':
        // Stop here, so a menu inside a dialog doesn't close the dialog too.
        e.preventDefault();
        e.stopPropagation();
        close(true);
        return;
      case 'Tab':
        setOpen(false);
        return;
      default:
        return;
    }
    e.preventDefault();
    list[next]?.focus();
  }

  return (
    <div ref={rootRef} className={cx('relative inline-flex', className)}>
      <button
        ref={triggerRef}
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        aria-label={triggerLabel}
        onClick={() => (open ? setOpen(false) : openAt('first'))}
        onKeyDown={onTriggerKeyDown}
        className={
          typeof triggerClassName === 'function' ? triggerClassName(open) : triggerClassName
        }
      >
        {trigger}
      </button>
      {open ? (
        <MenuContext.Provider value={{ close }}>
          <div
            ref={menuRef}
            id={menuId}
            role="menu"
            aria-label={label}
            tabIndex={-1}
            onKeyDown={onMenuKeyDown}
            style={{ width: `min(${width}px, calc(100vw - 24px))` }}
            className={cx(
              'absolute top-full z-50 mt-1 box-border animate-rise rounded-[8px] border border-control bg-raised p-1 shadow-menu',
              align === 'start' ? 'left-0' : 'right-0',
            )}
          >
            {children}
          </div>
        </MenuContext.Provider>
      ) : null}
    </div>
  );
}

const ITEM_CLASSES =
  'box-border flex min-h-8 w-full cursor-pointer items-center gap-2.5 rounded-[6px] border-0 bg-transparent px-2 text-left text-[13px] text-fg no-underline hover:bg-surface-300 hover:text-fg focus-visible:bg-surface-300';

export type MenuItemProps = {
  children: React.ReactNode;
  icon?: React.ReactNode;
  /** Trailing content: a plan pill, a hint. */
  end?: React.ReactNode;
  /** Makes it a `menuitemradio`, with a tick when checked. */
  checked?: boolean;
  /** An in-app link. */
  href?: string;
  /** An off-site link, opened in a new tab. */
  external?: boolean;
  onSelect?: () => void;
  /** Submits this form (by id) when chosen, like sign out. */
  form?: string;
};

/** One choice in a `Menu`. A link, a button, or a form's submit. */
export function MenuItem({
  children,
  icon,
  end,
  checked,
  href,
  external,
  onSelect,
  form,
}: MenuItemProps) {
  const ctx = useContext(MenuContext);
  const radio = checked !== undefined;
  const common = {
    role: radio ? 'menuitemradio' : 'menuitem',
    'aria-checked': radio ? checked : undefined,
    tabIndex: -1,
    className: cx(ITEM_CLASSES, checked && 'bg-surface-300'),
    onClick: (e: React.MouseEvent) => {
      if (form) {
        // Submit before the menu unmounts: a detached button has no form to submit.
        e.preventDefault();
        (document.getElementById(form) as HTMLFormElement | null)?.requestSubmit();
      }
      onSelect?.();
      // Focus goes back to the trigger, so a dialog opened from here returns focus there.
      ctx?.close(true);
    },
  } as const;
  const content = (
    <>
      {icon}
      <span className="min-w-0 flex-1 truncate">{children}</span>
      {end}
      {radio ? (
        checked ? (
          <MenuCheckIcon />
        ) : (
          <span aria-hidden="true" className="w-[13px] shrink-0" />
        )
      ) : null}
    </>
  );
  if (href && external) {
    return (
      <a href={href} target="_blank" rel="noreferrer" {...common}>
        {content}
      </a>
    );
  }
  if (href) {
    return (
      <Link href={href} {...common}>
        {content}
      </Link>
    );
  }
  return (
    <button type="button" {...common}>
      {content}
    </button>
  );
}

/** A group heading inside a menu: "Teams". */
export function MenuLabel({ children }: { children: React.ReactNode }) {
  // Presentational: a menu holds only items, groups and separators.
  return (
    <div
      role="none"
      className="px-2 pt-2 pb-1 text-[11px] font-medium tracking-[0.02em] text-fg-subtle uppercase"
    >
      {children}
    </div>
  );
}

/** Who's signed in, at the top of the account menu. */
export function MenuHeader({ title, subtitle }: { title: string; subtitle?: string }) {
  return (
    <div role="none" className="flex flex-col px-2 pt-2 pb-2.5">
      <span className="truncate font-medium">{title}</span>
      {subtitle ? <span className="truncate text-[12px] text-fg-muted">{subtitle}</span> : null}
    </div>
  );
}

/** Items that belong together, named for screen readers: the teams. */
export function MenuGroup({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div role="group" aria-label={label}>
      {children}
    </div>
  );
}

/** `<hr>` has the implicit `separator` role. */
export function MenuSeparator() {
  return <hr className="-mx-1 my-1 h-px border-0 bg-control" />;
}
