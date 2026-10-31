# curl-runner blog — design

## Intent

A blog on www.curl-runner.com that brings search and AI-answer traffic from engineers who
let agents write code and now worry about what reaches production. The first post, "The
importance of testing in the Agent era", argues that a layered test suite (unit, e2e,
smoke) is what makes agent-written changes safe to ship, and shows curl-runner as the
tool for the smoke layer, backed by a real user: Tiptap's document conversion pipeline.

Success: the post ranks and gets cited for "testing AI agent code" style queries, and a
reader who finishes it installs curl-runner and writes a first smoke test against their
own staging.

## Reader

A developer or engineer who wants a CI/CD pipeline that is much more robust and "AI
proof": agents already write part of their code, and they want the pipeline, not the
reviewer, to be what catches an agent's mistakes before users do.

## Goals

- `/blog` index styled like `/changelog`.
- `/blog/<slug>` full post page.
- The first post, written in Alex's voice (`/alex`) and cleaned with `/unslop`.
- Blog posts are first-class for SEO: sitemap, metadata, OG image, structured data,
  RSS, llms.txt.

## Non-goals

- Comments, tags/categories, search, pagination, multiple authors, newsletter signup.
  One author, a handful of posts; add any of these when the post count needs them.
- A CMS. Posts are MDX files in the repo.

## Anti-goals

- Leaking private Tiptap information (see Constraints).
- A post that reads as generated: filler, hedged lists, "in today's fast-paced world".
  It must sound like Alex.
- Passing the illustrative example off as a real Tiptap incident.

## Constraints

- **Tiptap disclosure.** The post may name Tiptap and its document conversion pipeline as
  a curl-runner user and link to https://tiptap.dev/ and
  https://tiptap.dev/product/conversion. It may say the pipeline runs hundreds of
  curl-runner smoke tests against its preview, staging and production pipelines, and that
  those tests have, multiple times, stopped changes that would have hurt existing
  integrators. Nothing else: no repo names, hostnames, CI config, suite names, exact
  counts, or internal incidents.
- **Example scenario.** The concrete catch in the post is invented and labeled as an
  example (e.g. "Here's the shape of it:"), not presented as a Tiptap incident: an agent
  refactors a converter, the export endpoint renames a field in its JSON response, and a
  smoke test asserting that field goes red in preview. Shown with the curl-runner YAML
  that catches it. The YAML must be valid against the current curl-runner schema and run
  with the current CLI.
- Lives in `packages/docs` (Next.js + fumadocs, Tailwind tokens, dark only), follows that
  package's conventions, tests in vitest like the rest of the site.
- Repo rules: public repo, squash merge, Lint and Check must pass.

## Decided (by Alex)

- Design: same as the changelog page, plus the post title (above the excerpt) links to a
  full page with the whole post.
- Post topic, angle, reader, the Tiptap framing and links above, and the labeled invented
  example.
- Written with /alex and /unslop.
- First person. The post may say Alex works at Tiptap and built and runs the conversion
  pipeline's curl-runner smoke suite (it's on his public profile). The disclosure limits
  under Constraints still apply to everything else.

## The pages

### `/blog`

Mirrors `/changelog`'s layout and components: hero (Eyebrow "Blog", big headline, one-line
lede, RSS button pointing at the blog feed), then one `<article>` per post with the same
two-column row. Left column: date and reading time where the changelog has version, date
and bump badge. Right column: the post title as an `h2` link to `/blog/<slug>`, the
excerpt, and a "Read the post" link. Newest first.

### `/blog/<slug>`

Site header, then a reading-width column (~720px): eyebrow/back link to `/blog`, title,
date, reading time, author ("Alex Casillas"), then the MDX body rendered with the same
code highlighting (YAML theme) and prose styles the docs use. Ends with a short call to
action: install command and links to the docs getting-started page and the smoke-testing
relevant docs.

### Content model

MDX files in `packages/docs/content/blog/<slug>.mdx`, a second fumadocs-mdx collection.
Frontmatter: `title`, `description` (meta and the excerpt on the index), `date`
(ISO), optional `updated`, `author`. Reading time computed from the body.

### SEO surfaces

- `pageMetadata` per post (title, description, canonical), `og:type=article`, published
  and modified times.
- Per-post OG image in the existing OG-card style.
- JSON-LD `BlogPosting` on each post.
- Sitemap entries for `/blog` and each post, with lastmod from git like the other pages
  (`page-sources.ts`).
- Blog RSS feed at `/blog/rss.xml` (the existing `/rss.xml` stays the releases feed), and
  `<link rel="alternate">` to it from `/blog` and posts.
- Posts listed in `llms.txt` with markdown versions, like docs pages.
- "Blog" link in `SiteHeader`, after Changelog.

## The first post

Working title: "The importance of testing in the Agent era". Rough arc (the voice and
final structure are the writer's):

1. Agents write code faster than anyone can review line by line; review stops being the
   safety net, the test suite becomes it.
2. Each layer catches a different failure: unit (broken logic), e2e (broken flows), smoke
   (a broken deployment — the thing an agent can't see from its editor). Smoke is the
   layer most teams skip.
3. Tiptap's conversion pipeline: hundreds of curl-runner smoke tests on preview, staging
   and production; they've stopped changes that would have broken integrators more than
   once. Links to tiptap.dev and the conversion product page.
4. The labeled example: the renamed-field catch, with its YAML.
5. Why curl-runner fits agents: tests are plain YAML an agent writes and reads as easily
   as a person, the same file runs in every environment, machine-readable output for CI.
6. Your first smoke test in five minutes: install, a minimal YAML, run against staging,
   wire into CI.

Length around 1,500–2,000 words. SEO title and description tuned for queries like
"testing AI-generated code", "smoke tests CI/CD", "AI agent code safety".

## Calls made in writing this up (Claude's, not Alex's)

- **CTA targets:** `quick-start` and `features/ci-integration`. No new docs page.
- **Publishing:** any post in `content/blog` is public; there is no draft flag. The first
  post's `date` is the day it merges.
- **CLI claims:** every claim and YAML snippet in the post is checked against the current
  CLI and docs (`output-formats`, `response-validation`). If a claim doesn't hold, the
  wording changes to match what the CLI does.
- **Markdown versions:** `/blog/<slug>.md`, listed in `llms.txt` under a "Blog" section.

## Left to the builder

Component reuse versus extraction from the changelog page, exact prose styles, OG card
layout, reading-time formula, test coverage shape, and the post's final wording and
section order.
