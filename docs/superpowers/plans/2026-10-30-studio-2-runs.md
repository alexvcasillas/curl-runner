# curl-runner studio, plan 2: runs, results, environments, history

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every local (non-server) board on the canvas real.
- Run a request or a collection through `curl-runner-core`.
- Show the inspector, the failed-request view, the collection run, history, environments with Keychain secrets, settings, import and the first-launch screen.

**Architecture:**
- **Portable model.** Run orchestration, environment resolution, check breakdowns, history and export live in `crates/curl-runner-studio/src/model/` with no GPUI, so they unit-test on every OS.
- **Run thread.** A run executes on a background thread. It streams `RunEvent`s over a channel that the workspace drains on the GPUI executor.
- **Secrets.** Secrets sit behind a `SecretStore` trait. The macOS Keychain is one implementation and an in-memory store is the other, for tests.
- **Core.** Core gets two additive hooks only. The CLI's output does not change.

**Tech Stack:**
- Rust 2024, toolchain per `rust-toolchain.toml`.
- gpui-kit `=0.8.1` and gpui-pre `=0.3.10`.
- Already in `Cargo.lock`: `security-framework` 3.7.0, `similar` 2.7.0, `chrono` 0.4.45, `quick-xml` 0.41.0. Use these versions; add no new crates.
- `testkit` (path dev-dependency) for HTTP test servers.

**Spec:** `docs/superpowers/specs/2026-10-29-curl-runner-studio-design.md`.

**Boards:** each UI task names its board in `docs/superpowers/specs/studio-mockups/<Board>.dc.html`.
- The board is the visual source of truth: sizes, copy, colors and states.
- Open it and match it; any token comes from `studio-design-system/tokens.json`.
- Data on the boards is fake. Render the real data in the same layout.

## Global Constraints

- **Everything from plan 1's Global Constraints still applies.**
  - macOS app, with a stub on other OSes.
  - CI stays green on ubuntu, macos and windows.
  - `publish = false`.
  - Tokens only, Geist / Geist Mono.
  - Window chrome 48 / 264 / 400 / 28.
  - Public repo placeholders: Acme, Payments API, api.example.com.
  - Every change ships with tests.
- **CLI output stays the same.** `curl-runner` output, exit codes and the `parity/` suite must not change. Core changes are additive:
  - a new trait method with a default no-op;
  - a new builder method.
- **Collection runs follow the CLI's semantics.**
  - Each file is its own group, mirroring `RunCtx::execute` in `crates/curl-runner/src/run.rs`: its own `Executor`, its own captured-value store, and the same env lookup.
  - A failure stops only its own group unless `continueOnError` is set; the next group still runs.
  - Ruling: the spec says "one captured-value store shared across the run", but the spec's first anti-goal ("studio and CLI disagree") wins, so the CLI's per-file stores stand.
- **Value precedence at run time, highest first:** captured (the store) → Keychain secret → the environment's `.env` files → the process env.
  - Pass the merged lookup to `Executor::with_env`.
  - Also pass it to `load_run` as `env`, so `${VAR}` resolves the same way.
- **Environments.**
  - The list is `Default` plus one entry per `.env.{name}` file in the collection root, ignoring `*.local`, sorted.
  - The picked environment is passed as `CliOptions { env: Some(name) … }` (the CLI flag), so it wins over YAML `environment`.
  - `Default` passes none.
- **Secrets.**
  - A name starting with `SECRET_`, or listed in the collection's `secret_names`, is a secret.
  - Secret values come only from the Keychain. The studio never writes them to disk, history, exports or logs.
  - Before every run, every secret value goes to the run's `Redactor` through `add_secrets`.
  - Keychain item: service `curl-runner studio · {collection name} · {environment}`, account = the variable name. This is the copy shown on the Environments board.
- **Run saves first.** Run saves the active file before running (the CLI reads disk).
- **Application Support.** Studio data lives under `~/Library/Application Support/curl-runner studio/`.
  - Use `StateStore`'s base dir.
  - Tests use a temp dir; never the real one.
- **Machine safety for builders.**
  - Never send synthetic keyboard or mouse input to the desktop.
  - Never install toolchains, targets or global tools.
  - Never touch the user's real Keychain in tests: use `MemorySecrets`. One `#[ignore]`d test may exercise the real Keychain with a throwaway service name and delete it afterwards.

## Review Focus

1. **A secret leaking.** A Keychain value must never appear in an exported report, a history file or the inspector's Request tab. Pinned in Task 4 (history), Task 6 (export) and Task 2 (redactor gets the secrets).
2. **Stop mid-run.** Pressing Stop during a retry wait or an in-flight request ends the run within about 100 ms. The run is marked Stopped, not Failed, and the app keeps working. Pinned in Task 2.
3. **Running a file that doesn't parse at run time** (a `LoadError`, or a request that fails to type) shows one failed row with the parser's message. No panic, no hang. Pinned in Task 2.
4. **Huge or binary response bodies.** A 5 MB body or a non-UTF-8 body renders truncated, with a "Showing first 256 KB" note. History stores at most 64 KB per body. Pinned in Task 3 (model) and Task 4 (history).
5. **A missing variable.** A `${API_TOKEN}` with no value anywhere shows "1 missing" in the status bar and a warning in the inspector before the run. The run still goes, as the CLI would send the literal. Pinned in Task 2 (`missing_vars`) and Task 7.

---

### Task 1: Core hooks: per-attempt events and a seeded store

**Files:**
- Modify: `crates/curl-runner-core/src/executor/events.rs`
- Modify: `crates/curl-runner-core/src/executor/mod.rs`
- Modify: `crates/curl-runner-core/src/executor/sequential.rs`
- Modify: `crates/curl-runner-core/src/executor/parallel.rs`
- Test: `crates/curl-runner-core/src/executor/tests.rs`

**Interfaces:**
- Produces:
  - `EventSink::on_attempt(&mut self, index: usize, req: &RequestConfig, attempt: u32, outcome: &AttemptOutcome)`, with a default no-op.
  - `pub struct AttemptOutcome { pub status: Option<u16>, pub duration_ms: f64, pub error: Option<String> }`, exported from `executor`.
  - It fires after every transfer, including the last one, before `on_retry` or `on_result`.
  - `Executor::with_store(self, store: interpolate::Store) -> Self`: the sequential runner starts from this store instead of an empty one. The parallel runner ignores it, since it has no store.

- [ ] **Step 1: Write failing tests in `executor/tests.rs`** (reuse the file's recording sink and `testkit::TestServer`):
  - `attempts_reported_for_each_retry`: the server answers 503, 503, then 200. The request has `retry: {count: 2, delay: 1}`. Assert three `on_attempt` calls with statuses `[Some(503), Some(503), Some(200)]` and attempts `[1, 2, 3]`.
  - `attempt_reports_transport_error`: use `closed_port_url()`. Assert one attempt with `status: None` and `error: Some(_)`.
  - `seeded_store_interpolates_first_request`: `with_store({"CHARGE_ID": "ch_1"})`. The request URL `…/${store.CHARGE_ID}` hits the path `/ch_1`. Check the existing store syntax in `interpolate.rs` and use it.
- [ ] **Step 2:** Run `cargo test -p curl-runner-core executor::tests::attempt`. Expected: FAIL (no method).
- [ ] **Step 3: Implement.**
  - Call `on_attempt` in both runners, right after each transfer completes.
  - `with_store` stores an `Option<Store>`; `sequential::run` uses `ex.initial_store.clone().unwrap_or_default()`.
- [ ] **Step 4:** Run `cargo test -p curl-runner-core` and `cargo test -p curl-runner`, plus the parity suite as the repo runs it (see `AGENTS.md`). Expected: all PASS, with no CLI test changed.
- [ ] **Step 5:** Commit: `feat(core): per-attempt executor events and a seeded response store`.

### Task 2: Run engine model: environments, variable resolution, runs on a thread

**Files:**
- Create: `crates/curl-runner-studio/src/model/environments.rs`
- Create: `crates/curl-runner-studio/src/model/secrets.rs`
- Create: `crates/curl-runner-studio/src/model/run.rs`
- Modify: `crates/curl-runner-studio/src/model/mod.rs`
- Modify: `crates/curl-runner-studio/Cargo.toml`:
  - `security-framework = "3.7.0"` under the macOS deps;
  - `testkit = { path = "../testkit" }` and `chrono = "0.4.45"` as deps / dev-deps as needed.

**Interfaces:**
- Consumes: Task 1's `on_attempt`, `AttemptOutcome` and `with_store`; core's `load_run`, `Executor`, `Redactor`, `env::load_env_files` and `discover`.
- Produces:
  - `environments.rs`:
    - `pub fn list_environments(root: &Path) -> Vec<String>`: `"Default"` first, then the names from `.env.{name}`, sorted, skipping `.local`.
    - `pub fn is_secret(name: &str, marked: &[String]) -> bool`.
    - `pub struct VarSource { Captured, Keychain, EnvFile(PathBuf), Process }` (an enum).
    - `pub struct ResolvedVar { pub name: String, pub value: Option<String>, pub source: Option<VarSource>, pub secret: bool }`.
    - `pub fn resolve_vars(names: &[String], captured: &Store, secrets: &dyn SecretStore, ctx: &EnvCtx) -> Vec<ResolvedVar>`.
    - `pub struct EnvCtx { pub root: PathBuf, pub collection: String, pub environment: String, pub marked_secret: Vec<String> }`.
    - `pub fn missing_vars(yaml: &str, resolved: &[ResolvedVar]) -> Vec<String>`: `${NAME}` references with no value, ignoring `store.*` and dynamic variables. Use `interpolate::extract_variables`.
  - `secrets.rs`:
    - `pub trait SecretStore: Send + Sync { fn get(&self, service: &str, name: &str) -> Option<String>; fn set(&self, service: &str, name: &str, value: &str) -> Result<(), String>; fn delete(&self, service: &str, name: &str) -> Result<(), String>; fn names(&self, service: &str) -> Vec<String>; }`
    - `pub fn service_name(collection: &str, environment: &str) -> String`, which gives `curl-runner studio · {collection} · {environment}`.
    - `pub struct MemorySecrets`, for every OS.
    - `#[cfg(target_os = "macos")] pub struct KeychainSecrets`, built on `security_framework::passwords` plus an item search for `names`.
  - `run.rs`:
    - `pub enum RunTarget { File(PathBuf), Request { file: PathBuf, index: usize }, Collection, Files(Vec<PathBuf>) }`. `Files` is used by Re-run failed. Paths are relative to the root.
    - `pub struct RunRequest { pub target: RunTarget, pub ctx: EnvCtx, pub seed: Option<Store> }`.
    - `pub enum RunEvent { Started { total: usize, groups: Vec<(PathBuf, usize)> }, RequestStarted { group: usize, index: usize, name: String }, Attempt { group: usize, index: usize, attempt: u32, outcome: AttemptOutcome }, RetryWait { group: usize, index: usize, delay_ms: u64 }, Result { group: usize, index: usize, result: Box<ExecutionResult> }, Stored { group: usize, values: Store }, LoadFailed { file: PathBuf, message: String }, Finished(RunOutcome) }`.
    - `pub struct RunOutcome { pub summary: ExecutionSummary, pub stopped: bool, pub stores: Vec<(PathBuf, Store)>, pub started_at: chrono::DateTime<chrono::Local> }`.
    - `pub struct RunHandle { pub events: std::sync::mpsc::Receiver<RunEvent>, cancel: … }` with `pub fn stop(&self)`.
    - `pub fn start_run(req: RunRequest, secrets: Arc<dyn SecretStore>) -> RunHandle`.
    - `Request { index }` runs that request alone. `seed` comes from the file's last run's store.
  - Every `ExecutionResult` sent in an event has already been redacted for display: body text, headers and the URL all go through `Redactor::redact`.

- [ ] **Step 1: Write failing tests** (each in its own module's `#[cfg(test)]`, with tempdirs and `testkit::TestServer`):
  - `environments_are_default_plus_env_files`: files `.env`, `.env.staging`, `.env.production`, `.env.staging.local` → `["Default", "production", "staging"]`.
  - `secret_by_prefix_or_mark`: `is_secret("SECRET_KEY", &[])`; `is_secret("API_TOKEN", &["API_TOKEN".into()])`; `!is_secret("BASE_URL", &[])`.
  - `precedence_captured_over_keychain_over_env_file`: one name set in all three. The value comes from the store; remove it and it comes from the Keychain; remove that and it comes from `.env.staging`.
  - `missing_vars_lists_unset_names_only`: `"${BASE_URL}/x ${API_TOKEN} ${store.ID} ${DATE:YYYY}"` with only `BASE_URL` set → `["API_TOKEN"]`.
  - `run_file_streams_events_and_finishes`: a 2-request file against a TestServer. Events in order: Started (total 2), then RequestStarted, Attempt and Result twice, then Finished with `successful == 2`.
  - `collection_groups_have_own_stores`: `a.yaml` stores `ID=abc` from the response, and `b.yaml` requests `/${store.ID}`. The server never sees the path `/abc`. Pin the exact path `b` sends by running the same two files through `curl-runner` (the CLI binary, built by the test via `env!("CARGO_BIN_EXE_…")` is not available across crates, so drive `load_run` plus one `Executor` per file the way `run.rs` `execute` does), and assert the studio sends the same path.
  - `request_target_runs_one_with_seed`: a file of 3 requests; target index 2 with seed `{ID: "x"}`. Exactly one Result, and its URL contains `x`.
  - `stop_ends_run_quickly`: the server sleeps 5 s. Call `stop()` after the first `RequestStarted`. `Finished { stopped: true }` arrives in under 1 s.
  - `load_failure_is_an_event_not_a_panic`: a file with `request: [` → `LoadFailed`, then `Finished`.
  - `secret_never_in_events`: Keychain (MemorySecrets) `API_TOKEN=sk_live_abc`, and the server echoes the Authorization header into the body. No event's `Debug` string contains `sk_live_abc`.
- [ ] **Step 2:** Run `cargo test -p curl-runner-studio model::`. Expected: FAIL.
- [ ] **Step 3: Implement.**
  - Group files exactly as `group_by_file` in the CLI's `run.rs` does. Copy its semantics; it is CLI-private.
  - The event sink sends on the channel. Map `on_retry` to `RetryWait`.
  - `KeychainSecrets.names` uses `security_framework::item::ItemSearchOptions` filtered by service.
- [ ] **Step 4:** Run the tests. Expected: PASS. Also run `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] **Step 5:** Commit: `feat(studio): run engine, environments and secret store`.

### Task 3: Result model: checks, body diff, attempts, display limits

**Files:**
- Create: `crates/curl-runner-studio/src/model/result_view.rs`

**Interfaces:**
- Consumes: `ExecutionResult`; core `validate::{validate_status, validate_headers, validate_value, is_regex_pattern}`; `similar`.
- Produces:
  - `pub struct Check { pub label: String, pub expected: String, pub actual: Option<String>, pub passed: bool }`.
    - Labels are `status`, `header.{name}`, `body.{path}` and `responseTime`.
    - Expected text is `matches ^ch_` for a regex, else the JSON text.
  - `pub fn checks(result: &ExecutionResult) -> Vec<Check>`.
    - Walk `result.request.expect`.
    - Each leaf of `expect.body` is one check at its dotted path, the same paths `validate_body_properties` uses.
    - Compute each check by calling the core validators on that one leaf, so verdicts match the CLI.
  - `pub fn body_diff(expected: &Json, actual: &Json) -> Vec<DiffLine>`.
    - `pub struct DiffLine { pub kind: DiffKind, pub text: String }`, with `DiffKind = Same | Added | Removed`.
    - Pretty-print the actual body.
    - For every failed body check, show the expected line as `Added` and the actual line as `Removed`, at the actual line's position. This is how the board shows `+ "amount": 2500,` over `- "amount": 2000,`.
  - `pub struct AttemptRow { pub attempt: u32, pub status: Option<u16>, pub duration_ms: f64, pub note: String, pub wait_ms: Option<u64> }`.
  - `pub fn attempt_rows(attempts: &[(u32, AttemptOutcome)], waits: &[u64], final_checks_failed: usize, retries_left: bool) -> Vec<AttemptRow>`.
    - Notes copy the board: `Service unavailable — retrying`, `Response received · 2 checks failed — no retries left`, `Response received`.
    - Status text comes from the reason phrase.
  - `pub fn display_body(body: &Json) -> DisplayBody { text: String, truncated: bool }`: at most 256 KB, with JSON pretty-printed.
  - `pub fn timing_segments(m: &Metrics) -> Vec<(&'static str, f64)>`: `DNS`, `TLS`, `Wait` and `Body`, as on the Main board. Missing fields are omitted.
  - `pub fn status_line(result: &ExecutionResult) -> String`, giving `201 Created`.

- [ ] **Step 1: Write failing tests:**
  - `checks_split_body_leaves`: expect `{status: 201, body: {id: "^ch_", amount: 2500}}` and actual `{id: "ch_1", amount: 2000}` → three checks. `body.amount` fails with expected `2500` and actual `2000`; `body.id` passes with expected `matches ^ch_`.
  - `check_verdicts_agree_with_cli`: for 6 fixtures, `checks().all(passed) == result.success` whenever there is no transport error.
  - `diff_marks_mismatched_lines`: for the case above, the output contains `Added "amount": 2500,` immediately before `Removed "amount": 2000,`.
  - `attempt_notes_match_board`: 503, 503, 200 with 2 failed checks → the three notes from the board, verbatim.
  - `large_body_truncates`: a 5 MB string body gives `truncated == true` and text length at most 256 KB.
  - `non_utf8_body_does_not_panic`: a body of `Json::String` holding replacement chars renders.
- [ ] **Step 2:** Run. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run. Expected: PASS.
- [ ] **Step 5:** Commit: `feat(studio): check breakdown, body diff and attempt timeline model`.

### Task 4: History store and run comparison

**Files:**
- Create: `crates/curl-runner-studio/src/model/history.rs`

**Interfaces:**
- Consumes: `RunOutcome` (Task 2) and `Redactor`.
- Produces:
  - `pub struct HistoryEntry { pub id: String, pub at: DateTime<Local>, pub collection: String, pub target: String, pub kind: EntryKind /* CollectionRun | Request */, pub environment: String, pub passed: u64, pub total: u64, pub duration_ms: f64, pub retries: u32, pub results: Vec<StoredResult> }`.
  - `pub struct StoredResult { pub file: PathBuf, pub name: String, pub method: String, pub status: Option<u16>, pub success: bool, pub duration_ms: f64, pub attempts: u32, pub checks_passed: usize, pub checks_total: usize, pub failed_checks: Vec<String>, pub body: Option<String> }`.
    - `body` is redacted and capped at 64 KB.
    - All types are serde.
  - `pub struct History { dir: PathBuf }` with:
    - `open(base: &Path, collection_root: &Path) -> History`. The dir is `history/{hex of a stable hash of the root path}/`.
    - `record(&self, e: &HistoryEntry) -> io::Result<()>`, one JSON file per entry: `{at:%Y%m%dT%H%M%S}-{id}.json`.
    - `list(&self) -> Vec<HistoryEntry>`, newest first, skipping unreadable files.
    - `prune(&self, now: DateTime<Local>)`: delete anything older than 30 days.
    - `usage(&self) -> (usize, u64)`: the count and bytes, shown as `142 runs stored · 3.1 MB`.
  - `pub fn entry_from_outcome(o: &RunOutcome, target: &str, kind: EntryKind, env: &str, redactor: &Redactor) -> HistoryEntry`.
  - `pub struct Comparison { pub newly_failing: Vec<StoredResult>, pub fixed: Vec<StoredResult>, pub slower: Vec<(StoredResult, f64 /*prev ms*/)>, pub unchanged: usize }`.
  - `pub fn compare(current: &HistoryEntry, previous: &HistoryEntry) -> Comparison`.
    - Results match by `(file, name)`.
    - "Slower" means `cur >= prev * 1.25 && cur - prev > 50.0`, per the board's footnote.
  - `pub fn previous_for(entries: &[HistoryEntry], e: &HistoryEntry) -> Option<&HistoryEntry>`: the latest earlier entry with the same target and environment.
  - `pub fn duration_series(entries: &[HistoryEntry], target: &str, env: &str, n: usize) -> Vec<f64>`, for the last 12 runs.

- [ ] **Step 1: Write failing tests:**
  - `record_list_roundtrip_newest_first`.
  - `prune_drops_entries_older_than_30_days`.
  - `body_capped_and_redacted`: a 100 KB body containing secret `sk_live_abc` is stored as at most 64 KB and without the secret.
  - `compare_classifies`: one result newly failing, one fixed, one at 109 → 212 ms (slower), one at 100 → 120 ms (not slower: +20%), one at 20 → 40 ms (not slower: +100% but only 20 ms) → counts 1/1/1, `unchanged == 2`.
  - `corrupt_file_skipped`: a garbage `.json` in the dir; `list` still returns the others.
- [ ] **Step 2–4:** Run (FAIL), implement, run (PASS).
- [ ] **Step 5:** Commit: `feat(studio): local run history with comparison`.

### Task 5: Inspector and single-request runs (board: Main)

**Files:**
- Create: `crates/curl-runner-studio/src/ui/inspector.rs`
- Create: `crates/curl-runner-studio/src/ui/runs.rs`: the workspace's run state, which drains `RunHandle` events on `cx.spawn`.
- Modify:
  - `ui/workspace.rs`: a third column, the inspector at 400 px;
  - `ui/run_button.rs`: run instead of log; `⌘↵`;
  - `ui/title_bar.rs`: the environment picker and Run collection, per the board;
  - `ui/status_bar.rs`: the environment and variable sources text;
  - `ui/app.rs`: keymap;
  - `ui/mod.rs`.
- Fold-in from the plan 1 ledger: gate the `CURL_RUNNER_STUDIO_*` dev env hooks behind `cfg(debug_assertions)`.

**Interfaces:**
- Consumes: Tasks 2 and 3. The `AppState` active document.
- Produces:
  - `Workspace::run(&mut self, target: RunTarget, window, cx)`.
  - `Workspace.runs: Runs`, which holds:
    - `last: HashMap<PathBuf, FileRun>`, where `FileRun { results: Vec<ExecutionResult>, attempts: HashMap<usize, Vec<(u32, AttemptOutcome)>>, waits: HashMap<usize, Vec<u64>>, store: Store, env: String, at: DateTime<Local> }`;
    - `active: Option<RunHandle>`.
  - `Workspace.environment: String`, persisted per collection in `StateStore`.
  - `InspectorTab = Response | Checks | Attempts | Headers | Timing | Request`.
    - Show Attempts only when there was more than one attempt.
    - The Request tab shows the method, the URL and the headers as sent, redacted.

**Board details to match (Main):**
- The status line: `201 Created`, a green status dot, `184 ms · 412 B`, `Staging · 2 min ago`.
- The summary line: `4 of 4 checks passed · 1 attempt`.
- The tabs, with counts.
- The JSON body highlighted with the YAML theme's token colors.
- The Checks list with a check icon per row.
- The Timing stacked bar with the legend `DNS 8 ms  TLS 22 ms  Wait 120 ms  Body 34 ms`.
- Empty state before any run: `Run this request to see the response` with the `⌘↵` hint.
- While running: a spinner and `Running… attempt 2 of 3`.
- In a multi-request file, the inspector follows the request under the cursor. Run runs the file; `⌥⌘↵` runs only the request under the cursor (`RunTarget::Request`).
- The status bar shows `Staging`, then `BASE_URL · API_TOKEN from local keychain`, built from `resolve_vars` over the file's referenced names, grouped by source. Missing names show `1 missing` in amber (Review Focus 5).

- [ ] **Step 1: Write failing GPUI smoke tests** (`#[gpui_kit::test]`, TestServer, temp collection, `MemorySecrets` injected through the `Workspace::new` parameters):
  - `run_shows_response_in_inspector`: open `ok.yaml` and dispatch `RunActive`. After events drain, the inspector text contains `200 OK` and `1 of 1 checks passed`.
  - `run_saves_dirty_buffer_first`: edit the URL path in the buffer, run, and assert the server saw the edited path.
  - `blocked_file_does_not_run`: a file with an Error diagnostic; Run does nothing and the server sees 0 requests.
  - `environment_picker_switches_values`: `.env.staging` has `BASE_URL=<server>/s` and `.env.production` has `BASE_URL=<server>/p`. Pick production, run, and the server path starts with `/p`.
  - `dev_hooks_ignored_in_release`: a unit test on the gating helper. It compiles under both cfgs; assert behavior for `cfg!(debug_assertions)`.
- [ ] **Step 2:** Run `cargo test -p curl-runner-studio ui::`. Expected: FAIL.
- [ ] **Step 3:** Implement, matching the board.
- [ ] **Step 4:** Run the tests and clippy. Expected: PASS. Then launch on `~/Desktop/curl-runner-studio-examples`, run `users/user-flow.yaml`, screenshot the window (`screencapture -l` on the window id, no input events) to `fidelity/out/P2-T5-inspector.png`, and compare with `Main.dc.html`.
- [ ] **Step 5:** Commit: `feat(studio): inspector and request runs`.

### Task 6: Report export (JSON, JUnit, Markdown, CLI command)

**Files:**
- Create: `crates/curl-runner-studio/src/model/export.rs`

**Interfaces:**
- Consumes: `ExecutionSummary`, `Redactor`, `RunTarget` and `EnvCtx`.
- Produces:
  - `pub fn to_json(s: &ExecutionSummary, r: &Redactor) -> String`.
    - It is exactly core's `save_summary` content: `serde_json::to_value` → `snapshot::json::js_stringify_pretty(_, 2)` → `r.redact`.
    - Make `js_stringify_pretty` reachable if it isn't. That is an additive `pub use`.
  - `pub fn to_junit(s: &ExecutionSummary, groups: &[(PathBuf, usize)], r: &Redactor) -> String`.
    - One `<testsuite>` per file and one `<testcase name=… classname=file time=seconds>` per request.
    - A failure becomes `<failure message=…>` holding the error. A skipped request becomes `<skipped/>`.
    - Write it with `quick-xml`.
  - `pub fn to_markdown(s: &ExecutionSummary, title: &str, env: &str, r: &Redactor) -> String`.
    - A heading, a `Passed N · Failed N · Duration` line, a table (`Method | Request | Status | Duration | Result`) and a `### Failures` list.
  - `pub fn cli_command(target: &RunTarget, ctx: &EnvCtx, needed_secrets: &[String]) -> String`.
    - Gives `curl-runner <paths> --env staging`, preceded by a comment line naming the variables to set: `# set API_TOKEN (keychain) before running`.

- [ ] **Step 1: Write failing tests:**
  - `json_matches_core_save_summary`: write with core `save_summary` to a temp file and compare it with `to_json` byte for byte.
  - `junit_is_well_formed_and_counts`: parse the output back with quick-xml. The suite holds `tests="3" failures="1" skipped="1"`.
  - `exports_redact_secrets`: a secret inside an error message and a body is absent from all three formats.
  - `cli_command_lists_env_and_secrets`.
- [ ] **Step 2–4:** FAIL → implement → PASS.
- [ ] **Step 5:** Commit: `feat(studio): JSON, JUnit and Markdown run reports`.

### Task 7: Failed request view (board: FailedRequest)

**Files:**
- Modify: `ui/inspector.rs` and `ui/editor_pane.rs`
- Create: `ui/result_marks.rs`: inline `got 2000` hints after expect lines, using `lint::spans` to locate `request.expect.body.amount`.

**Interfaces:**
- Consumes: Task 3 (`checks`, `body_diff`, `attempt_rows`) and `curl_runner_core::lint::spans`.
- Produces:
  - `pub fn expect_hints(text: &str, request_index: Option<usize>, checks: &[Check]) -> Vec<(Range<usize>, String)>`. It is pure and lives in `model/result_view.rs`.
  - Inspector actions: `CopyAsCurl` (the `on_command` text, redacted) and `Rerun`.

**Board details:**
- The breadcrumb badge `2 checks failed on last run`.
- The status line in red when checks failed: `200 OK · 846 ms total · 388 B`.
- The summary line `2 of 4 checks failed · 3 attempts`.
- The buttons `Copy as curl` and `Re-run`.
- Tabs: `Checks 2` (the failed count) and `Attempts 3`.
- The Checks list: passed rows compact; failed rows expanded with `mismatch` and `expected` / `actual` rows in mono.
- The panel `Response body · diff against expected`, with a `Diff | Raw` segmented control. `+` lines use the success tokens and `-` lines the error tokens, per the board.
- The `Retry timeline` with the subtitle `count 3 · delay 1000 · backoff ×2` from the request's retry config, and one row per `AttemptRow` with its wait chip.
- In the editor: amber `got 2000` ghost text at the end of the failing expect line. It is cleared on the next edit of that line.
- The status bar adds `CHARGE_ID captured from create-charge` when a referenced value came from the store.

- [ ] **Step 1: Write failing tests:**
  - `expect_hints_point_at_failed_leaves` (pure): the board's YAML with a failure on `amount` gives one hint, on line 16, with text `got 2000`.
  - A smoke test, `failed_run_shows_checks_and_timeline`: the server returns 503, 503, then 200 with a mismatching body. The inspector text contains `2 of 4 checks failed`, `Attempt 3` and `no retries left`.
- [ ] **Step 2–4:** FAIL → implement → PASS. Screenshot `P2-T7-failed.png` and compare with the board.
- [ ] **Step 5:** Commit: `feat(studio): failed request view with diff and retry timeline`.

### Task 8: Collection run screen (board: RunCollection)

**Files:**
- Create: `ui/run_view.rs`
- Modify: `ui/workspace.rs`, so the main area shows the run view while a collection run is open; it closes back to the editor.
- Modify: `ui/title_bar.rs`, so Run collection starts a run.

**Interfaces:**
- Consumes: Tasks 2, 4 and 6. `RunTarget::Collection` and `RunTarget::Files`.
- Produces: `RunView` (an entity) with a `rows: Vec<RowState>` grouped by top-level folder.
  - `RowState::{Queued, Running, Passed, Failed, Skipped}`.
  - Grouping by folder is pure: `pub fn group_rows(groups: &[(PathBuf, usize)], names: …) -> Vec<FolderGroup>` in `model/run.rs`, so it is testable.

**Board details:**
- The header: `Run · {collection}`, then `Started 09:41:07 · local · curl-runner {VERSION}`, the environment chip, and a status chip (`Running`, `Passed`, `Failed` or `Stopped`).
- The buttons:
  - `Stop ⌘.`
  - `Re-run failed`, enabled only after a finish with failures.
  - `Export report ▾`, a menu with `JUnit XML .xml`, `JSON .json`, `Markdown summary .md` and `Copy CLI command`, plus the note `Exports when the run finishes. Same format as curl-runner --output.`
  - Export saves through the native save panel.
- The progress bar: `14 of 22 requests · Running webhooks/send-test-event`, with an ETA of `~6 s left` = average duration × remaining.
- Four stat cards:
  - `Passed 12 of 14 finished`
  - `Failed 2` with the names
  - `Queued 8 · 1 running now`
  - `Retries 3 across 2 requests`
  - `Elapsed 4.8 s · avg 196 ms per request`
- The table: `Method | Request | Status | Duration | Checks | Attempts`, with collapsible folder headers (`charges 5 · 4 passed · 1 failed`).
  - Clicking a row opens that file, with the inspector on that result.
- The status bar: `Running 14/22`, then `Concurrency 1 · stop on failure off`, from the global config.
- Finishing records a history entry (Task 4) and prunes.

- [ ] **Step 1: Write failing tests:**
  - `group_rows_by_top_folder` (pure).
  - `eta_from_average` (pure): `pub fn eta_ms(done: &[f64], remaining: usize) -> Option<f64>`.
  - A smoke test, `collection_run_fills_table_and_history`: 3 files, one failing. After finish, the view text has `Passed 2` and `Failed 1`, `History::list()` has 1 entry, and Re-run failed starts a `Files([failing])` run.
  - A smoke test, `stop_marks_stopped`.
- [ ] **Step 2–4:** FAIL → implement → PASS. Screenshot `P2-T8-run.png`.
- [ ] **Step 5:** Commit: `feat(studio): collection run view with stop, re-run failed and export`.

### Task 9: History screen (board: History)

**Files:**
- Create: `ui/history_view.rs`
- Modify:
  - `ui/workspace.rs`: a route;
  - `ui/palette.rs`: an `Open history` action;
  - `ui/app.rs`: the `⌘Y` key binding.

**Interfaces:**
- Consumes: Task 4.
- Produces: `HistoryView`.
  - Filters: `All | Collection runs | Requests | Failed`, plus an environment dropdown.
  - `pub fn filter_entries(e: &[HistoryEntry], f: &Filter) -> Vec<usize>` and `pub fn day_bucket(at, now) -> String` (`Today`, `Yesterday`, `Oct 27`), both pure in `model/history.rs`.

**Board details:**
- The left list: `Time | Run | Environment | Where | Passed | Duration`, grouped by day.
  - `Where` is always `Local` in v1; CI rows are out of scope.
- The right detail:
  - the title and environment chip, with `Export` and `Re-run` buttons;
  - `Collection run · today 09:41 · local · curl-runner X`;
  - four stats;
  - a `Duration, last 12 runs on Staging` bar chart, with the slowest bar highlighted;
  - `Compared with previous run`, with the sections `Newly failing`, `Fixed` and `Slower`, and the footnote copy verbatim.
- The footer: `142 runs stored · 3.1 MB` and `History stays on this device unless you share a run`.

- [ ] **Step 1: Write failing tests:** `filter_failed_only`, `day_buckets` (with a fixed `now`), and a smoke test `history_shows_comparison`, which records two entries and finds `Newly failing 1` in the view.
- [ ] **Step 2–4:** FAIL → implement → PASS. Screenshot.
- [ ] **Step 5:** Commit: `feat(studio): run history screen`.

### Task 10: Environments screen (board: Environments)

**Files:**
- Create: `ui/collection_settings.rs`, a page with the board's left nav.
  - v1 implements `General` (name and path, read only) and `Environments`.
  - `Sharing`, `Workspace · Acme`, `Members`, `Sync & hosting` and `Billing` render disabled, with a `Soon` chip; plan 3 fills them.
  - The `App` items link to Settings (Task 11).
- Create: `model/collection_meta.rs`: per-collection metadata in Application Support, `meta/{hash}.json`.
  - `pub struct VarMeta { pub description: String, pub required_for_shared: bool }`.
  - `pub struct CollectionMeta { pub secret_names: Vec<String>, pub vars: BTreeMap<String, VarMeta>, pub environment: Option<String> }`.
  - With `load` and `save`.
- Modify: `model/environments.rs`:
  - `pub fn write_env_var(root, env, name, value) -> io::Result<()>` edits `.env` (Default) or `.env.{env}`. It keeps other lines and comments and never touches `*.local`.
  - `pub fn create_environment(root, name) -> io::Result<()>` writes an empty `.env.{name}`. It rejects names that are empty or contain `/` or `.`.
  - `pub fn import_dotenv(root, env, content) -> io::Result<usize>`: secret-classified keys go to the Keychain instead of the file, and the function returns the count.
  - `pub fn usage(root, name) -> Vec<(String /*top folder*/, usize)>` scans request files for `${name}`. It gives the board's `Used by 18 requests · auth · 2 · charges · 5 …`.

**Board details:**
- The environment tabs (`Local | Staging | Production`), `New environment`, `Import .env` and `Add variable`.
- The info callout copy verbatim.
- The table: `Name | Value | Source | Scope`.
  - Source is `Collection default` (the `.env` file), `My keychain`, `Captured at run` (from the last run's stores, with the `from create-charge → body.id` subtitle when the store config shows it) or `Not set` (with a `Set value` button).
  - Scope is the top folders from `usage`, or `All requests` when every folder uses it.
- The edit drawer:
  - the name;
  - a `Secret` badge;
  - a masked value with reveal;
  - `Stored in macOS Keychain under “curl-runner studio · Payments API · Staging”.`;
  - the description;
  - the `Secret` toggle (moving a value between the file and the Keychain);
  - the `Required for shared users` toggle;
  - `Used by N requests`;
  - `Delete variable`, `Cancel` and `Save`.
- The legend: `Resolution order: captured → my keychain → environment → collection default`.
- The status bar: `Staging · 6 of 7 variables set · 1 missing · Keychain unlocked`.
- `Shared by Acme` rows appear only in plan 3.

- [ ] **Step 1: Write failing tests:**
  - `write_env_var_preserves_other_lines`: comments and order are kept, and an existing key is replaced in place.
  - `never_writes_local_files`.
  - `create_environment_validates_name`.
  - `import_dotenv_routes_secrets_to_keychain`: `SECRET_X` lands in `MemorySecrets` and does not appear in the file.
  - `toggle_secret_moves_value`: from the file to the Keychain, the file line is removed; the reverse writes it back.
  - `usage_counts_by_folder`.
  - A smoke test, `environments_screen_lists_sources`.
- [ ] **Step 2–4:** FAIL → implement → PASS. Screenshot.
- [ ] **Step 5:** Commit: `feat(studio): environments and keychain-backed variables`.

### Task 11: Settings (board: Settings)

**Files:**
- Create: `ui/settings_view.rs`
- Create: `model/settings.rs`: serde `Settings { general, editor: { font_size: 13, tab_size: 2, word_wrap, format_on_save }, … }` stored in Application Support, with defaults.
- Modify: `ui/app.rs`, for `⌘,`.

**Interfaces:** `Settings::load(base) -> Settings` and `Settings::save(&self, base)`. The editor settings apply live to open editors.

**Board details:**
- The left nav: `General`, `Editor`, `Appearance`, `Accounts & sync`, `Keyboard shortcuts` and `Updates`, with the footer `curl-runner studio 0.1.0 · CLI {core VERSION} · Changelog`.
- The pages:
  - **Appearance:** Dark is selected. Light and System show `Coming soon` (spec non-goal).
  - **Keyboard shortcuts:** a read-only table generated from the registered keymap. Write a pure `pub fn shortcut_rows() -> Vec<(String, String)>` from the same binding list `app.rs` registers, so they can't drift.
  - **Updates:** shows the versions and `Updates arrive with new builds` (spec non-goal: auto-update).
  - **Accounts & sync:** only the `Credentials` table is real in this plan. It lists every Keychain item across environments, with `Edit` opening Task 10's drawer. The Server, Account and Sync cards render disabled with `Available in a later build`; plan 3 replaces them.

- [ ] **Step 1: Write failing tests:**
  - `settings_roundtrip_and_defaults`.
  - `shortcut_rows_cover_every_binding`: the count equals the number of bindings registered in `app.rs`'s list.
  - A smoke test, `editor_font_size_applies`.
- [ ] **Step 2–4:** FAIL → implement → PASS. Screenshot.
- [ ] **Step 5:** Commit: `feat(studio): settings`.

### Task 12: Import screen and first launch (boards: Import, Welcome)

**Files:**
- Create: `ui/import_dialog.rs`, matching the Import board.
  - Tabs: `curl | Postman collection | OpenAPI | Folder of YAML`.
  - The `curl` tab reuses `request_from_curl` and the YAML preview. It accepts several commands: split on blank lines or on lines starting with `curl `, and create one file per command.
  - The Postman and OpenAPI tabs show `Coming soon`.
  - `Folder of YAML` copies the `*.yaml`/`*.yml` files from a picked folder into a target folder in the collection.
- Modify:
  - `ui/new_dialog.rs`: `import()` now opens `ImportDialog`;
  - `ui/welcome.rs`: the Recent list and keyboard hints per the Welcome board;
  - `ui/state.rs`: `recent: Vec<RecentEntry { path, opened_at }>`, capped at 10, with `Clear`.
- The Welcome cards: `New collection ⌘N`, `Open folder ⌘O` and `Import ⌘I`.
  - `Open a shared link ⌘L` and the "Connected to curl-runner Cloud" chip render disabled until plan 3.
  - Recent shows the folder name, the path (with `~` for home), a `Local` chip and a relative time (`2 days ago`, `Sep 12`).
  - Missing folders are greyed with `Folder not found`.

**Interfaces:**
- `pub fn split_curl_commands(text: &str) -> Vec<String>` in `model/templates.rs`.
- `pub fn relative_time(at, now) -> String` in `model/history.rs`, shared with History.
- `StateStore::recent() -> Vec<RecentEntry>`, `remember_folder` (which now also updates recent) and `clear_recent()`.

- [ ] **Step 1: Write failing tests:**
  - `split_curl_commands_handles_continuations`: two commands with `\` continuations give 2 entries.
  - `recent_is_mru_capped_at_10`.
  - `relative_time_buckets`.
  - `copy_folder_of_yaml_skips_hidden_and_non_yaml`.
  - A smoke test, `import_two_curls_creates_two_files`.
- [ ] **Step 2–4:** FAIL → implement → PASS. Screenshots for both boards.
- [ ] **Step 5:** Commit: `feat(studio): import dialog and first-launch recents`.

---

## After plan 2

Plan 3 covers the mocked server boards:
- SignIn;
- Share;
- OpenShared;
- ReadOnly;
- SyncConflict;
- the Accounts & sync and Sharing pages;
- the title-bar sync/share chips;
- the Welcome shared-link card.

It runs behind the backend trait from the spec.
