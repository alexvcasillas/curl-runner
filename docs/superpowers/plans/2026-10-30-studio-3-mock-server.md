# curl-runner studio, plan 3: the mocked server

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every server-backed board work end to end against an in-process mock:
- sign-in and server choice;
- sync status, offline and conflicts;
- sharing and share links;
- opening a shared link as a customer;
- the read-only customer view.

**Architecture:**
- **One seam.** The studio talks to "the server" only through `model::backend::Backend`, an async, object-safe trait.
- **The mock.** v1 ships `MockBackend`: stateful, JSON-persisted under Application Support, with fixture data and developer toggles. Views hold an `Arc<dyn Backend>` and never name `MockBackend`.
- **Real files stay real.** Duplicating a shared collection and the local side of a sync conflict use real files.

**Tech Stack:** as plan 2. No new crates; `futures` is already a macOS dependency, and the model must stay portable. Use `futures` only if it is already a portable dependency; otherwise use boxed futures from `std`.

**Spec:** `docs/superpowers/specs/2026-10-29-curl-runner-studio-design.md`. The relevant sections are "Real vs mocked", "The backend seam" and boards 4, 6, 11, 12, 13 and 16.

**Boards:** in `docs/superpowers/specs/studio-mockups/`:
- `SignIn`
- `Share`
- `OpenShared`
- `ReadOnly`
- `SyncConflict`
- `Settings` (Accounts & sync)
- `Environments` (the "Shared by Acme" rows)
- `Welcome` (shared link and the cloud chip)
- `Main` (title-bar `Shared · 3`, sidebar "Shared with me" and the sync footer)

Render each reference with headless Chrome into `fidelity/ref/`.

## Global Constraints

- **Plans 1 and 2 still apply.** Their Global Constraints hold, including the machine-safety rules and "tests never touch the real Keychain or Application Support".
- **The mock never leaks into views.** No `ui/` file imports `MockBackend` except `ui/app.rs`, which constructs it, and the debug-only dev menu, which reaches it through `Backend::dev()` returning `Option<&dyn DevControls>`.
  - A test greps `src/ui` for `MockBackend` and fails on any other use.
- **Secrets never reach the backend.**
  - No `Backend` method takes a variable value. Share and sync payloads carry only:
    - variable names;
    - descriptions;
    - `required_for_shared`;
    - and non-secret defaults, which are the `.env` (Default) values of non-secret names, only when the owner ticks "Share default".
  - A test feeds a collection with Keychain secrets through every share and sync call and asserts that no secret value appears in any recorded request to the mock.
- **Placeholder data only.** Accounts are `Jamie Rivera <jamie@acme.dev>` (owner, org Acme) and `Maria`, plus `Sam` as the teammate who pushes the conflicting change. The shared fixture is `Shipping API` on `api.example.com`; the server is `https://cloud.curl-runner.com` (fake), and self-hosted is a typed URL.
- **Mock state** lives in `{base}/mock-backend.json`: sign-in, server choice, share links, invites, people with access, received shares, and the dev toggles. A corrupt file falls back to the initial fixture state.
- **The mock behaves like a network.** Every call resolves after a short delay (`MOCK_LATENCY = 120ms`; `0` in tests) and fails with `BackendError::Offline` while the offline toggle is on.
- **Dev menu** (debug builds only): App menu › Developer, with:
  - Offline
  - Conflict on next sync
  - Expire share links
  - Fail next sign-in
  - Reset mock

## Review Focus

1. **Offline mid-flow.** Turning Offline on while the Share dialog or Sync is in flight gives a clear inline error and keeps local work intact. Pinned in Tasks 1 and 3.
2. **Conflict resolution loses nothing.** "Keep both" writes the server copy as `{name}.server.yaml` next to the local file; "Keep mine" and "Take theirs" are exact. Pinned in Task 7.
3. **Read-only really is read-only.** A shared collection's files can't be saved, renamed, deleted or quick-fixed, even through the palette or a keyboard shortcut. Run still works. Pinned in Task 6.
4. **An expired or unknown link** shows the board's error state and never creates a collection. Pinned in Task 5.
5. **No secret reaches the backend** (Global Constraint). Pinned in Tasks 1, 4 and 5.

---

### Task 1: Backend trait and the mock

**Files:**
- Create: `crates/curl-runner-studio/src/model/backend/mod.rs` (the trait and types)
- Create: `crates/curl-runner-studio/src/model/backend/mock.rs`
- Create: `crates/curl-runner-studio/src/model/backend/fixtures.rs`
- Modify: `model/mod.rs`

**Interfaces (Produces):**
- `pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = Result<T, BackendError>> + Send + 'a>>`.
- `pub enum BackendError { Offline, Unauthorized, NotFound, Expired, Conflict(Box<Conflict>), Invalid(String) }`, with a `Display` that gives the boards' copy.
- `pub enum Server { Cloud, SelfHosted { url: String } }`.
- `pub struct Account { name, email, org: Option<String>, role, provider }`.
- `pub struct ConnectionInfo { version, sign_in: SignInMethod, latency_ms }`.
- `pub trait Backend: Send + Sync`:
  - `server(&self) -> Server`
  - `set_server(&self, Server) -> BoxFut<()>`
  - `test_connection(&self, &Server) -> BoxFut<ConnectionInfo>`
  - `sign_in(&self, SignInMethod) -> BoxFut<Account>`
  - `sign_out(&self) -> BoxFut<()>`
  - `account(&self) -> Option<Account>`
  - `sync_status(&self, collection: &CollectionKey) -> SyncStatus`
  - `sync(&self, collection: &CollectionKey, local: &[SyncFile]) -> BoxFut<SyncReport>`
  - `share_settings(&self, &CollectionKey) -> BoxFut<ShareSettings>`
  - `update_share(&self, &CollectionKey, ShareUpdate) -> BoxFut<ShareSettings>`
  - `invite(&self, &CollectionKey, email: &str) -> BoxFut<Person>`
  - `remove_person(&self, &CollectionKey, email: &str) -> BoxFut<()>`
  - `resolve_link(&self, link: &ShareLink) -> BoxFut<SharedCollection>`
  - `fetch_shared(&self, id: &str) -> BoxFut<SharedSnapshot>`
  - `received(&self) -> Vec<ReceivedShare>`
  - `add_received(&self, ReceivedShare) -> BoxFut<()>`
  - `dev(&self) -> Option<&dyn DevControls>`
- `pub struct SyncFile { rel: PathBuf, hash: u64, text: String }`. The text is YAML only: never `.env`, never secrets.
- `pub struct SyncReport { pushed: usize, pulled: Vec<SyncFile>, conflicts: Vec<Conflict> }`.
- `pub struct Conflict { rel, local: String, server: String, server_author: String, server_at }`.
- `pub enum SyncStatus { Off, Synced { at, collections, requests }, Pending(usize), Offline(usize), Conflict(usize) }`.
- `pub struct ShareSettings { link: Option<String>, access: Access /* AnyoneWithLink | InvitedOnly */, expires: Option<Date>, people: Vec<Person>, variables: Vec<SharedVar> }`.
- `pub struct SharedVar { name, description, required, secret, default: Option<String> }`. `default` must be `None` when `secret`.
- `pub struct ShareLink { owner_slug, collection_slug, token }`, with `parse(&str) -> Option<ShareLink>`. It accepts `curl-runner://open/{owner}/{collection}?share={token}` and `https://studio.example.com/s/{collection}-{token}`.
- `SharedCollection { owner_org, name, summary, updated, request_count, variables: Vec<SharedVar> }`.
- `SharedSnapshot { files: Vec<SyncFile> }`.
- `DevControls`:
  - `set_offline(bool)` and `offline()`
  - `conflict_next_sync(bool)`
  - `expire_links(bool)`
  - `fail_next_sign_in(bool)`
  - `reset()`
- `MockBackend::open(base: Option<&Path>, latency: Duration) -> MockBackend`.
- `CollectionKey { id: String /* the CollectionMeta id */, name: String }`.
- Fixtures:
  - The Shipping API snapshot: 12 YAML requests across `rates/`, `labels/`, `tracking/` and `webhooks/`, in valid curl-runner YAML using `${BASE_URL}` and `${API_TOKEN}`. `rates/get-rates.yaml` matches the ReadOnly board.
  - Variables: `BASE_URL` (required, default `https://api.example.com/shipping`), `API_TOKEN` (required, secret) and `ACCOUNT_ID` (optional).

- [ ] **Step 1: Write the failing tests** (all on the mock, with zero latency and a temp base):
  - `sign_in_then_account_persists_across_open`
  - `fail_next_sign_in_fails_once`
  - `offline_fails_every_call_with_offline`
  - `share_link_round_trip_and_invite`
  - `expired_links_resolve_to_expired`
  - `parse_both_link_forms` and `garbage_link_is_none`
  - `sync_pushes_and_reports_pending_counts`
  - `conflict_next_sync_returns_one_conflict_with_sam_as_author`. The server copy is the local text with one value changed.
  - `fixture_snapshot_parses_as_curl_runner_yaml`: every fixture file passes core `lint` with no Error.
  - `shared_var_default_never_set_for_secret`
  - `corrupt_state_file_falls_back`
  - `no_secret_reaches_backend`: the mock records every argument. Running share and sync with a collection whose Keychain holds `sk-test-value` leaves no recorded string containing it.
- [ ] **Step 2–4:** FAIL → implement → PASS. Also run clippy on the portable model.
- [ ] **Step 5:** Commit: `feat(studio): backend seam and in-process mock`.

### Task 2: Sign in and server choice (board: SignIn) + Accounts & sync made real

**Files:**
- Create: `ui/sign_in.rs`
- Modify:
  - `ui/settings_view.rs` (Accounts & sync: Server, Account and Sync cards are real; drop the `Later build` badges);
  - `ui/welcome.rs` (cloud chip `Connected to curl-runner Cloud · Change`);
  - `ui/app.rs` (construct the `MockBackend`; dev menu);
  - `ui/workspace.rs` (route).

**Board details:**
- Two choice cards: `curl-runner Cloud · Managed` and `Self-hosted · Open source`, with the Setup guide link.
- A Server URL field and `Test connection`, which shows `Connected · server 2.1.1 · sign-in via SSO · 42 ms` or the error.
- `Continue with SSO` / GitHub / Email.
- `Use offline · Continue offline`.
- The status bar shows `Not signed in`.
- Shown on first launch when there is no stored choice, before Welcome. "Continue offline" stores `Offline` and never asks again; Settings › Accounts & sync can change it.

**Tests:**
- `first_launch_shows_sign_in_once`
- `continue_offline_skips_next_launch`
- `self_hosted_test_connection_shows_info`
- `sign_in_failure_toggle_shows_error`
- `settings_sign_out_returns_to_signed_out_cards`
- `ui_never_imports_mock_backend`, which greps `src/ui`.

Screenshot each state. Commit: `feat(studio): sign in, server choice and accounts`.

### Task 3: Sync status, offline and the sidebar footer (board: Main + SyncConflict banner)

**Files:**
- Create: `ui/sync.rs`, which holds the workspace sync state, a timer and sync-on-save.
- Modify:
  - `ui/sidebar.rs` (footer: `● Synced 2 min ago` + `Share`; offline: `Offline · 3 changes waiting`);
  - `ui/title_bar.rs` (`Shared · N` pill with the people count, opening Share);
  - `ui/status_bar.rs`.

**Behavior:**
- When signed in and the collection is synced, save marks the file pending. A sync runs about 2 s after the last save, and `Sync now` runs one immediately.
- `SyncFile` is built from tracked YAML files only. Never `.env*`, never anything under `.git` or hidden paths.
- While offline, pending changes are counted, and the next sync after going online pushes them.
- Signed out, or with Offline chosen, the footer shows `Local only` and the sync UI is hidden.

**Tests:**
- `save_marks_pending_then_syncs`
- `offline_counts_changes_and_flushes_when_back`
- `env_files_never_in_sync_payload`
- `signed_out_hides_sync`

Commit: `feat(studio): sync status and offline mode`.

### Task 4: Share dialog and the Sharing settings page (boards: Share, Environments "Shared by" rows)

**Files:**
- Create: `ui/share_dialog.rs`
- Modify: `ui/collection_settings.rs` (Sharing page made real, without its `Soon` chip; `Shared by {org}` rows and their lock legend on the Environments page for received collections only).

**Board details:**
- `Share {collection}` with the subtitle copy.
- The share link with `Copy link`.
- An access segmented control: `Anyone with the link · Can view and run` or `Invited people only`, with `Expires in 30 days`.
- Invite by email.
- The people list with role and remove.
- The variables section: names, `secret`/`required` chips, descriptions and an optional `Share default` per non-secret, with the board's note that values are never shared.
- The link is generated by the mock. Copy writes it to the clipboard.

**Tests:**
- `share_creates_link_and_copy_puts_it_on_clipboard`
- `invite_and_remove_person`
- `secret_vars_never_offer_share_default`
- `share_payload_has_no_values_unless_share_default`, which asserts on the mock's recorded arguments.
- `offline_share_shows_inline_error`

Commit: `feat(studio): share dialog and sharing settings`.

### Task 5: Open a shared link (board: OpenShared)

**Files:**
- Create: `ui/open_shared.rs`
- Modify:
  - `ui/welcome.rs` (`Open a shared link ⌘L`, made real);
  - `ui/palette.rs`;
  - `ui/app.rs` (`cx.on_open_urls` for `curl-runner://`);
  - `scripts/studio-bundle.sh`: builds a local `.app` whose `Info.plist` registers `CFBundleURLTypes` `curl-runner`. Documented in the crate README. Nothing is installed globally; the script writes into `target/`.

**Flow:**
1. Paste or receive a link, then `resolve_link`.
2. The header reads `Opened from a link · {url}`, with the owner avatar initials, `{Org} shared {Collection} with you` and the subtitle.
3. A two-step stepper: `Review` and `Your credentials`.
4. A summary card: `12 requests · rates, labels, tracking · updated 3 days ago · Read only`.
5. A variable form, required first:
   - secrets go to the Keychain for the new collection's service;
   - non-secrets go to the collection's local `.env`;
   - the default `from {Org}` is prefilled.
6. The footer note copy, then `Back · Cancel · Add to my studio`.
7. Add materializes the snapshot into `{base}/shared/{org}-{collection}/` (real files), records a `ReceivedShare`, marks the collection read-only in CollectionMeta (`read_only_from: Some(org)`), and opens it.

**Errors:**
- Expired: `This link has expired. Ask {org} for a new one.`
- Unknown: `This link doesn't work.`
- Offline: `You're offline. Connect to open shared links.`
- In every error case, no folder is created.

**Tests:**
- `paste_link_resolves_and_adds_collection`
- `credentials_go_to_keychain_and_env`
- `expired_link_creates_nothing`
- `garbage_link_shows_error`
- `deep_link_event_opens_flow`, which calls the `on_open_urls` handler directly.

Commit: `feat(studio): open shared links`.

### Task 6: Read-only customer view (board: ReadOnly)

**Files:**
- Modify:
  - `ui/sidebar.rs` ("Shared with me" section listing received collections, with a `Read only` pill; "My collections" section);
  - `ui/editor_pane.rs` (locked editor: no edits, a breadcrumb badge `Read only · shared by {org} · updated 3 days ago` and `Check for updates`; banner `Duplicate to my workspace to edit`);
  - `ui/workspace.rs` (guards on save, rename, delete, quick fix, new request and import for read-only collections);
  - `ui/inspector.rs` (summary line `ran with your API_TOKEN`).
- `Check for updates` calls `fetch_shared` and replaces the files after a diff count confirm. Run uses only the customer's own credentials.
- `Duplicate to my workspace`: a native folder picker, then copy the files. The `.env` is never copied, and the Keychain is re-keyed by copying secret NAMES only; the user re-enters values. The copy opens as a normal editable collection.

**Tests:**
- `read_only_blocks_every_write_path`: typing, ⌘S, rename, delete, quick fix, palette New request.
- `run_works_in_read_only`
- `check_for_updates_replaces_files`
- `duplicate_copies_yaml_not_env_and_is_editable`

Commit: `feat(studio): read-only shared collections`.

### Task 7: Sync conflict (board: SyncConflict)

**Files:**
- Create: `ui/conflict_view.rs`

**Behavior:**
- A conflict from a sync opens a full-width view over the editor:
  - title: `{file} changed on both sides`;
  - subtitle: `You edited it offline, and {author} pushed a change {ago}. Pick which version to keep.`;
  - `Conflict N of M`;
  - two columns, `Your version · edited offline · 40 min ago` and `{author}'s version · pushed {ago}`, with a line diff (`−`/`+` tokens from the design system);
  - buttons `Keep mine`, `Take theirs` and `Keep both`.
- `Keep both` writes the server copy as `{stem}.server.yaml`.
- Resolving the last conflict syncs again. The mock produces the server copy as specified in Task 1.

**Tests:**
- `keep_mine_pushes_local`
- `take_theirs_writes_server_text_exactly`
- `keep_both_writes_server_copy_next_to_local`
- `multiple_conflicts_step_through`
- `conflict_never_includes_env_files`

Commit: `feat(studio): sync conflicts`.

### Task 8: Deferred cleanup from plans 1–2

Take the parked items in `.superpowers/sdd/2026-10-30-studio-2-runs/progress.md` and the plan 1 ledger that are still live and cheap:
- split `ui/collection_settings.rs` (drawer and keychain moves) and `ui/workspace.rs` (routing) into focused files, as a pure move;
- drop the import `scrub_text` backstop's word mangling;
- the import-time referer and proxy-query scrub;
- prune unparseable history files by mtime;
- `CURL_RUNNER_STUDIO_OPEN` path validation;
- the breadcrumb `·` separator;
- the method badge column for DELETE/OPTIONS in the tree, if still clipped.

Each item gets a test where it changes behavior. Commit per item.

---

After plan 3, every board on the canvas is real or mocked as the spec's table says.
