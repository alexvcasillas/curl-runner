# Tagged changelog items

Status: draft, 2026-11-04

## Why

Pulsora's changelog at pulsora.co/changelog uses the same design language as curl-runner's site. Each release there has a title, a short lead, and a list of items, and each item is tagged New, Improved or Fixed. You can scan a release in seconds and filter by tag. curl-runner's /changelog shows each changeset as a block of prose, so you have to read every paragraph to find what's new and what's fixed.

The owner asked for the Pulsora treatment on curl-runner's changelog, including a backfill of every past release, so the whole page reads the same way.

## Goals

1. Each changeset entry can carry a headline, a lead paragraph, and tagged items. Tags are New, Improved, Fixed and Breaking.
2. /changelog renders tagged items like Pulsora: one row per item, with the tag in a fixed-width mono column.
3. The page can be filtered by tag, client-side, like Pulsora.
4. Every release in `packages/cli/CHANGELOG.md` is backfilled into the new structure.
5. Every new changeset is checked in CI, so a missing tag fails the build.

## Non-goals

- Rewriting GitHub release bodies. They were written at release time and stay as they are.
- Changing how changesets generates the file (prefix, `### Minor Changes` sections). The format lives inside each entry's text.
- Changing the bump badge, the release dates, or the RSS URL.

## Anti-goals

- **Changing what a release said.** Backfilled items restate the original text. They add no claim that the original entry, its PR or its commits don't support.
- **Making the raw CHANGELOG.md worse to read on GitHub or npm.** The format is plain Markdown that reads well there too.
- **A page that breaks or empties when an entry has no tags.** Untagged entries still render as prose.

## Format

Inside a changeset summary, so also inside each CHANGELOG.md entry:

```markdown
**Import OpenAPI specs**

`curl-runner convert openapi` turns an OpenAPI 3.0, 3.1 or Swagger 2.0 spec into YAML files and a `.env`.

- **New:** `convert openapi <spec>` writes one file per operation, in a folder per tag.
- **Improved:** variables are prefixed per API, so two APIs can share one `.env`.
- **Fixed:** the cleaned `.env` report line says what actually changed.
```

- **Headline:** a bold line, already supported today. It's required for new changesets.
- **Lead:** the paragraph or paragraphs before the first tagged item. It's optional, and should be one or two sentences.
- **Items:** list lines that start with `**New:**`, `**Improved:**`, `**Fixed:**` or `**Breaking:**` (case-insensitive label, colon inside the bold). An item may wrap onto indented continuation lines and contain inline Markdown.
- **Notes:** anything after the items, such as code blocks, `### Migration: …` sections or longer explanations. It renders under the items with today's notes renderer, and keeps today's collapse and migration-box behaviour.
- Untagged list items inside the lead or notes stay plain Markdown.

Tag meanings:
- **New:** something you couldn't do before: a command, flag, YAML field, studio feature or platform.
- **Improved:** something that already existed now works better: faster, clearer output, better defaults, more inputs accepted.
- **Fixed:** a bug fix.
- **Breaking:** you have to change something when you upgrade. It's used only in major releases, or for a documented behaviour change.

## Parser

`lib/changelog.ts` `ChangeEntry` gains `lead: string` (Markdown) and `items: { tag, text }[]`, plus `notes: string` (Markdown after the items). `body` is kept for RSS summaries and backwards compatibility, or derived. The builder decides which.

- The parser only looks at the dedented entry body that's already produced today.
- An entry with no tagged items has `items: []` and `lead` holding the whole body, and renders as now.

## Page

The layout stays as today: release column on the left (version, date, bump badge), entries on the right. Per entry:
- the headline (h3), as now;
- the lead, in secondary text;
- the items, as a list with hairline separators like Pulsora. The tag column is about 88px wide, mono, uppercase and 12px, colored per tag. The item text is 15px.
- the notes, as now.

Tag colors, from existing tokens:

| Tag | Token |
| --- | --- |
| New | `brand` |
| Improved | `fg-secondary` |
| Fixed | `yaml-string` |
| Breaking | `error` |

**Filter.** Chips under the hero for All / New / Improved / Fixed / Breaking, styled like Pulsora's toggle group.
- A filter keeps only the matching items, hides entries left with none, and hides releases left with none.
- The lead and notes of a kept entry stay visible.
- With no matches it shows "No changes with this tag yet."
- The selection is in the URL as `?tag=new`, so it can be shared. The page stays statically rendered, and only the list is a client component.
- Without JavaScript, everything is shown.

**More releases.** Since every release is backfilled, the page lists all releases from the CHANGELOG.
- It shows 10 first, then "Load older releases" adds 10 at a time.
- "Older releases on GitHub" stays as a small link at the end.
- Release anchors (`/changelog#2.5.0`) must keep working for releases past the first 10. Render them all, and hide the extra ones until they're loaded or targeted by the hash.

## RSS

Each release's description lists its items as `New: …` lines after the headline or lead summary. Untagged entries keep today's summary.

## Backfill

Rewrite every entry in `packages/cli/CHANGELOG.md` (46 releases) into headline + lead + tagged items + notes.
- **Sources:** the entry's own text first. Use its PR and commits (`gh pr view`, `git show`) only to classify an item or split a paragraph correctly. Never add facts that aren't in the entry.
- **Keep:** version headings, `### … Changes` sections, the changesets prefix line (PR, commit, thanks), code blocks, and migration sections with their wording.
- **Writing rules:** no em dashes, short sentences, plain verbs, present tense ("`convert` now honours …").
- Patch releases whose entry is a single fix become one Fixed item with a headline.

## CI check

A docs test reads every `.changeset/*.md` except README.md, and fails when one has no bold headline or no tagged item. It also checks that every release in CHANGELOG.md has at least one tagged item, so a future hand edit can't silently drop the format.

## Docs

- Update the "Writing a changeset" section in CONTRIBUTE.md with the format, the tag meanings and an example.
- Add one line to AGENTS.md, so agents writing changesets follow the format.
- Update the website's changelog-related tests.
- Take a screenshot of the new page (desktop and mobile) for the PR.

## Done when

- /changelog shows tagged rows for every release, the filter works, `?tag=` deep links work, and `#version` anchors work for old releases.
- CHANGELOG.md reads cleanly on GitHub, and every release has tagged items.
- The RSS feed validates and includes the items.
- The docs tests, typecheck, build and biome pass.

## Left to the builder

- Whether `body` stays or is derived.
- The exact chip and row spacing, matched against Pulsora's live page with screenshots.
- How hidden releases reveal on hash navigation (CSS `:target` or a small effect).
