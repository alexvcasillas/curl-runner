'use client';

import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useEffect, useId, useMemo, useState } from 'react';
import { adminRoutes, type SearchResults } from '@/lib/admin';
import { cx } from '@/lib/cx';
import { Dialog } from '../Dialog';
import { SearchIcon } from '../icons';

const DEBOUNCE_MS = 200;

type Row = { key: string; href: string; primary: string; secondary?: string };
type Group = { label: string; rows: Row[] };

function toGroups(r: SearchResults): Group[] {
  const groups: Group[] = [
    {
      label: 'Users',
      rows: r.users.map((u) => ({
        key: `user-${u.id}`,
        href: adminRoutes.user(u.id),
        primary: u.email,
        secondary: u.name ?? undefined,
      })),
    },
    {
      label: 'Teams',
      rows: r.teams.map((t) => ({
        key: `team-${t.slug}`,
        href: adminRoutes.team(t.slug),
        primary: t.name,
        secondary: t.slug,
      })),
    },
    {
      label: 'Collections',
      rows: r.collections.map((c) => ({
        key: `collection-${c.team}-${c.slug}`,
        href: adminRoutes.collectionsSearch(c.slug),
        primary: c.name,
        secondary: `${c.team}/${c.slug}`,
      })),
    },
  ];
  return groups.filter((g) => g.rows.length > 0);
}

type State =
  | { kind: 'idle' }
  | { kind: 'loading' }
  | { kind: 'done'; query: string; groups: Group[] }
  | { kind: 'error' };

/** The search box in the header and the ⌘K dialog behind it. */
export function CommandSearch() {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [state, setState] = useState<State>({ kind: 'idle' });
  const [active, setActive] = useState(0);
  const [pending, setPending] = useState(false);
  const listId = useId();

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setOpen(true);
      }
    }
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  const term = query.trim();
  useEffect(() => {
    if (!open || !term) {
      setPending(false);
      setState({ kind: 'idle' });
      return;
    }
    const ctrl = new AbortController();
    const timer = setTimeout(async () => {
      // Keep the last results on screen while the next ones load.
      setState((s) => (s.kind === 'done' ? s : { kind: 'loading' }));
      setPending(true);
      try {
        const res = await fetch(`/admin/search?q=${encodeURIComponent(term)}`, {
          signal: ctrl.signal,
        });
        // A dead session redirects to sign in: send the whole page there.
        if (res.redirected) {
          window.location.assign(res.url);
          return;
        }
        if (!res.ok) {
          throw new Error(String(res.status));
        }
        const groups = toGroups((await res.json()) as SearchResults);
        setActive(0);
        setPending(false);
        setState({ kind: 'done', query: term, groups });
      } catch {
        if (!ctrl.signal.aborted) {
          setPending(false);
          setState({ kind: 'error' });
        }
      }
    }, DEBOUNCE_MS);
    return () => {
      clearTimeout(timer);
      ctrl.abort();
    };
  }, [open, term]);

  const rows = useMemo(
    () => (state.kind === 'done' ? state.groups.flatMap((g) => g.rows) : []),
    [state],
  );

  function close() {
    setOpen(false);
    setQuery('');
    setActive(0);
  }

  function go(href: string) {
    close();
    router.push(href);
  }

  function onKeyDown(e: React.KeyboardEvent) {
    if (rows.length === 0) {
      return;
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setActive((i) => (i + 1) % rows.length);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setActive((i) => (i - 1 + rows.length) % rows.length);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      go(rows[active].href);
    }
  }

  const optionId = (key: string) => `${listId}-${key}`;
  let index = -1;

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="box-border inline-flex h-8 min-w-0 cursor-pointer items-center gap-2 rounded-[6px] border border-control bg-bg px-3 text-[13px] text-fg-muted max-lg:w-8 max-lg:justify-center max-lg:px-0 lg:w-[260px]"
      >
        <SearchIcon />
        <span className="flex-1 text-left max-lg:sr-only">Find users, teams, links…</span>
        <kbd className="rounded-[4px] border border-control px-[5px] font-mono text-[11px] leading-[18px] text-fg-muted max-lg:hidden">
          ⌘K
        </kbd>
      </button>
      <Dialog
        open={open}
        onClose={close}
        title="Search"
        className="max-w-[560px] [&_h2]:sr-only [&>div:first-child]:p-0 [&>div:nth-child(2)]:p-0"
      >
        <div className="flex flex-col">
          <div className="flex items-center gap-2 border-b border-hairline px-4">
            <SearchIcon className="shrink-0 text-fg-muted" />
            <input
              role="combobox"
              aria-expanded={rows.length > 0}
              aria-controls={listId}
              aria-activedescendant={rows[active] ? optionId(rows[active].key) : undefined}
              aria-autocomplete="list"
              aria-label="Search users, teams and collections"
              autoComplete="off"
              spellCheck={false}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={onKeyDown}
              placeholder="Find users, teams, links…"
              className="h-12 min-w-0 flex-1 border-0 bg-transparent text-[14px] text-fg outline-none focus-visible:shadow-none placeholder:text-fg-subtle"
            />
          </div>
          <div
            id={listId}
            role="listbox"
            aria-label="Results"
            aria-busy={pending}
            className={cx(pending && 'opacity-60', 'max-h-[360px] min-h-[96px] overflow-auto py-2')}
          >
            {state.kind === 'idle' ? (
              <Note>Type to search users, teams and collections.</Note>
            ) : null}
            {state.kind === 'loading' ? <Note>Searching…</Note> : null}
            {state.kind === 'error' ? (
              <Note>Search is not available right now. Try again.</Note>
            ) : null}
            {state.kind === 'done' && state.groups.length === 0 ? (
              <Note>Nothing found for “{state.query}”.</Note>
            ) : null}
            {state.kind === 'done'
              ? state.groups.map((g) => (
                  <div key={g.label} role="group" aria-label={g.label}>
                    <div
                      aria-hidden="true"
                      className="px-4 pt-2 pb-1 text-[11px] font-medium tracking-[0.02em] text-fg-subtle uppercase"
                    >
                      {g.label}
                    </div>
                    {g.rows.map((r) => {
                      index += 1;
                      const selected = index === active;
                      return (
                        <Link
                          key={r.key}
                          id={optionId(r.key)}
                          role="option"
                          aria-selected={selected}
                          href={r.href}
                          onClick={(e) => {
                            e.preventDefault();
                            go(r.href);
                          }}
                          className={cx(
                            'mx-2 flex items-center gap-3 rounded-[6px] px-2 py-1.5 text-fg no-underline hover:text-fg',
                            selected && 'bg-surface-300',
                          )}
                        >
                          <span className="min-w-0 flex-1 truncate">{r.primary}</span>
                          {r.secondary ? (
                            <span className="truncate font-mono text-[12px] text-fg-subtle">
                              {r.secondary}
                            </span>
                          ) : null}
                        </Link>
                      );
                    })}
                  </div>
                ))
              : null}
          </div>
        </div>
      </Dialog>
    </>
  );
}

function Note({ children }: { children: React.ReactNode }) {
  return <p className="m-0 px-4 py-3 text-fg-muted">{children}</p>;
}
