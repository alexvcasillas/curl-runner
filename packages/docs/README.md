# curl-runner website

The site at [curl-runner.com](https://www.curl-runner.com): marketing pages, the documentation and the changelog. It is a Next.js 16 app (App Router, Tailwind CSS 4) in this pnpm workspace; the documentation is written in MDX with [Fumadocs](https://fumadocs.dev).

Requires Node 20.9 or newer (the repo uses Node 24) and pnpm. Run everything from the repo root after `pnpm install`.

## Commands

```bash
pnpm --filter @curl-runner/docs dev        # dev server on http://localhost:3000
pnpm --filter @curl-runner/docs build      # production build
pnpm --filter @curl-runner/docs start      # serve the build (add -p 4010 for another port)
pnpm --filter @curl-runner/docs test       # generate the Fumadocs sources, then Vitest
pnpm --filter @curl-runner/docs test:e2e   # Playwright phone-width checks
pnpm typecheck                             # TypeScript across the workspace
```

`tests/features/examples.test.ts` runs the docs' YAML examples through a real CLI binary; point `CURL_RUNNER_BIN` at one to enable it.

## Where things live

- `content/docs/**` is the documentation: MDX pages with `title` and `description` front matter, ordered by each folder's `meta.json`. Pages are served at `/docs/...` and as Markdown at `/docs/....md`; `/llms.txt` and `/rss.xml` are generated from the same content.
- `app/(marketing)/` holds the marketing pages: home, `features/[slug]`, `alternatives`, `downloads`, `tools-database` (the fact sheet) and `changelog`. Each exports its own metadata through `pageMetadata` in `lib/site.ts`.
- `components/site/` holds the shared header, footer, install tabs, code panes and buttons.
- `lib/` holds page data (`features-data.ts`, `alternatives.ts`, `home-features.ts`, `install-methods.ts`) and the GitHub release and changelog readers. The changelog page is generated from `packages/cli/CHANGELOG.md`; each release headline comes from the bold first line of its changeset, see the changeset section of [CONTRIBUTE.md](../../CONTRIBUTE.md).
- `app/api/test/` holds the small endpoints the docs examples call.
- `tests/` holds the Vitest suites and the Playwright spec.

## Checking URLs

After a deployment (or a local `start`), confirm it serves every URL the production site serves:

```bash
scripts/check-site-urls.sh http://localhost:4010
```
