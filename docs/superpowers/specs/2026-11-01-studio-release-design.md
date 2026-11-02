# curl-runner studio — first public release (installable, local-first) and the server's first brick

Status: draft for owner review · 2026-11-01

Mockups: the "v1.1 — installable, local-first" row on the studio canvas,
https://claude.ai/artifact/EeLYjq1iRGw7SwAWSbvnps (private to the owner): `LocalFirst`,
`WelcomeLocal`, `TourRun`, `TourResult`.

## Why

Studio v1 is merged, but nobody can install it. The website and the blog post say it's "an
early macOS preview, built from source", so most people who read about it never try it.

This release makes the studio something you download and open. It only ships what works
today: local YAML collections, editing, validation and runs. Everything that needs the
curl-runner server is either marked "Coming soon" or hidden until the server exists.

It also starts that server, in Rust, with one endpoint. The studio's first-run walkthrough
calls it, so the server is real from day one and grows from there.

## The reader's path (owner's words, condensed)

1. Finds a download button on the landing page, the downloads page or the blog post.
2. Installs and launches the studio.
3. Chooses to work locally on first launch.
4. A short walkthrough helps them create and run a first request against
   `https://api.curl-runner.com/health`.
5. After it passes, they point a request at one of their own servers.

Success: someone who has never built Rust goes from the website to a passing first request
in a few minutes, and nothing in the app pretends to do something it can't.

## Goals

1. **A signed, notarized macOS app** people can download and open without Gatekeeper
   warnings.
2. **A Homebrew cask:** `brew install --cask curl-runner-studio`.
3. **Download entry points** on `/downloads`, the landing page's studio section and the
   blog post, replacing "build it from source".
4. **Local-first first launch** (mockup `LocalFirst`): "On this Mac" is the only choice you
   can pick. Cloud and Self-hosted are visible, disabled, with a "Coming soon" pill.
5. **Server features hidden or marked "Coming soon"**, behind one switch (below), never
   deleted.
6. **A first-run walkthrough** (mockups `WelcomeLocal`, `TourRun`, `TourResult`).
7. **`curl-runner-server`:** a new Rust crate with `GET /health`, shipped as a Docker image
   and deployed at `api.curl-runner.com`.
8. **Loose ends:** `curl-runner diff a b` honours `diff.exclude` (today it's a bug, see
   below), plus anything the owner finds testing this week.

## Non-goals

- Accounts, GitHub OAuth, Postgres, sync, sharing, share links. That's the next project,
  with its own design.
- Auto-update inside the app. People update with `brew upgrade` or a new download.
- Linux and Windows builds of the studio. Light theme.
- The Mac App Store.

## Anti-goals (a failure even if it technically works)

- **A release build shows something that doesn't work**, or shows mocked data: fake
  accounts, fake share links, the Acme fixtures, the Developer menu.
- **The mocked backend is compiled into the distributed app.**
- **Bringing server features back means rebuilding UI.** Hiding is a switch, not a
  deletion.
- **The walkthrough fails on a fresh Mac** for a reason the user can't understand (no
  network, server down) without saying what happened.
- **The studio and the CLI disagree** about a file or a run (unchanged from v1).
- **Anything private leaks into this public repo:** the owner's Apple ID or team details,
  signing material, employer or customer data (see `AGENTS.md`). Signing secrets live only
  in GitHub Actions secrets.

## Decided (by Alex)

- Make the studio installable and polish the loose ends before building the real server.
- Server features: disabled with "Coming soon" where that makes sense, otherwise hidden,
  and kept in the code so they come back without rework.
- The walkthrough: create a first request against `https://api.curl-runner.com/health`,
  run it, then try your own server.
- Start the backend now, small, and let it grow. Everything in Rust.
- Hosting: the owner's Coolify at `coolify.alexvcasillas.dev`. The owner points
  `api.curl-runner.com` (DNS currently on Vercel) at it.
- Cloud sign-in later is GitHub OAuth only. No email and password. (Next project; recorded
  here so this release doesn't contradict it.)
- The owner has an Apple Developer Program account for signing and notarization.

## The studio

### The server-features switch

One compile-time switch decides whether server-backed UI exists in the build.

- **Off** in every distributed build (the release workflow's build). The `MockBackend`, its
  fixtures and the Developer menu are not compiled in.
- **On** for development, so the existing screens, the mock and their tests keep working.
- The `Backend` trait seam (`crates/curl-runner-studio/src/model/backend/mod.rs`) stays as
  it is. With the switch off, the app has no backend at all, not a stub that pretends.

What the switch controls, using the existing code locations:

| Feature | Switch off (release) |
| --- | --- |
| First-launch screen (`sign_in.rs`, `workspace/routing.rs` `first_launch`) | Replaced by the local-first screen. Cloud and Self-hosted shown disabled, "Coming soon". |
| Share button and dialog (`share_dialog.rs`, `share_sync.rs`) | Button disabled with a "Coming soon" tooltip. Dialog unreachable. |
| Collection settings, Sharing page (`collection_settings.rs`) | Disabled with "Coming soon", or hidden. |
| Open a shared link (`open_shared.rs`, Welcome card, `curl-runner://` links) | Welcome card disabled, "Coming soon". A `curl-runner://` link opens the app and says sharing is coming soon. |
| Settings, Accounts & sync (`settings_view.rs`) | One "Coming soon" card in place of Server, Account and Sync. |
| Sync status (`sync.rs`, sidebar chip, title-bar pill) | Sidebar shows "Working locally". No Offline pill. |
| Read-only customer view (`read_only.rs`), received copies | Hidden. |
| Sync conflicts and Keep both (`conflict.rs`, `conflict_view.rs`) | Hidden. |
| Offline mode ("Continue offline", `BackendError::Offline` copy) | Hidden. Local is the only mode. |
| Developer menu | Already debug-only. Must also be absent with the switch off. |

"Coming soon" copy says what the feature does and that it needs the curl-runner server,
for example: "Share links need the curl-runner server. Until then, share the YAML files
through git."

### First launch

Matches mockup `LocalFirst`. Shown once, when no connection choice is stored. "Get started"
stores the local choice and opens Welcome. A small "Follow along on GitHub" link points to
the repo.

### Welcome and the walkthrough

Welcome (mockup `WelcomeLocal`) gains a "Run your first request" card at the top while the
walkthrough hasn't been completed or skipped.

The walkthrough has three steps:

1. **Create.** Creates a `getting-started` collection on disk with one file, `health.yaml`,
   and opens it.
2. **Run** (mockup `TourRun`). Explains the file and points at Run. The file:

   ```yaml
   request:
     name: curl-runner is up
     url: https://api.curl-runner.com/health
     method: GET
     expect:
       status: 200
       body:
         status: ok
   ```

3. **It passed** (mockup `TourResult`). Shows the passing checks, mentions that the same
   file runs in CI with `curl-runner getting-started/`, and offers "New request" for your
   own API, or "Done".

If the run fails (no network, server down), step 3 says what happened in plain words and
offers to try again or finish anyway. The file stays on disk either way.

The walkthrough can be skipped at any step and replayed from the command palette or the
Help menu.

### Packaging

- `scripts/studio-bundle.sh` already builds `curl-runner studio.app` with an Info.plist
  (`com.curl-runner.studio`, macOS 12.0+, the `curl-runner://` scheme). Release builds use
  it.
- **The app has no icon today.** Use the website's mark (owner's call): the `TerminalMark`
  SVG from `packages/docs/components/site/SiteHeader.tsx`, white on the dark rounded tile,
  as in `packages/docs/public/apple-icon-dark.png`. The 1024px master is generated from
  that SVG with the padding of macOS's app icon grid, then turned into an `.icns`, both
  committed. The source SVG stays the one place the mark is defined.
- One universal binary (Apple silicon and Intel).
- Signed with a Developer ID Application certificate and the hardened runtime, notarized,
  and stapled. Shipped as a `.dmg` (drag to Applications) on the GitHub release.

## The server: `curl-runner-server`

- New workspace member `crates/curl-runner-server`, `publish = false`.
- Axum on Tokio.
- One endpoint: `GET /health` returns `200` with JSON `{"status": "ok", "version": "<x.y.z>"}`.
  The version is the workspace version.
- Binds to `0.0.0.0` on `PORT` (default `8080`). Logs one line per request to stdout. Shuts
  down cleanly on SIGTERM, so Coolify redeploys don't drop requests.
- CORS open for `GET /health` (it's public and harmless).
- **Docker image:** a static musl binary in a minimal base image, for `linux/amd64` and
  `linux/arm64`. Published to `ghcr.io/alexvcasillas/curl-runner-server`.
- **Same pattern as the owner's other Coolify projects (owner's call), in one workflow:**
  - Runs on pull requests, on pushes to `main`, and by hand (`workflow_dispatch`). The
    server deploys from `main` on its own schedule, not with CLI releases.
  - **Build** needs the tests to pass. Buildx with the GitHub Actions cache.
    `docker/metadata-action` tags `latest` on `main` and `sha-<short>` on every build. Pull
    requests build the image but don't push it, so a broken Dockerfile fails before merge.
  - **Deploy** (not on pull requests) `POST`s the Coolify deploy webhook with a bearer
    token, `--fail` and `--retry 3`, in a deploy concurrency group that never cancels a
    running deploy. Secrets: `COOLIFY_WEBHOOK_SERVER` and `COOLIFY_TOKEN`.
  - **`docker-compose.prod.yml`** at the repo root is what Coolify runs, as one Docker
    Compose resource:
    - `image: ghcr.io/alexvcasillas/curl-runner-server:${SERVER_IMAGE_TAG:-latest}`, so a
      rollback is setting the tag to a sha in Coolify;
    - the external `coolify` network, so Coolify manages networking and the proxy;
    - a healthcheck with a `start_period`, and `restart: unless-stopped`;
    - settings as `${VAR}`, which Coolify shows in its UI.
  - **`docker-compose.preview.yml`** runs the real production image locally, the closest
    check to "does what we ship boot and serve".
  - A minimal image has no `curl`, so the binary checks itself: a
    `curl-runner-server healthcheck` subcommand calls its own `/health` and exits 0 or 1.
- **Self-hosting:** the same compose file, documented. When Postgres arrives in the next
  project it runs as its own service (a separate Coolify service in our deploy), and the
  server gets its host and credentials through `${VAR}` settings.
- Tests: `/health` returns the status and version, and unknown paths return `404`.

## The website

- **`/downloads`:** a studio section with the `.dmg` link from the latest GitHub release,
  `brew install --cask curl-runner-studio`, and the macOS 12+ requirement.
- **Landing page:** the studio section's "build it from source" copy becomes a download
  button.
- **`lib/studio.ts`, `content/docs/features/studio.mdx`:** drop "not distributed yet" and
  document installing.
- **Blog post:** the line "It's an early macOS preview for now, built from source" changes to
  match, and the post's `updated` date is set.
- **Self-hosting docs:** a short page for the server image and compose file, marked as early.

## The `diff.exclude` loose end

`curl-runner diff a b` (`crates/curl-runner/src/diff_mode.rs`, `run_subcommand`) builds its
`DiffConfig` with an empty `exclude` and default match rules, so it ignores `diff.exclude`
from the config file. The post-run diff mode honours it. `curl-runner diff a b` should
apply the same global `diff.exclude` and match rules, with a test that pins it.

## Release and versioning

- The studio ships with the CLI's release, same version, same GitHub release. New jobs in
  `.github/workflows/release.yml` (macOS build, sign, notarize, dmg, upload) run only when
  the CLI release runs.
- The server has its own workflow (above).
- Secrets the owner adds to GitHub Actions: the Developer ID certificate (`.p12`, base64)
  and its password, and an App Store Connect API key (key ID, issuer ID, `.p8`) for
  notarization. The image push uses `GITHUB_TOKEN`. For deploys: `COOLIFY_WEBHOOK_SERVER`
  and `COOLIFY_TOKEN`.
- Changeset bump: ask the owner after human verification (`AGENTS.md`).

## Constraints

- Rust toolchain `1.99.1`, edition 2024, workspace version from `packages/cli/package.json`.
- `AGENTS.md`: `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace` pass. Every change ships with tests. Public-repo hygiene.
- The studio's look follows the design system, dark only.
- Squash merges. Required checks: Lint and Check, Rust (macOS, Ubuntu, Windows), musl.

## Calls made in writing this up (Claude's, not Alex's)

- **The switch is compile-time**, so the mock can't ship by accident. A runtime flag would
  still compile the mock into the app.
- **The walkthrough's collection lives at `~/curl-runner/getting-started`**, and an existing
  folder there isn't overwritten.
- **`/health` returns `status` and `version`, nothing else.**
- **Universal binary and a `.dmg`**, rather than separate per-architecture downloads or a
  `.zip`.
- **The studio ships with the CLI's release**, sharing its version and tag. The server
  doesn't: it deploys from `main` (owner's Coolify pattern, above).
- **Coolify pulls the published GHCR image** rather than building from the repo.
- **The studio and server jobs never block the CLI.** They run after the CLI is published.
  If notarization or the image push fails, the CLI still ships, and a manual
  `workflow_dispatch` rebuilds the studio or the server for an existing tag. A studio-only
  fix ships as a normal patch release.
- **Order of shipping:**
  1. The server PR merges first and its workflow publishes the image. The owner makes the
     GHCR package public, sets up the Coolify stack from `docker-compose.prod.yml`, adds the
     deploy secrets and points DNS, and
     `https://api.curl-runner.com/health` answers.
  2. The studio PRs merge and a release produces the signed `.dmg`.
  3. The website changes (download buttons, copy) merge only after that release exists, so
     no button links to nothing.
- **Old v1 state:** with the switch off, the release build ignores any stored server
  connection, sign-in, received copies or mock data, and treats the app as local. It
  doesn't delete them, so a dev build still finds them. A test pins that a stored "Cloud"
  connection opens Welcome in local mode.
- **Testing findings from this week** become their own small PRs, outside this plan.
- **The cask follows the CLI formula's pattern.** `alexvcasillas/homebrew-tap` already has
  a workflow (`update-curl-runner.yml`) that updates the formula from this repo's releases
  every hour. The cask (`Casks/curl-runner-studio.rb`) is added there, and that workflow,
  or a sibling, updates its version and `sha256` the same way. No token from this repo is
  needed. The `.dmg` asset name is stable: `curl-runner-studio-<version>.dmg`. This is a
  change in the tap repo, which is part of this work.

## Left to the builder

- How the switch is expressed (a Cargo feature, `cfg`, or both) and which build turns it
  on, as long as distributed builds have it off and the mock isn't compiled in.
- The `.dmg` layout and tooling, notarization tooling (`notarytool`), and how CI imports
  the certificate.
- The minimal base image.
- Walkthrough step layout beyond the mockups, and its persisted state.
- Test shape.

## Open questions

None left for the owner. Owner steps during the work: make the GHCR package public, set up
the Coolify stack, point `api.curl-runner.com` at it, and add the signing and Coolify
secrets.
