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
 * While a tag is active, a folded item list opens (its toggle hidden) so every
 * matching row shows, and hides when none of its rows match. Back on All it
 * returns to the open state it had before filtering.
 */
function foldItems(more: HTMLDetailsElement, tag: string | null) {
  const summary = more.querySelector('summary');
  if (tag === null) {
    if (more.dataset.wasOpen !== undefined) {
      more.open = more.dataset.wasOpen === 'true';
      delete more.dataset.wasOpen;
    }
    more.hidden = false;
    delete more.dataset.filtering;
  } else {
    more.dataset.filtering = '';
    if (more.dataset.wasOpen === undefined) {
      more.dataset.wasOpen = String(more.open);
    }
    more.open = true;
    more.hidden = !more.querySelector('[data-tag]:not([hidden])');
  }
  if (summary) {
    summary.hidden = tag !== null;
  }
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
  // Index (among matching releases) of the first one a "Load older releases" click reveals.
  const focusFrom = useRef<number | null>(null);
  // Null until the URL has been read after hydration, so the server HTML and the first render agree.
  const [view, setView] = useState<View | null>(null);
  const viewRef = useRef<View | null>(null);
  viewRef.current = view;
  const [matching, setMatching] = useState<number | null>(null);
  const [status, setStatus] = useState('');

  /**
   * The view the URL asks for: a valid `?tag=` (else none), widened to show the
   * release a `#version` hash points at, with the filter cleared when it hides it.
   * `base` keeps the current page limit while the tag is unchanged.
   */
  const fromLocation = useCallback(
    (base: View | null): { view: View; target: HTMLElement | null } => {
      const param = new URLSearchParams(window.location.search).get('tag');
      let tag = tags.some((t) => t.value === param) ? param : null;
      let limit = base && base.tag === tag ? base.limit : pageSize;
      const root = rootRef.current;
      let id = window.location.hash.slice(1);
      try {
        id = decodeURIComponent(id);
      } catch {
        // A malformed escape: look the hash up as written.
      }
      const release = id
        ? document.getElementById(id)?.closest<HTMLElement>('[data-release]')
        : null;
      if (!root || !release || !root.contains(release)) {
        return { view: { tag, limit }, target: null };
      }
      if (!hasTag(release, tag)) {
        tag = null;
        limit = pageSize;
      }
      const index = releasesOf(root)
        .filter((r) => hasTag(r, tag))
        .indexOf(release);
      limit = Math.max(limit, Math.ceil((index + 1) / pageSize) * pageSize);
      return { view: { tag, limit }, target: release };
    },
    [tags, pageSize],
  );

  useEffect(() => {
    // On load the browser scrolled to the hash in the full list; hiding releases above it moves it, so always re-scroll.
    const initial = fromLocation(null);
    scrollTarget.current = initial.target;
    setView(initial.view);

    const onNavigate = () => {
      const current = viewRef.current;
      const next = fromLocation(current);
      if (current && next.view.tag === current.tag && next.view.limit === current.limit) {
        return;
      }
      scrollTarget.current = next.target;
      setView(next.view);
    };
    window.addEventListener('hashchange', onNavigate);
    window.addEventListener('popstate', onNavigate);
    return () => {
      window.removeEventListener('hashchange', onNavigate);
      window.removeEventListener('popstate', onNavigate);
    };
  }, [fromLocation]);

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
    const shown: HTMLElement[] = [];
    let count = 0;
    for (const release of releasesOf(root)) {
      for (const entry of release.querySelectorAll<HTMLElement>('[data-entry]')) {
        let kept = 0;
        for (const row of entry.querySelectorAll<HTMLElement>('[data-tag]')) {
          row.hidden = tag !== null && row.dataset.tag !== tag;
          kept += row.hidden ? 0 : 1;
        }
        entry.hidden = tag !== null && kept === 0;
        for (const more of entry.querySelectorAll<HTMLDetailsElement>('details[data-more-items]')) {
          foldItems(more, tag);
        }
      }
      const matches = hasTag(release, tag);
      release.hidden = !matches || count >= limit;
      if (!release.hidden) {
        shown.push(release);
      }
      count += matches ? 1 : 0;
    }
    setMatching(count);
    if (scrollTarget.current) {
      scrollTarget.current.scrollIntoView?.();
      scrollTarget.current = null;
    }
    if (focusFrom.current !== null) {
      const first = shown[focusFrom.current];
      const link =
        first?.querySelector<HTMLElement>('h2 a') ??
        document.querySelector<HTMLElement>('[data-changelog-github]');
      link?.focus();
      focusFrom.current = null;
    }
  }, [view]);

  const tag = view?.tag ?? null;
  const limit = view?.limit ?? pageSize;

  function loadOlder() {
    const next = limit + pageSize;
    focusFrom.current = limit;
    setStatus(`Showing ${Math.min(next, matching ?? next)} releases`);
    setView({ tag, limit: next });
  }

  function selectTag(value: string | null) {
    setStatus('');
    setView({ tag: value, limit: pageSize });
  }

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
            onClick={() => selectTag(option.value)}
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
        {status ? <span className="sr-only">{status}</span> : null}
      </div>

      {matching !== null && matching > limit ? (
        <div className="flex justify-center pt-10">
          <button
            type="button"
            onClick={loadOlder}
            className={`inline-flex h-10 cursor-pointer items-center rounded-[10px] border border-line px-[18px] text-sm text-fg transition-colors hover:bg-hover ${FOCUS}`}
          >
            Load older releases
          </button>
        </div>
      ) : null}
    </div>
  );
}
