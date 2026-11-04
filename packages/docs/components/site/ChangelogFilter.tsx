'use client';

import { type ReactNode, useCallback, useEffect, useRef, useState } from 'react';

interface TagOption {
  value: string;
  label: string;
}

interface ChangelogFilterProps {
  tags: TagOption[];
  /** How many releases show at first, and how many each "Load older releases" adds. */
  pageSize: number;
  /** The server-rendered release `<article>`s, each with `data-release` and `data-tags`. */
  children: ReactNode;
}

interface View {
  tag: string | null;
  limit: number;
}

const FOCUS = 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand';

function releasesOf(root: HTMLElement): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>('[data-release]'));
}

function hasTag(release: HTMLElement, tag: string | null): boolean {
  return tag === null || (release.dataset.tags ?? '').split(' ').includes(tag);
}

/**
 * Filters and pages the server-rendered releases by toggling `hidden` on them,
 * their entries (`data-entry`) and item rows (`data-tag`). The page stays static:
 * the tag lives in `?tag=` and is written with `history.replaceState`. Without
 * JavaScript nothing is hidden and the chips are invisible.
 */
export function ChangelogFilter({ tags, pageSize, children }: ChangelogFilterProps) {
  const rootRef = useRef<HTMLDivElement>(null);
  const scrollTarget = useRef<HTMLElement | null>(null);
  // Null until the URL has been read after hydration, so the server HTML and the first render agree.
  const [view, setView] = useState<View | null>(null);
  const [matching, setMatching] = useState<number | null>(null);

  // Shows the release a `#version` hash points at, clearing a filter that hides it.
  const revealHash = useCallback(
    (current: View): View => {
      const root = rootRef.current;
      let id = window.location.hash.slice(1);
      try {
        id = decodeURIComponent(id);
      } catch {
        // A malformed escape: look the hash up as written.
      }
      const target = id ? document.getElementById(id) : null;
      const release = target?.closest<HTMLElement>('[data-release]');
      if (!root || !release || !root.contains(release)) {
        return current;
      }
      const tag = hasTag(release, current.tag) ? current.tag : null;
      const index = releasesOf(root)
        .filter((r) => hasTag(r, tag))
        .indexOf(release);
      if (tag === current.tag && index < current.limit) {
        return current;
      }
      scrollTarget.current = release;
      return { tag, limit: Math.max(current.limit, Math.ceil((index + 1) / pageSize) * pageSize) };
    },
    [pageSize],
  );

  useEffect(() => {
    const param = new URLSearchParams(window.location.search).get('tag');
    const tag = tags.some((t) => t.value === param) ? param : null;
    setView(revealHash({ tag, limit: pageSize }));
    const onHashChange = () =>
      setView((current) => revealHash(current ?? { tag: null, limit: pageSize }));
    window.addEventListener('hashchange', onHashChange);
    return () => window.removeEventListener('hashchange', onHashChange);
  }, [tags, pageSize, revealHash]);

  useEffect(() => {
    const root = rootRef.current;
    if (!root || !view) {
      return;
    }
    root.dataset.js = '';
    const { tag, limit } = view;
    const url = new URL(window.location.href);
    if (url.searchParams.get('tag') !== tag) {
      if (tag) {
        url.searchParams.set('tag', tag);
      } else {
        url.searchParams.delete('tag');
      }
      window.history.replaceState(window.history.state, '', url);
    }
    let count = 0;
    for (const release of releasesOf(root)) {
      for (const entry of release.querySelectorAll<HTMLElement>('[data-entry]')) {
        let kept = 0;
        for (const row of entry.querySelectorAll<HTMLElement>('[data-tag]')) {
          row.hidden = tag !== null && row.dataset.tag !== tag;
          kept += row.hidden ? 0 : 1;
        }
        entry.hidden = tag !== null && kept === 0;
      }
      const matches = hasTag(release, tag);
      release.hidden = !matches || count >= limit;
      count += matches ? 1 : 0;
    }
    setMatching(count);
    if (scrollTarget.current) {
      scrollTarget.current.scrollIntoView?.();
      scrollTarget.current = null;
    }
  }, [view]);

  const tag = view?.tag ?? null;
  const limit = view?.limit ?? pageSize;

  const options: { value: string | null; label: string }[] = [
    { value: null, label: 'All' },
    ...tags,
  ];

  return (
    <div ref={rootRef} data-changelog="" className="group/changelog">
      <div
        role="group"
        aria-label="Filter changes by tag"
        className="invisible flex flex-wrap gap-2 pb-14 group-data-js/changelog:visible"
      >
        {options.map((option) => (
          <button
            key={option.label}
            type="button"
            aria-pressed={tag === option.value}
            onClick={() => setView({ tag: option.value, limit: pageSize })}
            className={`h-8 cursor-pointer rounded-full border border-line px-3 text-[13px] text-fg-secondary transition-colors duration-[120ms] hover:bg-hover hover:text-fg aria-pressed:border-selected aria-pressed:bg-selected aria-pressed:text-fg ${FOCUS}`}
          >
            {option.label}
          </button>
        ))}
      </div>

      <section aria-label="Releases" className="border-t border-hairline">
        {children}
      </section>

      <div aria-live="polite">
        {matching === 0 ? (
          <p className="m-0 py-14 text-[15px] text-fg-secondary">No changes with this tag yet.</p>
        ) : null}
      </div>

      {matching !== null && matching > limit ? (
        <div className="flex justify-center pt-10">
          <button
            type="button"
            onClick={() => setView({ tag, limit: limit + pageSize })}
            className={`inline-flex h-10 cursor-pointer items-center rounded-[10px] border border-line px-[18px] text-sm text-fg transition-colors hover:bg-hover ${FOCUS}`}
          >
            Load older releases
          </button>
        </div>
      ) : null}
    </div>
  );
}
