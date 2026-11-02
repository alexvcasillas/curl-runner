# Studio public release and server first brick — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship curl-runner studio as a signed, notarized, local-first macOS download with a first-run walkthrough, and start `curl-runner-server` with `GET /health` deployed through GHCR and Coolify.

**Architecture:** A new Axum crate builds into a static musl image that a GitHub workflow pushes to GHCR and deploys through a Coolify webhook. The studio gains a compile-time `server` Cargo feature, off by default and in every distributed build. With it off, the app wires a `NoServer` backend, server UI shows "Coming soon" or is hidden, and first launch is local-only. The release workflow then bundles, signs, notarizes and uploads a universal `.dmg`, the Homebrew tap picks it up, and the website links to it.

**Tech Stack:** Rust 1.99.1 (edition 2024), Axum on Tokio, GPUI-kit 0.8.1, Docker Buildx, GitHub Actions, `codesign`, `notarytool`, `hdiutil`, Homebrew casks, Next.js 16 docs site (vitest).

**Spec:** `docs/superpowers/specs/2026-11-01-studio-release-design.md`

## Global Constraints

- `AGENTS.md`: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` pass before every commit. Every change ships with tests. Conventional Commits, one line.
- Public repo: no Apple ID, team name or team ID, certificate names, or employer or customer details in code, docs, commits or PRs. Signing identities are discovered at runtime and secrets live only in GitHub Actions secrets.
- The mock backend (`MockBackend`, `fixtures`) and the Developer menu are never compiled into a build without the `server` feature, except under `cfg(test)`.
- Server image: `ghcr.io/alexvcasillas/curl-runner-server`, tags `latest` (on `main`) and `sha-<short>`, platforms `linux/amd64` and `linux/arm64`.
- `/health` returns `200` with exactly `{"status":"ok","version":"<workspace version>"}`.
- Server binds `0.0.0.0:$PORT`, default `8080`.
- Coolify compose file: `docker-compose.yaml` at the repo root. Image tag `${SERVER_IMAGE_TAG:-latest}`. External network `coolify`.
- Deploy secrets: `COOLIFY_WEBHOOK_SERVER`, `COOLIFY_TOKEN`.
- Signing secrets: `STUDIO_CERT_P12_BASE64`, `STUDIO_CERT_PASSWORD`, `APPLE_API_KEY_ID`, `APPLE_API_ISSUER_ID`, `APPLE_API_KEY_P8`.
- DMG asset name: `curl-runner-studio-<version>.dmg`. App bundle: `curl-runner studio.app`. Bundle ID `com.curl-runner.studio`, macOS 12.0+.
- Walkthrough collection: `~/curl-runner/getting-started/health.yaml`, never overwritten.
- Walkthrough request URL: `https://api.curl-runner.com/health`.
- "Coming soon" copy names the feature and says it needs the curl-runner server. Example, verbatim, for Share: "Share links need the curl-runner server. Until then, share the YAML files through git."
- User-facing copy (studio and website) follows `/alex` and `/unslop`: no em dashes, plain words.
- Studio look: design-system tokens only, dark theme. Mockups: `LocalFirst`, `WelcomeLocal`, `TourRun`, `TourResult` on https://claude.ai/artifact/EeLYjq1iRGw7SwAWSbvnps.
- The studio and CLI must agree: the walkthrough's `health.yaml` passes `curl-runner validate`.

## Review Focus

1. **A fresh Mac with no network, or the server down, runs the walkthrough:** the result step says what happened in plain words, offers "Try again" and "Finish anyway", and keeps the file. Test in Task 8.
2. **A v1 source-build user upgrades:** stored `Connection::Online` plus received copies and `mock-backend.json` in app support. The release build opens Welcome in local mode, shows no received copies or Acme data, and deletes nothing. Test in Task 5.
3. **A `curl-runner://` link opens a release build cold:** no sign-in screen, no crash, and a "Coming soon" notice. Test in Task 6.
4. **`~/curl-runner/getting-started` already exists with the user's edits:** the walkthrough opens it and only writes `health.yaml` if it's missing. Test in Task 8.
5. **The latest GitHub release has no studio `.dmg`** (older releases, or a failed notarization): `/downloads` still renders with the CLI table and the studio section falls back to the GitHub releases link. Test in Task 12.

---

## Shipping order (from the spec)

| PR | Tasks | Merges when |
| --- | --- | --- |
| 1. Server | 1–3 | First. The owner then makes the GHCR package public, configures Coolify (auto-deploy on push off), adds the deploy secrets and points DNS. `https://api.curl-runner.com/health` must answer before PR 3 ships in a release. |
| 2. `diff.exclude` fix | 4 | Any time. |
| 3. Studio local-first | 5–8 | After PR 1 is live. |
| 4. Packaging and release | 9–10 | After PR 3. The owner adds the signing secrets first. |
| 5. Homebrew cask (tap repo) | 11 | After the first release with a `.dmg`. |
| 6. Website | 12 | Only after that release exists. |

Pushes, PRs and merges are owner-approved steps. Stop and ask before each.
After the owner verifies a PR by hand, ask whether to add a changeset bump (`AGENTS.md`).

---

### Task 1: `curl-runner-server` crate with `/health`

**Files:**
- Create: `crates/curl-runner-server/Cargo.toml`, `crates/curl-runner-server/src/main.rs`, `crates/curl-runner-server/src/lib.rs`
- Modify: `Cargo.toml` (workspace `members`)
- Test: `crates/curl-runner-server/src/lib.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Produces:
  - `pub fn app() -> axum::Router`
  - `pub async fn serve(listener: tokio::net::TcpListener, shutdown: impl Future<Output = ()> + Send + 'static) -> std::io::Result<()>`
  - `pub fn port_from_env(value: Option<&str>) -> u16` (default `8080`, invalid values fall back to `8080`)
  - `pub async fn healthcheck(port: u16) -> bool`, used by the `healthcheck` subcommand (Task 2)
  - Binary `curl-runner-server`: no args serves; `curl-runner-server healthcheck` exits `0` if `GET http://127.0.0.1:$PORT/health` returns `200`, else `1`

- [ ] **Step 1: Write the failing tests** (drive the router with `tower::ServiceExt::oneshot`)

```rust
#[tokio::test] async fn health_returns_ok_and_version() // 200, content-type application/json, body == {"status":"ok","version":env!("CARGO_PKG_VERSION")}
#[tokio::test] async fn health_allows_any_origin()      // Origin: https://example.com → access-control-allow-origin: *
#[tokio::test] async fn unknown_path_is_404()           // GET /nope → 404
#[tokio::test] async fn post_health_is_405()            // POST /health → 405
#[test] fn port_defaults_to_8080()                      // None, Some(""), Some("abc") → 8080; Some("9000") → 9000
#[tokio::test] async fn healthcheck_true_when_serving_false_when_not() // serve on an ephemeral port → true; unused port → false
#[tokio::test] async fn shutdown_finishes_in_flight_request() // start a slow test-only route, trigger shutdown mid-request, request still gets 200
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p curl-runner-server`
Expected: compile errors (crate items missing), then FAIL.

- [ ] **Step 3: Implement**

- Dependencies: `axum`, `tokio` (`rt-multi-thread`, `macros`, `net`, `signal`), `tower-http` (`cors`, `trace`), `tracing` and `tracing-subscriber`. Dev: `tower`, `http-body-util`. `publish = false`, version and edition from the workspace.
- `healthcheck` uses a plain `tokio::net::TcpStream` plus a hand-written HTTP/1.1 `GET`, so the image needs no HTTP client.
- `main` logs one line per request to stdout. It shuts down on SIGTERM and Ctrl-C through `axum::serve(...).with_graceful_shutdown`.

- [ ] **Step 4: Run tests, fmt, clippy**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p curl-runner-server`
Expected: all pass.

- [ ] **Step 5: Commit** — `feat(server): add curl-runner-server with a health endpoint`

### Task 2: Image, compose files and the build-and-deploy workflow

**Files:**
- Create: `crates/curl-runner-server/Dockerfile`, `.dockerignore`, `docker-compose.yaml`, `docker-compose.preview.yml`, `.github/workflows/server.yml`

**Interfaces:**
- Consumes: the binary and the `healthcheck` subcommand from Task 1.

- [ ] **Step 1: Dockerfile**

Multi-stage. The builder is `rust:1.99.1` with musl targets. It builds `cargo build --release --locked -p curl-runner-server --target <arch>-unknown-linux-musl`, choosing the arch from `TARGETARCH`. The final stage is `scratch` or a distroless static image with only the binary, running as a non-root user, `EXPOSE 8080`, `ENTRYPOINT ["/curl-runner-server"]`. The build context is the repo root, and `.dockerignore` excludes `target/`, `node_modules/`, `packages/` and `.worktrees/`.

- [ ] **Step 2: Verify the image locally**

Run: `docker build -f crates/curl-runner-server/Dockerfile -t crs:dev . && docker run --rm -d -p 8080:8080 --name crs crs:dev && sleep 1 && curl -s localhost:8080/health && docker exec crs /curl-runner-server healthcheck; echo "exit $?"; docker rm -f crs && docker image inspect crs:dev --format '{{.Size}}'`
Expected: `{"status":"ok","version":"…"}`, then `exit 0`, and an image under 30 MB.

- [ ] **Step 3: `docker-compose.yaml`**

One service, `server`:
- `image: ghcr.io/alexvcasillas/curl-runner-server:${SERVER_IMAGE_TAG:-latest}`;
- `environment: { PORT: ${PORT:-8080}, RUST_LOG: ${RUST_LOG:-info} }`;
- `networks: [coolify]`;
- healthcheck `["CMD", "/curl-runner-server", "healthcheck"]` with interval 10s, timeout 5s, retries 5, `start_period: 10s`;
- `restart: unless-stopped`;
- `networks: { coolify: { external: true } }`.

A header comment says Coolify runs this file, images come prebuilt from GHCR, and self-hosters without Coolify create the network with `docker network create coolify` or drop the `networks` keys.

- [ ] **Step 4: `docker-compose.preview.yml`**

Builds the Dockerfile locally (`image: curl-runner-server-preview`), maps `8080:8080` and doesn't use the external network. The header comment gives the command.

Run: `docker compose -f docker-compose.preview.yml up -d --build && sleep 2 && curl -s localhost:8080/health && docker compose -f docker-compose.preview.yml down`
Expected: the health JSON.

- [ ] **Step 5: `.github/workflows/server.yml`** (the same shape as the owner's ragnaonline-v2 `ci.yml`, which the spec describes)

- **Triggers:**
  - `pull_request` and `push` to `main`, both with `paths:` `crates/curl-runner-server/**`, `Cargo.toml`, `Cargo.lock`, `docker-compose.yaml` and `.github/workflows/server.yml`;
  - `workflow_dispatch`.
- **Jobs:**
  - `test`: `cargo test -p curl-runner-server` and `cargo clippy -p curl-runner-server -- -D warnings`.
  - `build`: needs `test`. It uses `docker/setup-qemu-action`, `docker/setup-buildx-action`, `docker/login-action` (skipped on PRs) and `docker/metadata-action` (`type=raw,value=latest,enable={{is_default_branch}}` and `type=sha,format=short,prefix=sha-`). Then `docker/build-push-action` with `file: crates/curl-runner-server/Dockerfile`, `platforms: linux/amd64,linux/arm64`, `push: ${{ github.event_name != 'pull_request' }}` and `cache-from/to: type=gha`. Permissions: `contents: read`, `packages: write`.
  - `deploy`: needs `build`. It runs only when `github.event_name != 'pull_request'` and `github.ref == 'refs/heads/main'`, with `concurrency: { group: deploy-server, cancel-in-progress: false }`. One step: `curl --request POST "$COOLIFY_WEBHOOK_SERVER" --header "Authorization: Bearer $COOLIFY_TOKEN" --fail --silent --show-error --retry 3 --retry-delay 5`, with both values from secrets.
- Pin action versions to the majors already used in this repo's other workflows, where they appear.

- [ ] **Step 6: Lint the workflow**

Run: `actionlint .github/workflows/server.yml` (install with `brew install actionlint` if it's missing)
Expected: no findings.

- [ ] **Step 7: Commit** — `ci(server): build the image on GHCR and deploy through the Coolify webhook`

### Task 3: Self-hosting docs page

**Files:**
- Create: `packages/docs/content/docs/self-hosting.mdx`
- Modify: the docs navigation meta file that lists pages (find the `meta.json` next to `quick-start.mdx`)
- Test: `packages/docs/tests/` (extend the existing test that asserts every docs page has a description ≤ 160 characters, if there is one; otherwise add a page-render test like the other docs tests)

- [ ] **Step 1: Write the page**

Mark it as early: only `/health` today, and accounts and sync are coming. Cover `docker compose up` with `docker-compose.yaml` (and the `coolify` network note), the `PORT` and `RUST_LOG` settings, `SERVER_IMAGE_TAG` for pinning, and checking `curl localhost:8080/health`. Follow `/alex` and `/unslop`.

- [ ] **Step 2: Run the docs tests and lint**

Run: `pnpm --filter docs test && pnpm --filter docs lint`
Expected: pass.

- [ ] **Step 3: Commit** — `docs: self-hosting the curl-runner server`

### Task 4: `curl-runner diff a b` honours `diff.exclude` and match rules

**Files:**
- Modify: `crates/curl-runner/src/diff_mode.rs:122-182` (`run_subcommand`)
- Test: `crates/curl-runner/tests/run_cli.rs` (or a new `crates/curl-runner/tests/diff_cli.rs`, following `run_cli.rs`'s harness)

**Interfaces:**
- Consumes: `curl_runner_core::config::load_config_file(cwd)`, the YAML file's own global config (load it the same way the run path does before `handle_diff_mode`), `merge_global_configs`, and `BaselineManager::merge_config(Some(&dc), Some(&DiffSetting::Bool(true)))` (`crates/curl-runner/src/diff_mode.rs:51-52` shows the post-run use).

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn diff_subcommand_ignores_excluded_paths()
// two baselines under __baselines__ that differ only at body.timestamp;
// curl-runner.yaml (config file) has diff.exclude: ["body.timestamp"];
// `curl-runner diff a b req.yaml` exits 0 and prints no changed request.
#[test] fn diff_subcommand_reads_exclude_from_the_yaml_global()
// same, with the exclude in the YAML file's `global.diff.exclude` instead.
#[test] fn diff_subcommand_flags_override_config_dir()
// config sets diff.dir: other; --diff-dir __baselines__ wins.
#[test] fn diff_subcommand_still_reports_real_changes()
// a difference outside the excluded path → exit 1.
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p curl-runner --test run_cli diff_subcommand`
Expected: `ignores_excluded_paths` and `reads_exclude_from_the_yaml_global` FAIL with exit 1.

- [ ] **Step 3: Implement**

Build `GlobalDiffConfig` from the merged config file plus the YAML global `diff`, then apply the CLI `--diff-dir` and `--diff-output` on top. Pass that to `DiffOrchestrator::new` and to `BaselineManager::merge_config` in place of the hard-coded `DiffConfig`.

- [ ] **Step 4: Run tests, fmt, clippy**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p curl-runner`
Expected: pass.

- [ ] **Step 5: Commit** — `fix(diff): honour diff.exclude and match rules in curl-runner diff`

### Task 5: The `server` feature, `NoServer`, and local-first connection state

**Files:**
- Modify: `crates/curl-runner-studio/Cargo.toml` (add `[features] server = []`, not default)
- Create: `crates/curl-runner-studio/src/features.rs`, `crates/curl-runner-studio/src/model/backend/no_server.rs`
- Modify:
  - `crates/curl-runner-studio/src/model/backend/mod.rs`: gate `mod mock`, `fixtures` and `pub use mock::…` with `#[cfg(any(test, feature = "server"))]`; add `BackendError::Unavailable`.
  - `crates/curl-runner-studio/src/ui/app.rs:332`: choose the backend, and gate the Developer menu and its actions on `feature = "server"`, in addition to `debug_assertions`.
  - `crates/curl-runner-studio/src/ui/workspace/routing.rs`: `connection()` and `connection_text()`.
  - `.github/workflows/tests.yml`: add macOS steps `cargo clippy -p curl-runner-studio --all-targets --features server -- -D warnings` and `cargo test -p curl-runner-studio --features server`.
  - `scripts/studio-bundle.sh`: pass `--features server` only when `--server` is given.

**Interfaces:**
- Produces:
  - `pub const SERVER: bool = cfg!(feature = "server");` in `features.rs`. UI code branches on `features::SERVER`; `cfg` attributes are only for what must not be compiled in.
  - `pub struct NoServer;` with `impl Backend for NoServer`. Every async call resolves to `Err(BackendError::Unavailable)`, `account()` is `None`, and `sync_status()` is idle/none. `dev()` is `None`.
  - `pub fn app_backend(base: Option<&Path>) -> Arc<dyn Backend>` in `ui/app.rs`. It returns `NoServer` without the feature and `MockBackend::open(base, MOCK_LATENCY)` with it. It stays the only place the mock is named.
  - `BackendError::Unavailable`, with copy "This needs the curl-runner server, which is coming soon."
  - `Workspace::connection()`: with `!SERVER`, any stored value maps to `Some(Connection::Offline)`, and `None` stays `None` (first launch). Nothing is written back.
  - `Workspace::connection_text()`: with `!SERVER` it returns `("text-tertiary", "Working locally")`.

- [ ] **Step 1: Write the failing tests** (they run without the feature; the existing server-UI tests that need `MockBackend` behaviour visible in the UI get `#[cfg(feature = "server")]`)

```rust
#[test] fn no_server_refuses_every_call()                        // test_connection, sign_in, sync, resolve_link… → Err(Unavailable); account() None
#[test] fn app_backend_without_feature_is_no_server()            // app_backend(None).test_connection() → Err(Unavailable)
#[gpui_kit::test] fn stored_online_connection_opens_welcome_locally()
// StateStore with Connection::Online + a received copy + mock-backend.json in base:
// first_launch shows no sign-in/first-launch screen, Welcome shows "Working locally",
// no received copies are listed, and all three are still on disk afterwards.
#[test] fn mock_is_only_named_in_app_backend()                   // extend `ui_never_imports_mock_backend` to also allow only `app_backend`
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p curl-runner-studio no_server app_backend stored_online mock_is_only`
Expected: compile errors, then FAIL.

- [ ] **Step 3: Implement** the items in Interfaces.

Received copies (`model/received.rs`, `workspace.rs:677`) aren't loaded when `!SERVER`.

- [ ] **Step 4: Run both builds**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo clippy -p curl-runner-studio --all-targets --features server -- -D warnings && cargo test -p curl-runner-studio --features server`
Expected: all pass. Then confirm the mock isn't in a release binary.

Run: `cargo build --release -p curl-runner-studio && ! strings target/release/curl-runner-studio | grep -q "Acme"`
Expected: exit 0.

- [ ] **Step 5: Commit** — `feat(studio): server feature switch, off by default, with a no-server backend`

### Task 6: "Coming soon" across the app

**Files:**
- Create: `crates/curl-runner-studio/src/ui/coming_soon.rs`
- Modify: `ui/welcome.rs`, `ui/title_bar.rs`, `ui/palette.rs`, `ui/app.rs` (File menu, `on_open_urls`), `ui/open_shared.rs` (`link_handler`), `ui/settings_view.rs`, `ui/collection_settings.rs`, `ui/sidebar.rs`, `ui/sync.rs`, `ui/workspace.rs`, `ui/workspace/routing.rs`

**Interfaces:**
- Consumes: `features::SERVER` (Task 5).
- Produces:
  - `pub fn soon_pill() -> impl IntoElement`: the yellow "Coming soon" pill from mockup `LocalFirst`, using design-system warning tokens.
  - `pub fn soon_tooltip(title: &str, body: &str) -> AnyView` for disabled controls.
  - `pub fn soon_card(title: &str, body: &str) -> impl IntoElement` for Settings.

Behaviour with `!SERVER`, per the spec's table:

| Surface | Behaviour |
| --- | --- |
| Welcome "Open a shared link" card | Disabled (dashed border), `soon_pill`, body "Open a collection someone shared with you. Arrives with the curl-runner server." |
| Title-bar Share | Disabled, with `soon_tooltip("Sharing", "Share links need the curl-runner server. Until then, share the YAML files through git.")` |
| Palette `OpenSharedLink`, File › "Open Shared Link…" | Not listed. |
| `curl-runner://` link | The app opens, and the workspace shows a notice: "Shared links need the curl-runner server, which is coming soon." No sign-in screen. |
| Settings › Accounts & sync | One `soon_card` replaces the Server, Account and Sync cards. |
| Collection settings › Sharing | Not listed. |
| Sidebar chip, title-bar Offline pill, sync loop | Chip "Working locally" with `soon_pill` text "Sync soon"; no Offline pill; `start_sync` is a no-op. |
| Read-only view, received copies, conflict view | Unreachable; their entry points aren't rendered. |

- [ ] **Step 1: Write the failing tests** (GPUI tests, without the feature)

```rust
#[gpui_kit::test] fn welcome_shared_link_card_is_disabled_with_coming_soon()
#[gpui_kit::test] fn share_button_is_disabled_and_explains()
#[gpui_kit::test] fn palette_has_no_open_shared_link()
#[gpui_kit::test] fn cold_link_shows_coming_soon_notice_not_sign_in()   // Review Focus 3
#[gpui_kit::test] fn settings_accounts_page_is_one_coming_soon_card()
#[gpui_kit::test] fn collection_settings_has_no_sharing_page()
#[gpui_kit::test] fn sidebar_says_working_locally_and_never_syncs()     // Spy backend records zero sync calls
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo test -p curl-runner-studio coming_soon welcome_shared share_button palette_has cold_link settings_accounts collection_settings_has sidebar_says`
Expected: FAIL.

- [ ] **Step 3: Implement** the table above. Gate on `features::SERVER`. Don't delete any server UI code.

- [ ] **Step 4: Run both builds** (the Task 5 Step 4 command, without the `strings` check)

Expected: all pass. Existing tests for these surfaces pass under `--features server`.

- [ ] **Step 5: Screenshot check** against mockup `WelcomeLocal`, using the existing dev hooks (`CURL_RUNNER_STUDIO_SHOW`). Compare the Welcome and Settings screens with the canvas. Fix any drift from the design system.

- [ ] **Step 6: Commit** — `feat(studio): mark server features as coming soon in local builds`

### Task 7: Local-first first launch

**Files:**
- Create: `crates/curl-runner-studio/src/ui/first_launch.rs`
- Modify: `ui/workspace/routing.rs` (`first_launch`), `ui/workspace.rs`, `ui/state.rs` (no new key; uses `Connection::Offline` as "local")

**Interfaces:**
- Consumes: `soon_pill` (Task 6), `features::SERVER`, `StateStore::{connection, set_connection}`.
- Produces: `pub struct FirstLaunchView` and `pub enum FirstLaunchEvent { GetStarted }`. `Workspace::first_launch` opens `FirstLaunchView` when `!SERVER && connection().is_none()`, and `SignInView` otherwise (unchanged under the feature). `GetStarted` stores `Connection::Offline` and opens Welcome.

Content, verbatim from mockup `LocalFirst`:
- Title: "Welcome to curl-runner studio".
- Lede: "Write requests as YAML, run them as tests and read the results. Everything stays on this Mac."
- Options:
  - "On this Mac", selected: "Plain YAML files in folders you pick. The same files the CLI runs. No account needed."
  - "curl-runner Cloud", disabled, `soon_pill`: "Sync collections and share them with the people who call your API."
  - "Self-hosted", disabled, `soon_pill`: "The same server, open source, on your own infrastructure."
- Button: "Get started".
- Footnote: "Sync and sharing need the curl-runner server, which we're building now." with the link "Follow along on GitHub" to `https://github.com/alexvcasillas/curl-runner`.

- [ ] **Step 1: Write the failing tests**

```rust
#[gpui_kit::test] fn first_launch_shows_local_first_once()      // no stored connection → FirstLaunchView; after GetStarted, a new Workspace on the same store shows Welcome
#[gpui_kit::test] fn cloud_and_self_hosted_are_disabled()       // clicking them leaves "On this Mac" selected
#[gpui_kit::test] fn get_started_stores_local_and_opens_welcome() // store.connection() == Some(Connection::Offline)
```

- [ ] **Step 2: Run them and watch them fail.** Run: `cargo test -p curl-runner-studio first_launch get_started cloud_and_self` Expected: FAIL.

- [ ] **Step 3: Implement.** Reuse `sign_in.rs`'s option-card layout and tokens. Don't fork the style.

- [ ] **Step 4: Run both builds** (the Task 6 Step 4 command), then a screenshot check against `LocalFirst`.

- [ ] **Step 5: Commit** — `feat(studio): local-first first launch`

### Task 8: First-run walkthrough

**Files:**
- Create: `crates/curl-runner-studio/src/model/tour.rs`, `crates/curl-runner-studio/src/ui/tour.rs`
- Modify: `ui/welcome.rs` (card), `ui/workspace.rs`, `ui/palette.rs` (`PaletteAction::Walkthrough`, "Run the walkthrough"), `ui/app.rs` (Help menu item "Walkthrough"), `ui/state.rs` (persisted tour status)

**Interfaces:**
- Consumes: the run pipeline the Run button uses (`ui/runs.rs`), `Workspace::open_folder`, `Workspace::open_file`.
- Produces:
  - `pub const HEALTH_YAML: &str`, which is exactly the spec's YAML block.
  - `pub fn getting_started_dir(home: &Path) -> PathBuf`, returning `home/curl-runner/getting-started`.
  - `pub fn ensure_getting_started(dir: &Path) -> std::io::Result<PathBuf>`. It creates the dir if missing and writes `health.yaml` only if absent. It returns the file path.
  - `pub enum TourStatus { NotStarted, Done, Skipped }`, stored by `StateStore::{tour_status, set_tour_status}`.
  - `pub enum TourStep { Create, Run, Result }`.
  - `TourView` is an overlay anchored as in mockups `TourRun` and `TourResult`.

Behaviour:
- Welcome shows the "Run your first request" card (mockup `WelcomeLocal`) while the status is `NotStarted`.
- **Step 1, Create:** `ensure_getting_started`, then open the folder and `health.yaml`.
- **Step 2, Run:** the callout from mockup `TourRun`. "Run" in the callout or the Run button runs the request.
- **Step 3, Result:**
  - On a pass, it shows mockup `TourResult`'s copy with the buttons "New request" (opens the New request dialog) and "Done".
  - On a failure, it says what happened in plain words. For a connection error: "Couldn't reach api.curl-runner.com. Check your connection and try again." For a failed check, it names the check. It offers "Try again" and "Finish anyway".
- "Skip walkthrough" sets `Skipped`, and "Done" and "Finish anyway" set `Done`. The palette and Help entries restart the walkthrough at step 1 whatever the status.

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn health_yaml_passes_the_cli_validator()        // curl_runner_core validator on HEALTH_YAML → no errors
#[test] fn ensure_creates_dir_and_file()
#[test] fn ensure_never_overwrites_existing_health_yaml() // pre-write "edited" → content unchanged (Review Focus 4)
#[test] fn ensure_keeps_other_files_in_existing_dir()
#[gpui_kit::test] fn welcome_shows_card_until_done_or_skipped()
#[gpui_kit::test] fn tour_pass_reaches_result_step()       // HEALTH_YAML with url pointed at a testkit server returning {"status":"ok"}
#[gpui_kit::test] fn tour_connection_failure_explains_and_offers_retry() // url → 127.0.0.1:<unused port>; copy contains "Couldn't reach"; file still on disk (Review Focus 1)
#[gpui_kit::test] fn palette_replays_walkthrough()
```

To point the tour at testkit in tests, give `TourView` the request URL as a constructor argument that defaults to the `HEALTH_YAML` URL. Don't add an env hook.

- [ ] **Step 2: Run them and watch them fail.** Run: `cargo test -p curl-runner-studio tour ensure_ health_yaml welcome_shows palette_replays` Expected: FAIL.

- [ ] **Step 3: Implement.**

- [ ] **Step 4: Run both builds**, then screenshot checks against `WelcomeLocal`, `TourRun` and `TourResult`. Also run once by hand against the live `https://api.curl-runner.com/health`, after PR 1 is deployed.

- [ ] **Step 5: Commit** — `feat(studio): first-run walkthrough with a health check request`

### Task 9: Icon and universal bundle

**Files:**
- Create: `scripts/studio-icon.sh`, `crates/curl-runner-studio/assets/AppIcon-1024.png`, `crates/curl-runner-studio/assets/AppIcon.icns`
- Modify: `scripts/studio-bundle.sh`

**Interfaces:**
- Produces: `scripts/studio-bundle.sh [--release] [--universal] [--server] [repo-root]`.
  - It copies `AppIcon.icns` into `Contents/Resources/` and sets `CFBundleIconFile` to `AppIcon`.
  - `--universal` builds `aarch64-apple-darwin` and `x86_64-apple-darwin` and joins them with `lipo -create`.
  - `--server` adds `--features server`.

- [ ] **Step 1: `studio-icon.sh`**

It renders the `TerminalMark` paths (`polyline 4 17 10 11 4 5`, `line 12 19 20 19`, stroke-width 2, round caps and joins, `viewBox 0 0 24 24`, from `packages/docs/components/site/SiteHeader.tsx`) in white on a dark rounded tile matching `packages/docs/public/apple-icon-dark.png`.
- The output is a 1024×1024 PNG on the macOS icon grid: 824×824 tile, 100px inset, ~185px corner radius.
- Then `sips` makes the `iconset` sizes and `iconutil -c icns` builds `AppIcon.icns`.
- Use whatever renderer is available (`rsvg-convert`, or `qlmanage`/`sips` on a temp SVG), and fail clearly if none is.

Run: `scripts/studio-icon.sh && file crates/curl-runner-studio/assets/AppIcon.icns`
Expected: "Mac OS X icon".

- [ ] **Step 2: View the PNG** (open it with the Read tool) and compare it with `apple-icon-dark.png`. Ask the owner to look at it before committing.

- [ ] **Step 3: Extend `studio-bundle.sh`** with the flags above.

Run: `scripts/studio-bundle.sh --release --universal && lipo -archs "target/studio/curl-runner studio.app/Contents/MacOS/curl-runner-studio" && plutil -lint "target/studio/curl-runner studio.app/Contents/Info.plist"`
Expected: `x86_64 arm64`, then `OK`.

- [ ] **Step 4: Commit** — `feat(studio): app icon and universal release bundle`

### Task 10: Sign, notarize and ship the `.dmg` from the release workflow

**Files:**
- Create: `scripts/studio-dmg.sh`, `crates/curl-runner-studio/entitlements.plist` (hardened runtime; network client; no other entitlements unless the build proves they're needed)
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: `studio-bundle.sh --release --universal` (Task 9).
- Produces: release asset `curl-runner-studio-<version>.dmg` on tag `@curl-runner/cli@<version>`, and `workflow_dispatch` input `studio_tag` (an existing tag) that rebuilds and re-uploads only the studio.

- [ ] **Step 1: `scripts/studio-dmg.sh <app> <out.dmg>`**

`hdiutil create` a compressed UDZO image containing the app and an `/Applications` symlink, with volume name "curl-runner studio".

Run: `scripts/studio-dmg.sh "target/studio/curl-runner studio.app" /tmp/t.dmg && hdiutil verify /tmp/t.dmg`
Expected: "checksum … is VALID".

- [ ] **Step 2: The `studio` job in `release.yml`**

- `runs-on: macos-latest`.
- `needs: [meta, upload]`, with `if: needs.meta.outputs.build == 'true' && needs.upload.result == 'success'`. It runs after the CLI is published and never blocks it.
- A second trigger path runs only this job when `inputs.studio_tag` is set.
- **Steps:**
  1. Check out (the tag, when `studio_tag` is set).
  2. Import `STUDIO_CERT_P12_BASE64` and `STUDIO_CERT_PASSWORD` into a temporary keychain.
  3. Run `studio-bundle.sh --release --universal`.
  4. Sign: `codesign --force --options runtime --timestamp --entitlements … --sign "$IDENTITY"`. `IDENTITY` is read with `security find-identity -v -p codesigning` and the first "Developer ID Application" match; it's never written in the repo.
  5. `codesign --verify --deep --strict`.
  6. Run `studio-dmg.sh`, then sign the dmg.
  7. Notarize: `xcrun notarytool submit --wait --key <p8 file> --key-id $APPLE_API_KEY_ID --issuer $APPLE_API_ISSUER_ID`.
  8. `xcrun stapler staple`, then `spctl -a -t open --context context:primary-signature -v` on the dmg.
  9. Upload with `gh release upload "$TAG" curl-runner-studio-<version>.dmg --clobber`.
  10. Delete the temporary keychain in an `if: always()` step.
- Never `echo` secrets. Write the `.p8` to `$RUNNER_TEMP` and delete it.

- [ ] **Step 3: Lint.** Run: `actionlint .github/workflows/release.yml` Expected: no findings.

- [ ] **Step 4: Owner-approved dry run.** After the owner adds the secrets, ask them to run `workflow_dispatch` with `studio_tag` set to the current release tag. Expected: the job is green, the dmg is attached, and a downloaded copy opens on a Mac without a Gatekeeper prompt beyond the standard "downloaded from the internet".

- [ ] **Step 5: Commit** — `ci(release): sign, notarize and attach the studio dmg`

### Task 11: Homebrew cask (repo `alexvcasillas/homebrew-tap`)

**Files (in the tap repo):**
- Create: `Casks/curl-runner-studio.rb`, `scripts/update-curl-runner-studio.sh`, `.github/workflows/update-curl-runner-studio.yml`

**Interfaces:**
- Consumes: the `curl-runner-studio-<version>.dmg` release asset.

- [ ] **Step 1:** Mirror `scripts/update-curl-runner.sh` and `update-curl-runner.yml`:
  - Run hourly, on `workflow_dispatch` with `version`, and on pushes that touch the cask or its tooling (validate only).
  - Read the latest release. Skip when it has no studio dmg.
  - Download the dmg and compute `sha256`. Regenerate the cask.
  - Commit, then `brew style --cask`, `brew audit --cask --strict` and `brew install --cask`, then push.
- The cask:
  - `app "curl-runner studio.app"`;
  - `depends_on macos: ">= :monterey"`;
  - `zap trash:` the app-support folder `StateStore::app_support` uses (read the path from `ui/state.rs`);
  - `url` on the `@curl-runner/cli@#{version}` tag with the `%40` and `%2F` encoding the formula uses.

- [ ] **Step 2:** Run `workflow_dispatch` once (owner-approved) after the first release with a dmg. Expected: green, and `brew install --cask alexvcasillas/tap/curl-runner-studio` installs the app.

- [ ] **Step 3: Commit (tap repo)** — `feat: curl-runner-studio cask, updated from releases`

### Task 12: Website download entry points

**Files:**
- Modify:
  - `packages/docs/lib/github-release.ts`: parse the studio asset.
  - `packages/docs/app/(marketing)/downloads/page.tsx`
  - `packages/docs/app/(marketing)/page.tsx`: studio section, lines 249–305.
  - `packages/docs/lib/studio.ts`
  - `packages/docs/content/docs/features/studio.mdx`
  - `packages/docs/content/blog/testing-in-the-agent-era.mdx`: line 91, plus `updated:` in the frontmatter.
- Test: `packages/docs/tests/marketing/downloads.test.tsx`, the landing test, and `tests/lib/github-release.test.ts` (create it if it doesn't exist).

**Interfaces:**
- Produces: `LatestRelease.studio?: { fileName: string; url: string; size: number }`, matched by `/^curl-runner-studio-(\d+\.\d+\.\d+\S*)\.dmg$/`. `STUDIO_CASK_COMMAND = 'brew install --cask alexvcasillas/tap/curl-runner-studio'` in `lib/studio.ts`, replacing `STUDIO_BUILD_COMMAND` in user-facing copy.

- [ ] **Step 1: Write the failing tests**

```ts
test('parses the studio dmg from release assets')
test('studio is undefined when the release has no dmg')
test('downloads page shows the studio dmg link, the cask command and macOS 12+')
test('downloads page falls back to the releases link when there is no dmg')   // Review Focus 5
test('landing studio section links to the download, not to building from source')
```

- [ ] **Step 2: Run them and watch them fail.** Run: `pnpm --filter docs test -- downloads github-release marketing` Expected: FAIL.

- [ ] **Step 3: Implement.**
- Rewrite the copy that says "not distributed yet" or "build it from source" in `lib/studio.ts`, the landing page and `studio.mdx`. Mention macOS 12+ and that sync and sharing are coming.
- In the blog post, replace "It's an early macOS preview for now, built from source." with a sentence linking to `/downloads`, and set `updated` to the merge day.
- Follow `/alex` and `/unslop`.

- [ ] **Step 4: Run tests, lint, build.** Run: `pnpm --filter docs test && pnpm --filter docs lint && pnpm --filter docs build` Expected: pass.

- [ ] **Step 5: Commit** — `feat(docs): download the studio from the site`

---

## Unresolved questions

- None blocking. These are owner steps, in the order they're needed:
  - GHCR package public;
  - the Coolify stack with auto-deploy off;
  - `COOLIFY_WEBHOOK_SERVER` and `COOLIFY_TOKEN`;
  - DNS for `api.curl-runner.com`;
  - the signing secrets;
  - approval of the icon in Task 9.
