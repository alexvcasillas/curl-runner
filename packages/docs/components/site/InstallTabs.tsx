'use client';

import { type KeyboardEvent, useId, useRef, useState } from 'react';
import { CopyButton } from '@/components/site/CopyButton';
import { INSTALL_METHODS, type InstallMethodId } from '@/lib/install-methods';

interface InstallTabsProps {
  methods: InstallMethodId[];
  defaultId: InstallMethodId;
  showNote: boolean;
}

export function InstallTabs({ methods, defaultId, showNote }: InstallTabsProps) {
  const items = methods
    .map((id) => INSTALL_METHODS.find((m) => m.id === id))
    .filter((m) => m !== undefined);
  const [selectedId, setSelectedId] = useState<InstallMethodId>(
    items.some((m) => m.id === defaultId) ? defaultId : (items[0]?.id ?? defaultId),
  );
  const tabRefs = useRef<(HTMLButtonElement | null)[]>([]);
  const baseId = useId();
  const selected = items.find((m) => m.id === selectedId) ?? items[0];
  if (!selected) {
    return null;
  }

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const index = items.findIndex((m) => m.id === selectedId);
    let next: number;
    switch (event.key) {
      case 'ArrowRight':
        next = (index + 1) % items.length;
        break;
      case 'ArrowLeft':
        next = (index - 1 + items.length) % items.length;
        break;
      case 'Home':
        next = 0;
        break;
      case 'End':
        next = items.length - 1;
        break;
      default:
        return;
    }
    event.preventDefault();
    setSelectedId(items[next].id);
    tabRefs.current[next]?.focus();
  }

  const panelId = `${baseId}-panel`;

  return (
    <div
      className={`overflow-hidden border border-line bg-surface text-left ${showNote ? 'rounded-[14px]' : 'rounded-xl'}`}
    >
      <div
        role="tablist"
        aria-label="Install method"
        onKeyDown={onKeyDown}
        className="flex flex-wrap gap-1 border-b border-hairline p-1.5"
      >
        {items.map((m, i) => {
          const isSelected = m.id === selected.id;
          return (
            <button
              key={m.id}
              ref={(el) => {
                tabRefs.current[i] = el;
              }}
              type="button"
              role="tab"
              id={`${baseId}-tab-${m.id}`}
              aria-selected={isSelected}
              aria-controls={panelId}
              tabIndex={isSelected ? 0 : -1}
              onClick={() => setSelectedId(m.id)}
              className={`relative h-9 cursor-pointer rounded-md px-3 text-[13px] transition-colors before:absolute before:inset-x-0 before:-inset-y-1 before:content-[''] focus-visible:outline-2 focus-visible:outline-brand ${
                isSelected
                  ? 'bg-selected text-fg'
                  : 'text-fg-secondary hover:bg-hover hover:text-fg'
              }`}
            >
              {m.label}
            </button>
          );
        })}
      </div>
      <div id={panelId} role="tabpanel" aria-labelledby={`${baseId}-tab-${selected.id}`}>
        <div
          className={`flex items-center gap-3 font-mono ${showNote ? 'px-5 py-[18px] text-[15px]' : 'px-[18px] py-4 text-sm'}`}
        >
          <span aria-hidden="true" className="select-none text-fg-tertiary">
            $
          </span>
          <code className="grow overflow-x-auto whitespace-nowrap text-fg">{selected.command}</code>
          <span className="-my-3 -mr-2.5">
            <CopyButton text={selected.command} label="Copy install command" />
          </span>
        </div>
        {showNote ? (
          <p className="px-5 pb-[18px] text-sm text-fg-tertiary">{selected.note}</p>
        ) : null}
      </div>
    </div>
  );
}
