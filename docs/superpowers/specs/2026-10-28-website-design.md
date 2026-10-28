# curl-runner website + Fumadocs docs: design

## Intent

The current site (`packages/docs`) is hand-built: docs pages are React components, search and the Markdown copies for LLMs come from a custom Bun script, and the visual design is generic. curl-runner 2.0 is now a native Rust CLI, so the site should look as sharp as the tool and tell a developer in seconds what it is: API tests as YAML, run anywhere.

Reference the owner chose: the nextjs.org homepage — minimal, very powerful, communicates the product almost instantly.

## Goals

- One Next.js App Router app that serves the marketing site and the docs, deployed to Vercel as today.
- The **Monochrome** design (mockups in `website-mockups/`) across every page, docs included.
- Docs on **Fumadocs** under `/docs`: MDX content, sidebar, built-in search, `llms.txt` and per-page Markdown.
- **pnpm** as the package manager; Bun removed from the repo.
- Every marketing claim checkable: real features, real numbers or explicit placeholders.

## Non-goals

- The desktop studio app.
- Any change to the CLI, to what the release workflow does (its Bun steps move to pnpm, see below), or to the Homebrew tap.
- New docs content beyond fixing what is wrong (see Docs content).

## Anti-goals (a failure even if it works)

- Breaking existing URLs: `/docs/...` pages, `/install.sh`, `/install.ps1`, `/api/test/...`, `/llms.txt`, `/rss.xml`, `/sitemap.xml`.
- Invented stats, logos, testimonials, or competitor claims stated as fact without checking.
- Gradient washes, glow effects, emoji, or a template look.

## Decided by the owner

- **One app, one Vercel project**, same domain (www.curl-runner.com) and project as today. Not a pure static export: `/api/test/*` stays as route handlers; every marketing and docs page is prerendered.
- **Fumadocs** for docs, inside the same app at `/docs`.
- **pnpm** workspace. Remove `bun.lock`, Bun scripts, the Bun shebang script, and `.cursor/rules/use-bun-instead-of-node-vite-npm-pnpm.mdc`; `bun install` stays in user-facing docs only as an install option of the npm package (`bun i -g @curl-runner/cli`).
  - This includes the CI workflows: `release.yml` (`setup-bun`, `bun install --frozen-lockfile`, changesets' `version: bun run version`) and `lint-and-check.yml` move to pnpm with no change in behaviour. Prove `release.yml` with `gh workflow run release.yml --ref <branch> -f smoke=true` before merging. If the app leaves `packages/docs`, update the paths `release.yml` hard-codes (`packages/docs/public/install.ps1`).
- **Vercel** project `curl-runner-docs` (settings read from the project): Root Directory `packages/docs`, Framework Next.js, Node 22.x, Install Command auto-detected from the lockfile, Build Command overridden in the dashboard to `bun run build`. The owner changes the Build Command (to `pnpm build`, or clears the override) and the Root Directory if the app moves; the builder can read settings and deployments with the Vercel CLI (logged in).
  - `vercel.json` sits at the repo root while Vercel reads it from the Root Directory, so its `ignoreCommand` (skip builds of `changeset-release/main`) may not be applied today. Check it, and put `vercel.json` where Vercel reads it.
  - Done = a passing **preview deployment** of the branch, checked with the CLI or curl: every `/docs/*` URL from the current sitemap, `/docs/<page>.md`, `/llms.txt`, `/install.sh` and `/install.ps1` byte-for-byte against `main`, and `/api/test/*`.
- **`AGENTS.md` replaces `CLAUDE.md`** (already on this branch).
- **Monochrome** design, homepage plus the four marketing pages and the changelog, as mocked up and approved.
- A **`/changelog` page** generated from the CLI's `CHANGELOG.md` (below).

## Design system (from the approved mockups)

- Ground `#0A0A0A`, raised surface `#0F0F0F`, hover/selected `#171717`/`#1F1F1F`; hairlines `#1F1F1F` (inside), `#262626` (cards, inputs).
- Text `#EDEDED`; secondary `#A1A1A1`; tertiary `#8A8A8A` (the lowest that passes 4.5:1 on the ground).
- One accent, cyan `#22D3EE`: eyebrow labels, YAML keys, the logo mark, status dots. Success `#4ADE80`, error `#F87171` only in terminal output.
- Type: Geist (UI and headings, 700 with tight negative tracking on display sizes) and Geist Mono (code, eyebrows, file names). Load with `next/font`, self-hosted.
- Primary button: light fill `#EDEDED` on dark text; secondary: 1px `#262626` outline. Radius 8–10px buttons, 12–14px cards.
- Content width 1200px with 24px gutters; layouts are flex/grid that wrap at phone width; code panes scroll horizontally inside their box.
- Dark only for the marketing site. Docs: dark by default matching these tokens; a light theme is the builder's call (Fumadocs supports both) but dark is the reference.
- Icons: inline stroke icons (lucide-react is already a dependency).

## Pages

Mockups (`website-mockups/*.dc.html`) are the reference for layout, copy and styles; the `.dc.html` wrapper and `{{holes}}` are the mockup format, not code to ship.

| Route | Mockup | Notes |
|---|---|---|
| `/` | `Main.dc.html` | Install box tabs (curl, Homebrew, npm, cargo) with copy; YAML + real terminal output; feature grid; "libcurl inside"; CI; footer. The `[N] ms startup` tile stays a placeholder until measured; drop the tile if no benchmark exists at launch. |
| `/features/[slug]` | `Feature.dc.html` | One template for all 19 features in `lib/features-data.ts`; sidebar lists every feature; previous/next links. Rewrite each feature's copy in the mockup's voice; remove the "up to 10x" claim and other unsourced speed claims. Every YAML example must use real config keys (check against `crates/curl-runner-core/src/model`). |
| `/alternatives` | `Alternatives.dc.html` | Descriptive cells, not ✓/✗. Resolve every `[VERIFY]` cell against each tool's current docs before launch, and set the "Checked" date. |
| `/downloads` | `Downloads.dc.html` | Install tabs with a note each; binaries table filled live from the latest GitHub release (`lib/github-release.ts` logic, cached). |
| `/tools-database` | `ToolsDatabase.dc.html` | Visible fact sheet; remove the hidden SEO block. |
| `/changelog` | `Changelog.dc.html` | New page; the nav and footer "Changelog" links point here. See Changelog below. |
| `/docs/**` | — | Fumadocs, themed with the tokens above; same header as the marketing site. |

Keep: OG images per page (restyle to Monochrome), `sitemap`, `robots`, `rss.xml`, `manifest`, icons, Vercel Analytics, Speed Insights and Pulsora analytics.

## Changelog

- Built at deploy time from `packages/cli/CHANGELOG.md`, not the GitHub Releases API: merging the changesets "Version Packages" PR updates that file and triggers the Vercel build, while the GitHub Release only exists minutes later, so the file is always current and needs no API token.
- Per release: version (anchor `/changelog#<version>`), date, Major/Minor/Patch label, a headline, the notes rendered as Markdown with the docs' code styling, PR and commit links. Long releases (2.0.0) collapse behind "Show all changes"; a `### Migration` section renders as the highlighted box.
- Dates: the changelog has none; take each version's date from its git tag or the commit that added the heading (the builder picks; it must work in Vercel's build, which may be a shallow clone).
- Headline convention, owner-approved: a changeset starts with a bold first line (`**Install with Homebrew**`) that becomes the headline; without one the page shows the version alone. Add headlines to the existing 2.1.0, 2.0.0, 1.25.0 and 1.24.0 entries once; document the convention in `CONTRIBUTE.md`.
- Feeds `rss.xml` (one item per release) and the sitemap. Show the newest releases with an "Older releases" link or pagination; the builder picks the cut-off.

## Docs content

- Convert the ~40 pages under `app/(docs)/docs/**` (React + `snippets.ts`) to MDX with the **same URLs**.
- Carry content over; fix only what is wrong for 2.x: Bun-era install steps, the `validation:` key in snippets (the real key is `expect:`), timeouts (milliseconds), anything the CLI no longer does or does differently (`crates/CHANGES-FROM-TS.md` lists behaviour changes).
- Fumadocs' `llms.txt` and per-page Markdown replace `scripts/jsx-to-markdown-with-frontmatter.ts` and `public/docs/*.md`; `/llms.txt` and `/docs/<page>.md` keep working.
- Search: Fumadocs' built-in search.

## Constraints

- `/api/test/*` routes keep their current behaviour (the bundled examples call them), including `/api/test/delay/[seconds]` taking seconds.
- `public/install.sh` and `public/install.ps1` are served byte-for-byte as today.
- `vercel.json`'s `ignoreCommand` (skips builds of the changesets release branch) stays.
- The repo is public: no employer or customer details anywhere.
- Performance: pages prerendered, fonts self-hosted, no client JS on marketing pages beyond the install-tab and copy buttons.

## Left to the builder

- Whether the app keeps the `packages/docs` path or moves (for example `apps/web`); update the Vercel root directory accordingly.
- Fumadocs layout details (sidebar grouping, TOC), MDX components for code tabs and callouts, and the Shiki theme matching the mockup's YAML colours.
- How `features-data.ts` is restructured to hold the rewritten copy.
- Whether `packages/remotion` and `packages/redirect` move into the pnpm workspace as they are.
