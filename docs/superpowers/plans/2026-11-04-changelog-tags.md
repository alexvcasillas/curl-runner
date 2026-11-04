# Tagged Changelog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pulsora-style tagged items (New / Improved / Fixed / Breaking) on curl-runner's /changelog, with a filter, and every past release backfilled.

**Architecture:** The tags live in the changeset text. `parseChangelog` splits each entry into headline, lead, tagged items and notes. The page renders items as hairline rows with a mono tag column. A small client component filters and paginates releases that are rendered on the server. A docs test enforces the format on new changesets and on CHANGELOG.md.

**Tech Stack:** Next.js (App Router) in `packages/docs`, TypeScript, Tailwind v4 tokens in `app/globals.css`, `marked` + `shiki` (already used), vitest + Testing Library, biome.

**Spec:** `docs/superpowers/specs/2026-11-04-changelog-tags-design.md`

## Global Constraints

- Tags are exactly `new`, `improved`, `fixed`, `breaking`. Labels are `New`, `Improved`, `Fixed`, `Breaking`. An item line is `- **<Label>:** text`, with a case-insensitive label and the colon inside the bold.
- Tag colors use only existing tokens: New `text-brand`, Improved `text-fg-secondary`, Fixed `text-yaml-string`, Breaking `text-error`.
- Untagged entries render exactly as today.
- Raw HTML in changelog Markdown stays escaped, as today.
- The changelog page and RSS still build from `packages/cli/CHANGELOG.md` at build time, and the page stays statically rendered.
- Copy (CHANGELOG backfill, CONTRIBUTE, page text) has no em dashes and uses short sentences. The backfill adds no claim that isn't in the original entry, its PR or its commits.
- Run `pnpm --filter @curl-runner/docs test`, `typecheck` and `build`, plus `pnpm exec biome ci packages/docs` (the root `check:ci` also scans local `target/`), before every commit.
- Commits: Conventional Commits, one line.

## Review Focus

1. An entry whose item text wraps onto an indented continuation line, or contains inline code with a colon (`` `a:b` ``), parses as one item with the full text. Test in Task 1.
2. A `#2.0.0`-style anchor for a release past the first 10 scrolls to a visible release. Test in Task 2.
3. `?tag=breaking` with no breaking items shows the empty state, not a blank page. Test in Task 2.
4. An entry with tagged items AND a `### Migration:` section keeps the migration box. Test in Task 1/2.
5. A changeset that has a bold headline but only untagged bullets fails the new check, with a message naming the file. Test in Task 4.

---

### Task 1: Parse lead, tagged items and notes

**Files:** Modify `packages/docs/lib/changelog.ts`, `packages/docs/lib/changelog-markdown.ts`. Test: `packages/docs/tests/lib/changelog.test.ts`.

**Interfaces:**
- Produces in `lib/changelog.ts`:
  - `export type ChangeTag = 'new' | 'improved' | 'fixed' | 'breaking'`
  - `export const CHANGE_TAGS: readonly ChangeTag[]`
  - `export const TAG_LABEL: Record<ChangeTag, string>`
  - `ChangeEntry` gains `lead: string`, `items: { tag: ChangeTag; text: string }[]` and `notes: string`. `body` stays as the full text after the headline, for RSS and the collapse logic.
  - `export function splitEntry(body: string): { lead: string; items: ChangeItem[]; notes: string }`
- Produces in `lib/changelog-markdown.ts`: `RenderedEntry` gains `leadHtml: string` and `items: { tag: ChangeTag; html: string }[]` (inline-rendered item text). `previewHtml`/`restHtml` now cover `notes` for entries with items, and the whole body for untagged entries (today's behaviour).

- [ ] **Step 1: Write failing tests**
  - `splitEntry` on lead + 3 items + a code block → lead text, 3 items with the right tags in order, notes = the code block.
  - Wrapped item continuation → one item.
  - Item with `` `a:b` `` → text intact.
  - Lowercase `**fixed:**` → `fixed`.
  - No items → `items: []`, lead = whole body, notes = `''`.
  - An untagged bullet before the first tagged item stays in the lead.
  - `parseChangelog` on a two-release sample → items populated per entry.
- [ ] **Step 2: Run** `pnpm --filter @curl-runner/docs test changelog`. Expected: FAIL.
- [ ] **Step 3: Implement.** Items are the contiguous run of tagged list items. Untagged list items or blank lines between tagged items don't end the run. A non-list block after the run starts the notes.
- [ ] **Step 4: Run** the tests. Expected: PASS.
- [ ] **Step 5: Commit** `feat(site): parse tagged changelog items`

### Task 2: Render tagged items, filter and pagination

**Files:**
- Modify `packages/docs/app/(marketing)/changelog/page.tsx`
- Create `packages/docs/components/site/ChangelogFilter.tsx` (client)
- Modify `packages/docs/app/rss.xml/route.ts`
- Test: `packages/docs/tests/marketing/changelog.test.tsx`

**Interfaces:**
- Consumes Task 1's `TAG_LABEL`, `CHANGE_TAGS`, `RenderedEntry.items`/`leadHtml`.
- The page renders every release server-side. Each release `<article>` gets `data-tags="new fixed …"` (the tags present), and each item row gets `data-tag`.
- `ChangelogFilter` (client) owns the chips and the "Load older releases" button. It reads and writes `?tag=` with `history.replaceState`, no navigation. It toggles a `hidden` attribute on rows, entries and releases by data attributes, and reveals a release targeted by `location.hash` on load and on `hashchange`.
- Server HTML shows all releases. Releases past the first 10 get `data-older` and are hidden by CSS only when the root has `data-js` set by the client component, so no-JS shows everything.

- [ ] **Step 1: Write failing tests**
  - Tagged rows render with label text and the per-tag class.
  - The lead renders.
  - An untagged entry renders as before.
  - Chips: clicking "Fixed" hides non-fixed rows and releases without fixed items.
  - `?tag=breaking` with none shows "No changes with this tag yet."
  - "Load older releases" reveals the next 10.
  - A hash of an older release reveals it.
  - RSS: the description contains `New: …` lines.
- [ ] **Step 2: Run.** Expected: FAIL.
- [ ] **Step 3: Implement.** Match Pulsora's live page (pulsora.co/changelog) for chip and row spacing. Take a screenshot of both pages with chrome-devtools at 1440 and 390 wide, compare, and fix drift. Keep curl-runner's own tokens and fonts.
- [ ] **Step 4: Run** the tests, typecheck and build. Expected: PASS.
- [ ] **Step 5: Commit** `feat(site): tag, filter and page changelog releases`

### Task 3: Backfill CHANGELOG.md

**Files:** Modify `packages/cli/CHANGELOG.md` only.

- [ ] **Step 1:** For each of the 46 releases, rewrite each entry's text (after the changesets prefix) into a bold headline, an optional one or two sentence lead, tagged items, and notes, following the spec's Backfill rules and tag meanings. Keep the prefix line, the version and section headings, code blocks, and `### Migration:` sections with their wording.
- [ ] **Step 2:** Run a script check in the scratchpad: every entry parses with `splitEntry` to at least one item. Diff-review the whole file for meaning changes.
- [ ] **Step 3:** Run the docs tests and build. Render /changelog locally and read it top to bottom.
- [ ] **Step 4: Commit** `docs(changelog): backfill tagged items for every release`

### Task 4: Enforce the format and document it

**Files:**
- Create `packages/docs/tests/lib/changeset-format.test.ts`
- Modify `CONTRIBUTE.md` ("Writing a changeset") and `AGENTS.md` (one line)

- [ ] **Step 1: Write the test.** For every `.changeset/*.md` except README.md, the body after the front matter has a bold headline and at least one tagged item. On failure, the message names the file and shows the expected format. The same test checks that every release in `packages/cli/CHANGELOG.md` has at least one tagged item. Prove it fails with a temporary bad changeset written to a tmp dir passed to the checker function; don't commit the bad file.
- [ ] **Step 2:** Update CONTRIBUTE.md with the format example, the tag meanings and the note about the check. Add an AGENTS.md line: "Changesets: bold headline, short lead, then `- **New|Improved|Fixed|Breaking:**` items (see CONTRIBUTE.md)."
- [ ] **Step 3: Run** the docs tests and biome.
- [ ] **Step 4: Commit** `test(site): require tagged items in changesets`

## Unresolved questions

- None blocking. Defaults: no changeset (site-only change, no CLI release), and GitHub release bodies left untouched.
